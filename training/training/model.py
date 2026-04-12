"""Value network and state featurization for StS combat."""

import json

import numpy as np
import torch
import torch.nn as nn


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
    """Extract a fixed-size feature vector from a PyGameState."""
    summary = json.loads(state.summary())
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
            is_alive = m.get("state", "Alive") == "Alive"
            features[base] = m["hp"] / m_max_hp if is_alive else 0.0
            features[base + 1] = m.get("block", 0) / 10.0
            features[base + 2] = 1.0 if is_alive else 0.0
            features[base + 3] = (m.get("damage") or 0) / 10.0

    return features


def batch_featurize(states) -> torch.Tensor:
    """Featurize a list of PyGameState into a batched tensor."""
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
