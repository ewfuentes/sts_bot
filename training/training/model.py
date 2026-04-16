"""Value network and state featurization for StS combat."""

import math

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
        MONSTER_SCALAR_FIELDS = ["monster_hp", "monster_max_hp", "monster_block", "monster_damage", "monster_hits"]
        self.monster_scalar_fields = MONSTER_SCALAR_FIELDS
        MAX_MONSTERS = 4
        self.max_monsters = MAX_MONSTERS
        TOKEN_TYPES = (PLAYER_FIELDS
                       + ["deck", "hand", "draw", "discard", "exhaust"]
                       + COMBAT_SCALAR_FIELDS
                       + MONSTER_SCALAR_FIELDS
                       + ["monster_id", "monster_intent"]
                       + [f"monster_{i}" for i in range(MAX_MONSTERS)]
                       + ["player_power", "monster_power"])
        self.idx_from_token_type = {x: i for i, x in enumerate(TOKEN_TYPES)}
        self.token_type_embedding = nn.Embedding(len(TOKEN_TYPES), self.model_dim)

        self.idx_from_card_id = {x: i for i, x in enumerate(sts.all_card_ids())}
        self.card_id_embedding = nn.Embedding(len(self.idx_from_card_id), self.model_dim)

        self.idx_from_monster_id = {x: i for i, x in enumerate(sts.all_monster_ids())}
        self.monster_id_embedding = nn.Embedding(len(self.idx_from_monster_id), self.model_dim)

        self.idx_from_power_id = {x: i for i, x in enumerate(sts.all_power_ids())}
        self.power_id_embedding = nn.Embedding(len(self.idx_from_power_id), self.model_dim)

        # 4 intent types: UNKNOWN=0, ATTACK=1, ATTACK_BUFF=2, BUFF=3
        self.intent_embedding = nn.Embedding(4, self.model_dim)

    @property
    def device(self) -> torch.device:
        return self.token_type_embedding.weight.device

    def _featurize_player_scalars(self, td: sts.TokenData, batch: int) -> tuple[torch.Tensor, torch.Tensor]:
        """Player scalar fields -> (batch, 4, model_dim) tokens, all unmasked."""
        num_freqs = self.model_dim // 2
        dev = self.device
        player_vals = torch.tensor(td.player_scalars)
        player_encoded = fourier_encode(player_vals, num_freqs).to(dev)
        player_type_indices = torch.tensor([self.idx_from_token_type[f] for f in PLAYER_FIELDS])
        tokens = player_encoded + _embed(self.token_type_embedding, player_type_indices).unsqueeze(0)
        mask = torch.zeros(batch, len(PLAYER_FIELDS), dtype=torch.bool, device=dev)
        return tokens, mask

    def _featurize_deck(self, td: sts.TokenData, batch: int) -> tuple[torch.Tensor, torch.Tensor] | None:
        """Deck cards -> (batch, max_deck, model_dim) tokens, or None if empty."""
        dev = self.device
        deck_indices = torch.tensor(td.deck_card_indices, dtype=torch.long)
        if deck_indices.shape[1] == 0:
            return None
        tokens = (_embed(self.card_id_embedding, deck_indices)
                  + _embed(self.token_type_embedding,
                           torch.tensor(self.idx_from_token_type["deck"])).unsqueeze(0).unsqueeze(0))
        mask = torch.ones(batch, deck_indices.shape[1], dtype=torch.bool, device=dev)
        for i, length in enumerate(td.deck_lengths):
            mask[i, :length] = False
        return tokens, mask

    def _featurize_combat_scalars(self, td: sts.TokenData, batch: int) -> tuple[torch.Tensor, torch.Tensor]:
        """Combat scalars (block, energy, die_roll, turn) -> (batch, 4, model_dim) tokens.
        Masked for non-combat states."""
        num_freqs = self.model_dim // 2
        dev = self.device
        combat_vals = torch.tensor(td.combat_scalars)
        combat_encoded = fourier_encode(combat_vals, num_freqs).to(dev)
        combat_type_indices = torch.tensor([self.idx_from_token_type[f] for f in self.combat_scalar_fields])
        tokens = combat_encoded + _embed(self.token_type_embedding, combat_type_indices).unsqueeze(0)
        mask = torch.tensor([[not ic] * 4 for ic in td.in_combat], dtype=torch.bool, device=dev)
        return tokens, mask

    def _featurize_card_piles(self, td: sts.TokenData, batch: int) -> tuple[torch.Tensor, torch.Tensor] | None:
        """Card piles (hand/draw/discard/exhaust) -> tokens, or None if empty."""
        dev = self.device
        pile_card_idx = torch.tensor(td.pile_card_indices, dtype=torch.long)
        if pile_card_idx.shape[1] == 0:
            return None
        pile_type_map = torch.tensor([
            self.idx_from_token_type["hand"],
            self.idx_from_token_type["draw"],
            self.idx_from_token_type["discard"],
            self.idx_from_token_type["exhaust"],
        ], dtype=torch.long)
        pile_type_idx = pile_type_map[torch.tensor(td.pile_type_indices, dtype=torch.long)]
        tokens = (_embed(self.card_id_embedding, pile_card_idx)
                  + _embed(self.token_type_embedding, pile_type_idx))
        mask = torch.ones(batch, pile_card_idx.shape[1], dtype=torch.bool, device=dev)
        for i, length in enumerate(td.pile_lengths):
            mask[i, :length] = False
        return tokens, mask

    def _featurize_monsters(self, td: sts.TokenData, batch: int) -> tuple[torch.Tensor, torch.Tensor] | None:
        """Alive monsters -> 1 ID token + scalar tokens per monster, or None if empty."""
        num_freqs = self.model_dim // 2
        dev = self.device
        max_m = max(td.monster_counts) if td.monster_counts else 0
        if max_m == 0:
            return None

        num_scalars = len(self.monster_scalar_fields)
        tokens_per_m = 1 + num_scalars
        max_monster_tokens = int(max_m) * tokens_per_m

        monster_pos_type_indices = [self.idx_from_token_type[f"monster_{p}"] for p in range(self.max_monsters)]
        monster_id_type_idx = self.idx_from_token_type["monster_id"]
        scalar_type_indices = [self.idx_from_token_type[f] for f in self.monster_scalar_fields]

        m_id_idx = torch.tensor(td.monster_id_indices, dtype=torch.long)
        m_pos_idx = torch.tensor(td.monster_position_indices, dtype=torch.long)
        m_scalars = torch.tensor(td.monster_scalars)

        id_embeds = _embed(self.monster_id_embedding, m_id_idx)
        id_type_embed = _embed(self.token_type_embedding, torch.tensor(monster_id_type_idx))
        pos_type_lut = torch.tensor(monster_pos_type_indices, dtype=torch.long)
        pos_embeds = _embed(self.token_type_embedding, pos_type_lut[m_pos_idx])

        scalar_type_embeds = _embed(self.token_type_embedding,
                                    torch.tensor(scalar_type_indices, dtype=torch.long))
        scalar_encoded = fourier_encode(m_scalars, num_freqs).to(dev)

        m_tokens = torch.zeros(batch, max_monster_tokens, self.model_dim, device=dev)
        m_mask = torch.ones(batch, max_monster_tokens, dtype=torch.bool, device=dev)

        for j in range(int(max_m)):
            offset = j * tokens_per_m
            m_tokens[:, offset] = id_embeds[:, j] + id_type_embed + pos_embeds[:, j]
            for k in range(num_scalars):
                m_tokens[:, offset + 1 + k] = (scalar_encoded[:, j, k]
                                                + scalar_type_embeds[k]
                                                + pos_embeds[:, j])

        for i in range(batch):
            n_alive = int(td.monster_counts[i])
            m_mask[i, :n_alive * tokens_per_m] = False

        return m_tokens, m_mask

    def _featurize_monster_intents(self, td: sts.TokenData, batch: int) -> tuple[torch.Tensor, torch.Tensor] | None:
        """Monster intents -> 1 token per alive monster, or None if empty."""
        dev = self.device
        max_m = max(td.monster_counts) if td.monster_counts else 0
        if max_m == 0:
            return None

        intent_idx = torch.tensor(td.monster_intent_indices, dtype=torch.long)  # (batch, max_m)
        m_pos_idx = torch.tensor(td.monster_position_indices, dtype=torch.long)
        pos_type_lut = torch.tensor(
            [self.idx_from_token_type[f"monster_{p}"] for p in range(self.max_monsters)],
            dtype=torch.long)
        intent_type_embed = _embed(self.token_type_embedding,
                                   torch.tensor(self.idx_from_token_type["monster_intent"]))

        tokens = (_embed(self.intent_embedding, intent_idx)
                  + intent_type_embed
                  + _embed(self.token_type_embedding, pos_type_lut[m_pos_idx]))
        mask = torch.ones(batch, int(max_m), dtype=torch.bool, device=dev)
        for i in range(batch):
            mask[i, :int(td.monster_counts[i])] = False
        return tokens, mask

    def _featurize_player_powers(self, td: sts.TokenData, batch: int) -> tuple[torch.Tensor, torch.Tensor] | None:
        """Player powers -> 1 token per active power. Only present powers are included."""
        num_freqs = self.model_dim // 2
        dev = self.device
        max_pp = max(td.player_power_counts) if td.player_power_counts else 0
        if max_pp == 0:
            return None

        power_idx = torch.tensor(td.player_power_indices, dtype=torch.long)  # (batch, max_pp)
        amounts = torch.tensor(td.player_power_amounts)  # (batch, max_pp)
        encoded = fourier_encode(amounts, num_freqs).to(dev)  # (batch, max_pp, dim)

        tokens = (encoded
                  + _embed(self.power_id_embedding, power_idx)
                  + _embed(self.token_type_embedding,
                           torch.tensor(self.idx_from_token_type["player_power"])))
        mask = torch.ones(batch, int(max_pp), dtype=torch.bool, device=dev)
        for i in range(batch):
            mask[i, :int(td.player_power_counts[i])] = False
        return tokens, mask

    def _featurize_monster_powers(self, td: sts.TokenData, batch: int) -> tuple[torch.Tensor, torch.Tensor] | None:
        """Monster powers -> 1 token per active power per monster. Sparse representation."""
        num_freqs = self.model_dim // 2
        dev = self.device
        max_mp = max(td.monster_power_counts) if td.monster_power_counts else 0
        if max_mp == 0:
            return None

        power_idx = torch.tensor(td.monster_power_indices, dtype=torch.long)  # (batch, max_mp)
        amounts = torch.tensor(td.monster_power_amounts)  # (batch, max_mp)
        pos_idx = torch.tensor(td.monster_power_positions, dtype=torch.long)  # (batch, max_mp)
        encoded = fourier_encode(amounts, num_freqs).to(dev)  # (batch, max_mp, dim)

        pos_type_lut = torch.tensor(
            [self.idx_from_token_type[f"monster_{p}"] for p in range(self.max_monsters)],
            dtype=torch.long)

        tokens = (encoded
                  + _embed(self.power_id_embedding, power_idx)
                  + _embed(self.token_type_embedding, pos_type_lut[pos_idx])
                  + _embed(self.token_type_embedding,
                           torch.tensor(self.idx_from_token_type["monster_power"])))
        mask = torch.ones(batch, int(max_mp), dtype=torch.bool, device=dev)
        for i in range(batch):
            mask[i, :int(td.monster_power_counts[i])] = False
        return tokens, mask

    def featurize_game_state(self, states: list[sts.GameState]) -> tuple[torch.Tensor, torch.Tensor]:
        """Extract token features from a batch of game states.

        Returns (tokens, mask) where tokens is (batch, seq_len, model_dim)
        and mask is (batch, seq_len) with True for padding positions.
        """
        td = sts.extract_token_data(states, list(self.idx_from_card_id.keys()),
                                    list(self.idx_from_monster_id.keys()),
                                    list(self.idx_from_power_id.keys()))
        batch = len(states)
        all_tokens = []
        all_masks = []

        # Player scalars
        tokens, mask = self._featurize_player_scalars(td, batch)
        all_tokens.append(tokens)
        all_masks.append(mask)

        # Deck cards
        result = self._featurize_deck(td, batch)
        if result is not None:
            all_tokens.append(result[0])
            all_masks.append(result[1])

        # Combat scalars
        tokens, mask = self._featurize_combat_scalars(td, batch)
        all_tokens.append(tokens)
        all_masks.append(mask)

        # Card piles
        result = self._featurize_card_piles(td, batch)
        if result is not None:
            all_tokens.append(result[0])
            all_masks.append(result[1])

        # Monsters (ID + scalars)
        result = self._featurize_monsters(td, batch)
        if result is not None:
            all_tokens.append(result[0])
            all_masks.append(result[1])

        # Monster intents
        result = self._featurize_monster_intents(td, batch)
        if result is not None:
            all_tokens.append(result[0])
            all_masks.append(result[1])

        # Player powers
        result = self._featurize_player_powers(td, batch)
        if result is not None:
            all_tokens.append(result[0])
            all_masks.append(result[1])

        # Monster powers
        result = self._featurize_monster_powers(td, batch)
        if result is not None:
            all_tokens.append(result[0])
            all_masks.append(result[1])

        tokens = torch.cat(all_tokens, dim=1)
        mask = torch.cat(all_masks, dim=1)
        return tokens, mask

    def forward(self, states: list[sts.GameState]) -> tuple[torch.Tensor, torch.Tensor]:
        """Returns (mean, log_variance), each of shape (batch,)."""
        features, mask = self.featurize_game_state(states)

        encoded = self.encoder(features, src_key_padding_mask=mask, is_causal=False)

        # Mean pool over non-masked tokens
        valid = (~mask).unsqueeze(-1).float()  # (batch, seq_len, 1)
        pooled = (encoded * valid).sum(dim=1) / valid.sum(dim=1).clamp(min=1)

        out = self.value_head(pooled)  # (batch, 2)
        return out[:, 0], out[:, 1]
