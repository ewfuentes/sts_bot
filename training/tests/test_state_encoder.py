"""Tests for StateEncoder."""

import json
import torch
from sts_simulator import GameState
from training.model import StateEncoder, StateEncoderConfig


def make_combat_state(**overrides) -> GameState:
    """Create a combat GameState. Override any top-level or screen fields."""
    state = {
        "hp": 6, "max_hp": 8, "gold": 15, "floor": 3, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": False},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": False},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": False},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": False},
            {"id": "BGBash", "name": "Bash", "cost": 2, "type": "ATTACK", "upgraded": False},
        ],
        "relics": [
            {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1},
        ],
        "potions": [None, None],
        "screen": {
            "type": "combat",
            "encounter": "BoardGame:Jaw Worm (Easy)",
        },
    }
    state.update(overrides)
    return GameState.from_json(json.dumps(state))


def make_map_state(**overrides) -> GameState:
    """Create a map GameState."""
    state = {
        "hp": 7, "max_hp": 8, "gold": 30, "floor": 2, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": False},
        ],
        "relics": [],
        "potions": [None, None],
        "screen": {
            "type": "map",
            "current_node": 0,
            "available_nodes": [
                {"label": "Monster (3,0)", "kind": "monster", "node_index": 1},
            ],
        },
    }
    state.update(overrides)
    return GameState.from_json(json.dumps(state))


def test_forward_combat():
    config = StateEncoderConfig(model_dim=32, num_heads=4, num_layers=2)
    model = StateEncoder(config)
    states = [make_combat_state(), make_combat_state(hp=3)]
    mean, log_var = model(states)
    assert mean.shape == (2,)
    assert log_var.shape == (2,)
    print(f"Combat: mean={mean.tolist()}, log_var={log_var.tolist()}")


def test_forward_map():
    config = StateEncoderConfig(model_dim=32, num_heads=4, num_layers=2)
    model = StateEncoder(config)
    states = [make_map_state(), make_map_state(gold=50)]
    mean, log_var = model(states)
    assert mean.shape == (2,)
    assert log_var.shape == (2,)
    print(f"Map: mean={mean.tolist()}, log_var={log_var.tolist()}")


def test_forward_mixed():
    config = StateEncoderConfig(model_dim=32, num_heads=4, num_layers=2)
    model = StateEncoder(config)
    states = [make_combat_state(), make_map_state()]
    mean, log_var = model(states)
    assert mean.shape == (2,)
    assert log_var.shape == (2,)
    print(f"Mixed: mean={mean.tolist()}, log_var={log_var.tolist()}")
