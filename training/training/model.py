"""Value network and state featurization for StS combat."""

import json
import math

import numpy as np
import torch
import torch.nn as nn
import msgspec

import sts_simulator as sts

def fourier_encode(x: torch.Tensor, num_freqs: int, max_freq: float = 1000.0) -> torch.Tensor:
    """Encode scalar values using sinusoidal features.

    Args:
        x: Tensor of any shape (...,)
        num_freqs: Number of frequency bands. Output dim is 2 * num_freqs.
        max_freq: Maximum frequency. Frequencies are log-spaced from 1 to max_freq.

    Returns:
        Tensor of shape (..., 2 * num_freqs) with [sin, cos] pairs.
    """
    freqs = torch.logspace(0, math.log10(max_freq), num_freqs, device=x.device)
    angles = x.unsqueeze(-1) * freqs * (2 * math.pi)
    return torch.cat([angles.sin(), angles.cos()], dim=-1)


class StateEncoderConfig(msgspec.Struct):
    model_dim: int
    num_heads: int
    num_layers: int

def _device(embedding: nn.Embedding) -> torch.device:
    return embedding.weight.device

def _embed(embedding: nn.Embedding, indices: torch.Tensor) -> torch.Tensor:
    """Look up embeddings, moving indices to the embedding's device."""
    return embedding(indices.to(_device(embedding)))


PLAYER_FIELDS = ["hp", "max_hp", "gold", "floor"]


def featurize_player(
        summaries: list[dict],
        model_dim: int,
        idx_from_token_type: dict[str, int],
        token_type_embedding: nn.Embedding,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Create one token per player scalar field.

    Returns:
        tokens: (batch, num_fields, model_dim)
        mask: (batch, num_fields) — all False (no padding)
    """
    batch = len(summaries)
    num_fields = len(PLAYER_FIELDS)
    num_freqs = model_dim // 2

    # Extract values: (batch, num_fields)
    values = torch.zeros(batch, num_fields)
    for i, s in enumerate(summaries):
        for j, field in enumerate(PLAYER_FIELDS):
            values[i, j] = float(s.get(field, 0))

    # Fourier encode: (batch, num_fields, model_dim)
    dev = _device(token_type_embedding)
    encoded = fourier_encode(values, num_freqs).to(dev)

    # Add token type embeddings: (num_fields,) -> (1, num_fields, model_dim)
    type_indices = torch.tensor([idx_from_token_type[f] for f in PLAYER_FIELDS])
    type_embeds = _embed(token_type_embedding, type_indices).unsqueeze(0)

    tokens = encoded + type_embeds
    mask = torch.zeros(batch, num_fields, dtype=torch.bool, device=dev)
    return tokens, mask


def featurize_deck(
        summaries: list[dict],
        model_dim: int,
        token_type_idx: int,
        token_type_embedding: nn.Embedding,
        idx_from_card_id: dict[str, int],
        card_id_embedding: nn.Embedding,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Create one token per card in the deck.

    Returns:
        tokens: (batch, max_deck_size, model_dim)
        mask: (batch, max_deck_size) — True for padding positions
    """
    batch = len(summaries)
    decks = [s["deck"] for s in summaries]
    max_cards = max((len(d) for d in decks), default=0)

    dev = _device(card_id_embedding)

    if max_cards == 0:
        return (torch.zeros(batch, 0, model_dim, device=dev),
                torch.ones(batch, 0, dtype=torch.bool, device=dev))

    card_indices = torch.zeros(batch, max_cards, dtype=torch.long)
    mask = torch.ones(batch, max_cards, dtype=torch.bool, device=dev)
    for i, deck in enumerate(decks):
        for j, card in enumerate(deck):
            card_indices[i, j] = idx_from_card_id[card["id"]]
            mask[i, j] = False

    tokens = _embed(card_id_embedding, card_indices)
    type_embed = _embed(token_type_embedding, torch.tensor(token_type_idx)).unsqueeze(0).unsqueeze(0)
    tokens = tokens + type_embed

    return tokens, mask


COMBAT_SCALAR_MAP = {
    "block": "player_block",
    "energy": "player_energy",
    "die_roll": "die_roll",
    "turn": "turn",
}


def featurize_combat_scalars(
        summaries: list[dict],
        model_dim: int,
        combat_scalar_fields: list[str],
        idx_from_token_type: dict[str, int],
        token_type_embedding: nn.Embedding,
) -> tuple[torch.Tensor, torch.Tensor]:
    """One token per combat scalar (block, energy, die_roll, turn).

    Non-combat states get all-masked tokens.

    Returns:
        tokens: (batch, num_fields, model_dim)
        mask: (batch, num_fields)
    """
    batch = len(summaries)
    num_fields = len(combat_scalar_fields)
    num_freqs = model_dim // 2

    dev = _device(token_type_embedding)
    values = torch.zeros(batch, num_fields)
    mask = torch.ones(batch, num_fields, dtype=torch.bool, device=dev)

    for i, s in enumerate(summaries):
        screen = s["screen"]
        if screen.get("type") != "combat":
            continue
        for j, field in enumerate(combat_scalar_fields):
            val = screen[COMBAT_SCALAR_MAP[field]]
            values[i, j] = float(val if val is not None else 0)
            mask[i, j] = False

    encoded = fourier_encode(values, num_freqs).to(dev)
    type_indices = torch.tensor([idx_from_token_type[f] for f in combat_scalar_fields])
    type_embeds = _embed(token_type_embedding, type_indices).unsqueeze(0)

    tokens = encoded + type_embeds
    return tokens, mask


CARD_PILE_KEYS = ["hand", "draw_pile", "discard_pile", "exhaust_pile"]
CARD_PILE_TOKEN_TYPES = ["hand", "draw", "discard", "exhaust"]


def featurize_card_piles(
        summaries: list[dict],
        model_dim: int,
        idx_from_token_type: dict[str, int],
        token_type_embedding: nn.Embedding,
        idx_from_card_id: dict[str, int],
        card_id_embedding: nn.Embedding,
) -> tuple[torch.Tensor, torch.Tensor]:
    """One token per card across hand, draw, discard, and exhaust piles.

    Returns:
        tokens: (batch, max_total_cards, model_dim)
        mask: (batch, max_total_cards)
    """
    batch = len(summaries)

    all_cards: list[list[tuple[int, int]]] = [[] for _ in range(batch)]
    for i, s in enumerate(summaries):
        screen = s["screen"]
        if screen.get("type") != "combat":
            continue
        for pile_key, type_name in zip(CARD_PILE_KEYS, CARD_PILE_TOKEN_TYPES):
            type_idx = idx_from_token_type[type_name]
            for card in screen[pile_key]:
                card_idx = idx_from_card_id[card["id"]]
                all_cards[i].append((card_idx, type_idx))

    dev = _device(card_id_embedding)
    max_cards = max((len(cl) for cl in all_cards), default=0)
    if max_cards == 0:
        return (torch.zeros(batch, 0, model_dim, device=dev),
                torch.ones(batch, 0, dtype=torch.bool, device=dev))

    card_indices = torch.zeros(batch, max_cards, dtype=torch.long)
    type_indices = torch.zeros(batch, max_cards, dtype=torch.long)
    mask = torch.ones(batch, max_cards, dtype=torch.bool, device=dev)
    for i, cards in enumerate(all_cards):
        for j, (c_idx, t_idx) in enumerate(cards):
            card_indices[i, j] = c_idx
            type_indices[i, j] = t_idx
            mask[i, j] = False

    tokens = _embed(card_id_embedding, card_indices) + _embed(token_type_embedding, type_indices)
    return tokens, mask


MONSTER_SCALAR_MAP = {
    "monster_hp": "hp",
    "monster_max_hp": "max_hp",
    "monster_block": "block",
    "monster_damage": "damage",
}


def featurize_monsters(
        summaries: list[dict],
        model_dim: int,
        monster_scalar_fields: list[str],
        max_monsters: int,
        idx_from_token_type: dict[str, int],
        token_type_embedding: nn.Embedding,
        idx_from_monster_id: dict[str, int],
        monster_id_embedding: nn.Embedding,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Tokens per alive monster: 1 ID token + 1 per scalar field.

    Each monster's tokens share a monster_index embedding (monster_0, monster_1, ...)
    to tie them together.

    Returns:
        tokens: (batch, max_monster_tokens, model_dim)
        mask: (batch, max_monster_tokens)
    """
    batch = len(summaries)
    num_freqs = model_dim // 2
    num_scalars = len(monster_scalar_fields)
    tokens_per_monster = 1 + num_scalars  # ID token + scalar tokens
    dev = _device(monster_id_embedding)

    # First pass: collect all data as Python lists
    # Per state: list of (monster_id_idx, index_type_idx, scalar_values)
    per_state: list[list[tuple[int, int, list[float]]]] = []
    for s in summaries:
        monsters = []
        screen = s["screen"]
        if screen.get("type") == "combat":
            for m_idx, m in enumerate(screen["monsters"][:max_monsters]):
                if m["state"] != "alive":
                    continue
                mid = idx_from_monster_id[m["id"]]
                index_type = idx_from_token_type[f"monster_{m_idx}"]
                vals = []
                for field in monster_scalar_fields:
                    v = m[MONSTER_SCALAR_MAP[field]]
                    vals.append(float(v if v is not None else 0))
                monsters.append((mid, index_type, vals))
        per_state.append(monsters)

    # Compute max tokens across batch
    counts = [len(ms) * tokens_per_monster for ms in per_state]
    max_len = max(counts, default=0)

    if max_len == 0:
        return (torch.zeros(batch, 0, model_dim, device=dev),
                torch.ones(batch, 0, dtype=torch.bool, device=dev))

    # Build flat index tensors: monster_ids, index_types, scalar_types, scalar_values
    # and a position map (batch_idx, token_offset) for scattering into output
    monster_id_list: list[int] = []
    index_type_list: list[int] = []
    id_type_idx = idx_from_token_type["monster_id"]
    scalar_type_indices = [idx_from_token_type[f] for f in monster_scalar_fields]

    # For scalar tokens: (flat_idx, type_idx, value, index_type)
    scalar_values_list: list[float] = []
    scalar_type_list: list[int] = []
    scalar_index_list: list[int] = []

    # Map from flat monster index -> (batch_idx, token_start)
    batch_indices: list[int] = []
    token_offsets: list[int] = []

    for i, ms in enumerate(per_state):
        offset = 0
        for mid, index_type, vals in ms:
            monster_id_list.append(mid)
            index_type_list.append(index_type)
            batch_indices.append(i)
            token_offsets.append(offset)
            offset += 1
            for s_idx, (s_type, val) in enumerate(zip(scalar_type_indices, vals)):
                scalar_values_list.append(val)
                scalar_type_list.append(s_type)
                scalar_index_list.append(index_type)
                offset += 1

    # Batch embedding lookups (one call each)
    id_embeds = _embed(monster_id_embedding, torch.tensor(monster_id_list, dtype=torch.long))
    id_type_embeds = _embed(token_type_embedding, torch.tensor([id_type_idx] * len(monster_id_list), dtype=torch.long))
    index_embeds_for_id = _embed(token_type_embedding, torch.tensor(index_type_list, dtype=torch.long))

    # Scalar embeddings
    if scalar_values_list:
        scalar_encoded = fourier_encode(torch.tensor(scalar_values_list), num_freqs).to(dev)
        scalar_type_embeds = _embed(token_type_embedding, torch.tensor(scalar_type_list, dtype=torch.long))
        scalar_index_embeds = _embed(token_type_embedding, torch.tensor(scalar_index_list, dtype=torch.long))

    # Scatter into output
    tokens = torch.zeros(batch, max_len, model_dim, device=dev)
    mask = torch.ones(batch, max_len, dtype=torch.bool, device=dev)

    # Place ID tokens
    for flat_idx in range(len(monster_id_list)):
        bi = batch_indices[flat_idx]
        to = token_offsets[flat_idx]
        tokens[bi, to] = id_embeds[flat_idx] + id_type_embeds[flat_idx] + index_embeds_for_id[flat_idx]
        mask[bi, to] = False

    # Place scalar tokens
    scalar_flat = 0
    for flat_idx in range(len(monster_id_list)):
        bi = batch_indices[flat_idx]
        to = token_offsets[flat_idx]
        for s_idx in range(num_scalars):
            tokens[bi, to + 1 + s_idx] = (scalar_encoded[scalar_flat]
                                           + scalar_type_embeds[scalar_flat]
                                           + scalar_index_embeds[scalar_flat])
            mask[bi, to + 1 + s_idx] = False
            scalar_flat += 1

    return tokens, mask


class StateEncoder(nn.Module):
    def __init__(self, config):
        super().__init__()
        encoder_layer = nn.TransformerEncoderLayer(
            d_model=config.model_dim,
            nhead=config.num_heads,
            dim_feedforward=config.model_dim * 4,
            batch_first=True,
            norm_first=True)
        self.encoder = nn.TransformerEncoder(encoder_layer, config.num_layers)
        self.model_dim = config.model_dim

        self.value_head = nn.Sequential(
            nn.Linear(config.model_dim, config.model_dim),
            nn.ReLU(),
            nn.Linear(config.model_dim, 2),  # [mean, log_variance]
        )

        COMBAT_SCALAR_FIELDS = ["block", "energy", "die_roll", "turn"]
        self.combat_scalar_fields = COMBAT_SCALAR_FIELDS
        MONSTER_SCALAR_FIELDS = ["monster_hp", "monster_max_hp", "monster_block", "monster_damage"]
        self.monster_scalar_fields = MONSTER_SCALAR_FIELDS
        MAX_MONSTERS = 4
        self.max_monsters = MAX_MONSTERS
        TOKEN_TYPES = (PLAYER_FIELDS
                       + ["deck", "hand", "draw", "discard", "exhaust"]
                       + COMBAT_SCALAR_FIELDS
                       + MONSTER_SCALAR_FIELDS
                       + ["monster_id"]
                       + [f"monster_{i}" for i in range(MAX_MONSTERS)])
        self.idx_from_token_type = {x: i for i, x in enumerate(TOKEN_TYPES)}
        self.token_type_embedding = nn.Embedding(len(TOKEN_TYPES), self.model_dim)

        self.idx_from_card_id = {x: i for i, x in enumerate(sts.all_card_ids())}
        self.card_id_embedding = nn.Embedding(len(self.idx_from_card_id), self.model_dim)

        self.idx_from_monster_id = {x: i for i, x in enumerate(sts.all_monster_ids())}
        self.monster_id_embedding = nn.Embedding(len(self.idx_from_monster_id), self.model_dim)


    def featurize_game_state(self, states: list[sts.GameState]) -> tuple[torch.Tensor, torch.Tensor]:
        td = sts.extract_token_data(states, list(self.idx_from_card_id.keys()),
                                    list(self.idx_from_monster_id.keys()))
        batch = len(states)
        num_freqs = self.model_dim // 2
        dev = self.device
        all_tokens = []
        all_masks = []

        # Player scalars: (batch, 4) -> (batch, 4, model_dim)
        player_vals = torch.tensor(td.player_scalars)  # (batch, 4)
        player_encoded = fourier_encode(player_vals, num_freqs).to(dev)  # (batch, 4, model_dim)
        player_type_indices = torch.tensor([self.idx_from_token_type[f] for f in PLAYER_FIELDS])
        player_tokens = player_encoded + _embed(self.token_type_embedding, player_type_indices).unsqueeze(0)
        all_tokens.append(player_tokens)
        all_masks.append(torch.zeros(batch, len(PLAYER_FIELDS), dtype=torch.bool, device=dev))

        # Deck cards: (batch, max_deck) -> (batch, max_deck, model_dim)
        deck_indices = torch.tensor(td.deck_card_indices, dtype=torch.long)  # (batch, max_deck)
        if deck_indices.shape[1] > 0:
            deck_tokens = (_embed(self.card_id_embedding, deck_indices)
                           + _embed(self.token_type_embedding,
                                    torch.tensor(self.idx_from_token_type["deck"])).unsqueeze(0).unsqueeze(0))
            deck_mask = torch.ones(batch, deck_indices.shape[1], dtype=torch.bool, device=dev)
            for i, length in enumerate(td.deck_lengths):
                deck_mask[i, :length] = False
            all_tokens.append(deck_tokens)
            all_masks.append(deck_mask)

        # Combat scalars: (batch, 4) -> (batch, 4, model_dim)
        combat_vals = torch.tensor(td.combat_scalars)  # (batch, 4)
        combat_encoded = fourier_encode(combat_vals, num_freqs).to(dev)
        combat_type_indices = torch.tensor([self.idx_from_token_type[f] for f in self.combat_scalar_fields])
        combat_tokens = combat_encoded + _embed(self.token_type_embedding, combat_type_indices).unsqueeze(0)
        combat_mask = torch.tensor([[not ic] * 4 for ic in td.in_combat], dtype=torch.bool, device=dev)
        all_tokens.append(combat_tokens)
        all_masks.append(combat_mask)

        # Card piles: (batch, max_pile) -> (batch, max_pile, model_dim)
        pile_card_idx = torch.tensor(td.pile_card_indices, dtype=torch.long)
        if pile_card_idx.shape[1] > 0:
            # Map pile type 0-3 to token type indices for hand/draw/discard/exhaust
            pile_type_map = torch.tensor([
                self.idx_from_token_type["hand"],
                self.idx_from_token_type["draw"],
                self.idx_from_token_type["discard"],
                self.idx_from_token_type["exhaust"],
            ], dtype=torch.long)
            pile_type_idx = pile_type_map[torch.tensor(td.pile_type_indices, dtype=torch.long)]
            pile_tokens = (_embed(self.card_id_embedding, pile_card_idx)
                           + _embed(self.token_type_embedding, pile_type_idx))
            pile_mask = torch.ones(batch, pile_card_idx.shape[1], dtype=torch.bool, device=dev)
            for i, length in enumerate(td.pile_lengths):
                pile_mask[i, :length] = False
            all_tokens.append(pile_tokens)
            all_masks.append(pile_mask)

        # Monsters: each alive monster -> 1 ID token + 4 scalar tokens
        max_m = max(td.monster_counts) if td.monster_counts else 0
        if max_m > 0:
            num_scalars = len(self.monster_scalar_fields)
            tokens_per_m = 1 + num_scalars
            max_monster_tokens = int(max_m) * tokens_per_m

            # Pre-compute token type indices for monster scalars and positions
            monster_pos_type_indices = [self.idx_from_token_type[f"monster_{p}"] for p in range(self.max_monsters)]
            monster_id_type_idx = self.idx_from_token_type["monster_id"]
            scalar_type_indices = [self.idx_from_token_type[f] for f in self.monster_scalar_fields]

            # Build padded (batch, max_m) tensors for batched lookups
            m_id_idx = torch.tensor(td.monster_id_indices, dtype=torch.long)  # (batch, max_m)
            m_pos_idx = torch.tensor(td.monster_position_indices, dtype=torch.long)  # (batch, max_m)
            m_scalars = torch.tensor(td.monster_scalars)  # (batch, max_m, 4)

            # Batch embedding lookups
            id_embeds = _embed(self.monster_id_embedding, m_id_idx)  # (batch, max_m, dim)
            id_type_embed = _embed(self.token_type_embedding,
                                   torch.tensor(monster_id_type_idx))  # (dim,)
            # Position type embeds: map each monster's position to its type index
            pos_type_lut = torch.tensor(monster_pos_type_indices, dtype=torch.long)
            pos_embeds = _embed(self.token_type_embedding,
                                pos_type_lut[m_pos_idx])  # (batch, max_m, dim)

            # Scalar type embeds: (num_scalars, dim) - same for all monsters
            scalar_type_embeds = _embed(self.token_type_embedding,
                                        torch.tensor(scalar_type_indices, dtype=torch.long))  # (4, dim)

            # Scalar fourier: (batch, max_m, 4, dim)
            scalar_encoded = fourier_encode(m_scalars, num_freqs).to(dev)  # (batch, max_m, 4, dim)

            # Assemble tokens: (batch, max_m * tokens_per_m, dim)
            m_tokens = torch.zeros(batch, max_monster_tokens, self.model_dim, device=dev)
            m_mask = torch.ones(batch, max_monster_tokens, dtype=torch.bool, device=dev)

            for j in range(int(max_m)):
                offset = j * tokens_per_m
                # ID token
                m_tokens[:, offset] = id_embeds[:, j] + id_type_embed + pos_embeds[:, j]
                # Scalar tokens
                for k in range(num_scalars):
                    m_tokens[:, offset + 1 + k] = (scalar_encoded[:, j, k]
                                                    + scalar_type_embeds[k]
                                                    + pos_embeds[:, j])

            # Build mask from counts
            for i in range(batch):
                n_alive = int(td.monster_counts[i])
                m_mask[i, :n_alive * tokens_per_m] = False

            all_tokens.append(m_tokens)
            all_masks.append(m_mask)

        tokens = torch.cat(all_tokens, dim=1)
        mask = torch.cat(all_masks, dim=1)
        return tokens, mask


    @property
    def device(self) -> torch.device:
        return self.token_type_embedding.weight.device

    def forward(self, states: list[sts.GameState]) -> tuple[torch.Tensor, torch.Tensor]:
        """Returns (mean, log_variance), each of shape (batch,)."""
        features, mask = self.featurize_game_state(states)

        encoded = self.encoder(features, src_key_padding_mask=mask, is_causal=False)

        # Mean pool over non-masked tokens
        # mask is True for padding, so invert for the valid tokens
        valid = (~mask).unsqueeze(-1).float()  # (batch, seq_len, 1)
        pooled = (encoded * valid).sum(dim=1) / valid.sum(dim=1).clamp(min=1)

        out = self.value_head(pooled)  # (batch, 2)
        return out[:, 0], out[:, 1]

# Feature layout:
#   0: hp / max_hp
#   1: floor / 13 (act progress)
#   2: gold / 100 (scaled)
#   3: deck_size / 20
#   4: num_relics / 10
#   5: num_potions / 2
#   -- combat-specific (0 outside combat) --
#   6: player_block (scaled)
#   7: player_energy (scaled)
#   8: die_roll / 6 (0 if not rolled)
#   9: turn (scaled)
#   10: hand_size / 10
#   11: draw_pile_size / 10
#   12: discard_pile_size / 10
#   13-16: monster 0: hp_ratio, block_scaled, alive, damage_scaled
#   17-20: monster 1: same
#   21-24: monster 2: same
#   25-28: monster 3: same
#   29-32: monster 4: same
FEATURE_DIM = 33


def featurize(state) -> np.ndarray:
    """Extract a fixed-size feature vector from a GameState."""
    summary = json.loads(state.summary())
    return featurize_summary(summary)


def featurize_summary(summary: dict) -> np.ndarray:
    """Extract a fixed-size feature vector from a summary dict."""
    features = np.zeros(FEATURE_DIM, dtype=np.float32)

    max_hp = max(summary["max_hp"], 1)
    features[0] = summary["hp"] / max_hp
    features[1] = summary.get("floor", 0) / 13.0
    features[2] = summary.get("gold", 0) / 100.0
    features[3] = len(summary.get("deck", [])) / 20.0
    features[4] = len(summary.get("relics", [])) / 10.0
    features[5] = sum(1 for p in summary.get("potions", []) if p is not None) / 2.0

    screen = summary["screen"]
    if screen.get("type") == "combat":
        features[6] = screen.get("player_block", 0) / 10.0
        features[7] = screen.get("player_energy", 0) / 3.0
        die_roll = screen.get("die_roll")
        features[8] = (die_roll / 6.0) if die_roll else 0.0
        features[9] = screen.get("turn", 0) / 10.0
        features[10] = len(screen.get("hand", [])) / 10.0
        features[11] = len(screen.get("draw_pile", [])) / 10.0
        features[12] = len(screen.get("discard_pile", [])) / 10.0

        monsters = screen.get("monsters", [])
        for i, m in enumerate(monsters[:5]):
            base = 13 + i * 4
            m_max_hp = max(m.get("max_hp", 1), 1)
            is_alive = m.get("state", "alive") == "alive"
            features[base] = m["hp"] / m_max_hp if is_alive else 0.0
            features[base + 1] = m.get("block", 0) / 10.0
            features[base + 2] = 1.0 if is_alive else 0.0
            features[base + 3] = (m.get("damage") or 0) / 10.0

    return features


def batch_featurize(states) -> torch.Tensor:
    """Featurize a list of GameState into a batched tensor."""
    return torch.from_numpy(np.stack([featurize(s) for s in states]))


class ValueNet(nn.Module):
    """Simple MLP that predicts game outcome from state features."""

    def __init__(self, input_dim: int = FEATURE_DIM, hidden_dim: int = 128):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(input_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, 1),
            nn.Sigmoid(),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.net(x).squeeze(-1)
