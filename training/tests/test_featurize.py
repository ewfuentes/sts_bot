"""Tests for featurize_summary."""

import numpy as np
import pytest

from training.model import FEATURE_DIM, featurize_summary


COMBAT_STATE = {
    "hp": 6,
    "max_hp": 8,
    "gold": 15,
    "floor": 3,
    "act": 1,
    "deck": [
        {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
        {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
        {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
        {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
        {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
        {"id": "BGDefend_R", "cost": 1, "type": "SKILL", "upgraded": False},
        {"id": "BGDefend_R", "cost": 1, "type": "SKILL", "upgraded": False},
        {"id": "BGDefend_R", "cost": 1, "type": "SKILL", "upgraded": False},
        {"id": "BGDefend_R", "cost": 1, "type": "SKILL", "upgraded": False},
        {"id": "BGBash", "cost": 2, "type": "ATTACK", "upgraded": False},
        {"id": "BGCleave", "cost": 1, "type": "ATTACK", "upgraded": False},
    ],
    "relics": [
        {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1},
    ],
    "potions": [
        {"id": "BlockPotion", "name": "Block Potion"},
        None,
    ],
    "screen": {
        "type": "combat",
        "encounter": "BoardGame:Jaw Worm (Easy)",
        "player_block": 3,
        "player_energy": 2,
        "player_powers": [
            {"id": "BGStrength", "amount": 1},
        ],
        "die_roll": 4,
        "turn": 2,
        "hand": [
            {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGBash", "cost": 2, "type": "ATTACK", "upgraded": False},
            {"id": "BGDefend_R", "cost": 1, "type": "SKILL", "upgraded": False},
        ],
        "draw_pile": [
            {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGDefend_R", "cost": 1, "type": "SKILL", "upgraded": False},
            {"id": "BGDefend_R", "cost": 1, "type": "SKILL", "upgraded": False},
        ],
        "discard_pile": [
            {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
            {"id": "BGCleave", "cost": 1, "type": "ATTACK", "upgraded": False},
        ],
        "exhaust_pile": [],
        "monsters": [
            {
                "id": "BGJawWorm",
                "name": "BGJawWorm",
                "hp": 5,
                "max_hp": 8,
                "block": 2,
                "damage": 3,
                "hits": 1,
                "intent": "ATTACK",
                "powers": [{"id": "BGVulnerable", "amount": 1}],
                "state": "alive",
                "move_index": 0,
            },
        ],
    },
}

MAP_STATE = {
    "hp": 7,
    "max_hp": 8,
    "gold": 30,
    "floor": 2,
    "act": 1,
    "deck": [
        {"id": "BGStrike_R", "cost": 1, "type": "ATTACK", "upgraded": False},
        {"id": "BGDefend_R", "cost": 1, "type": "SKILL", "upgraded": False},
    ],
    "relics": [
        {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1},
        {"id": "BoardGame:Vajra", "name": "Vajra", "counter": -1},
    ],
    "potions": [None, None],
    "screen": {
        "type": "map",
        "current_node": 5,
        "available_nodes": [
            {"label": "Monster (3,2)", "kind": "monster", "node_index": 8},
            {"label": "Event (3,4)", "kind": "event", "node_index": 10},
        ],
    },
}

GAME_OVER_STATE = {
    "hp": 0,
    "max_hp": 8,
    "gold": 5,
    "floor": 1,
    "act": 1,
    "deck": [],
    "relics": [],
    "potions": [None, None],
    "screen": {"type": "game_over", "victory": False},
}

MULTI_MONSTER_STATE = {
    "hp": 8,
    "max_hp": 8,
    "gold": 5,
    "floor": 1,
    "act": 1,
    "deck": [],
    "relics": [],
    "potions": [None, None],
    "screen": {
        "type": "combat",
        "encounter": "BoardGame:Small Slimes",
        "player_block": 0,
        "player_energy": 3,
        "player_powers": [],
        "die_roll": 5,
        "turn": 1,
        "hand": [],
        "draw_pile": [],
        "discard_pile": [],
        "exhaust_pile": [],
        "monsters": [
            {
                "id": "BGSmallSlimeA",
                "name": "BGSmallSlimeA",
                "hp": 3,
                "max_hp": 3,
                "block": 0,
                "damage": 1,
                "hits": 1,
                "intent": "ATTACK",
                "powers": [],
                "state": "alive",
                "move_index": 0,
            },
            {
                "id": "BGSmallSlimeB",
                "name": "BGSmallSlimeB",
                "hp": 0,
                "max_hp": 4,
                "block": 0,
                "damage": None,
                "hits": 1,
                "intent": "DEBUFF",
                "powers": [],
                "state": "dead",
                "move_index": 1,
            },
            {
                "id": "BGSmallSlimeC",
                "name": "BGSmallSlimeC",
                "hp": 2,
                "max_hp": 3,
                "block": 1,
                "damage": 2,
                "hits": 1,
                "intent": "ATTACK",
                "powers": [],
                "state": "alive",
                "move_index": 0,
            },
        ],
    },
}


class TestFeatureDim:
    def test_output_shape(self):
        f = featurize_summary(COMBAT_STATE)
        assert f.shape == (FEATURE_DIM,)
        assert f.dtype == np.float32

    def test_map_state_shape(self):
        f = featurize_summary(MAP_STATE)
        assert f.shape == (FEATURE_DIM,)

    def test_game_over_shape(self):
        f = featurize_summary(GAME_OVER_STATE)
        assert f.shape == (FEATURE_DIM,)


class TestRunLevelFeatures:
    def test_hp_ratio(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[0] == pytest.approx(6 / 8)

    def test_floor(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[1] == pytest.approx(3 / 13)

    def test_gold(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[2] == pytest.approx(15 / 100)

    def test_deck_size(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[3] == pytest.approx(11 / 20)

    def test_relic_count(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[4] == pytest.approx(1 / 10)

    def test_potion_count(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[5] == pytest.approx(1 / 2)  # one potion, one empty slot

    def test_zero_hp(self):
        f = featurize_summary(GAME_OVER_STATE)
        assert f[0] == pytest.approx(0.0)

    def test_map_has_relics(self):
        f = featurize_summary(MAP_STATE)
        assert f[4] == pytest.approx(2 / 10)


class TestCombatFeatures:
    def test_block(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[6] == pytest.approx(3 / 10)

    def test_energy(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[7] == pytest.approx(2 / 3)

    def test_die_roll(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[8] == pytest.approx(4 / 6)

    def test_turn(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[9] == pytest.approx(2 / 10)

    def test_hand_size(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[10] == pytest.approx(4 / 10)

    def test_draw_pile(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[11] == pytest.approx(3 / 10)

    def test_discard_pile(self):
        f = featurize_summary(COMBAT_STATE)
        assert f[12] == pytest.approx(2 / 10)

    def test_non_combat_has_zero_combat_features(self):
        f = featurize_summary(MAP_STATE)
        assert np.all(f[6:] == 0.0)


class TestMonsterFeatures:
    def test_single_monster(self):
        f = featurize_summary(COMBAT_STATE)
        # Monster 0: hp=5/8, block=2, alive, damage=3
        assert f[13] == pytest.approx(5 / 8)
        assert f[14] == pytest.approx(2 / 10)
        assert f[15] == pytest.approx(1.0)
        assert f[16] == pytest.approx(3 / 10)

    def test_remaining_monster_slots_zero(self):
        f = featurize_summary(COMBAT_STATE)
        # Monsters 1-4 should be zero
        assert np.all(f[17:33] == 0.0)

    def test_dead_monster(self):
        f = featurize_summary(MULTI_MONSTER_STATE)
        # Monster 1 is dead
        assert f[17] == pytest.approx(0.0)   # hp_ratio
        assert f[18] == pytest.approx(0.0)   # block
        assert f[19] == pytest.approx(0.0)   # alive
        assert f[20] == pytest.approx(0.0)   # damage

    def test_alive_monsters_in_multi(self):
        f = featurize_summary(MULTI_MONSTER_STATE)
        # Monster 0: alive, hp=3/3, block=0, damage=1
        assert f[13] == pytest.approx(3 / 3)
        assert f[14] == pytest.approx(0.0)
        assert f[15] == pytest.approx(1.0)
        assert f[16] == pytest.approx(1 / 10)
        # Monster 2: alive, hp=2/3, block=1, damage=2
        assert f[21] == pytest.approx(2 / 3)
        assert f[22] == pytest.approx(1 / 10)
        assert f[23] == pytest.approx(1.0)
        assert f[24] == pytest.approx(2 / 10)
