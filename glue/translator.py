"""Translates between CommunicationMod's raw JSON and our simplified game model.

CommunicationMod exposes ~13 screen types with varied structures.
We collapse these into a smaller set of semantic screens, each with
self-describing actions.
"""


def translate_state(raw):
    """Convert CommunicationMod JSON into our simplified state dict."""
    if "error" in raw:
        return {"screen": {"type": "error", "message": raw["error"]}, "actions": []}

    if not raw.get("in_game"):
        return {
            "screen": {"type": "main_menu"},
            "actions": [
                {"type": "start_run", "character": "IRONCLAD", "label": "Ironclad"},
                {"type": "start_run", "character": "SILENT", "label": "Silent"},
                {"type": "start_run", "character": "DEFECT", "label": "Defect"},
                {"type": "start_run", "character": "WATCHER", "label": "Watcher"},
                {"type": "start_run", "character": "BG_IRONCLAD", "label": "Board Game Ironclad"},
                {"type": "start_run", "character": "BG_MULTICHARACTER", "label": "Board Game Multi"},
            ],
        }

    game = raw["game_state"]
    persistent = _extract_persistent(game)
    available_commands = raw.get("available_commands", [])
    screen, actions = _translate_screen(game, available_commands)

    # Potion use/discard is available on most screens, not just combat
    if "potion" in available_commands:
        actions.extend(_potion_actions(game))

    # Clickable relics (Board Game mod)
    if "relic" in available_commands:
        actions.extend(_clickable_relic_actions(game))

    return {
        **persistent,
        "screen": screen,
        "actions": actions,
    }


def to_commod_command(action, raw_state):
    """Convert a simplified action dict into a CommunicationMod command string."""
    atype = action["type"]

    if atype == "start_run":
        return f"start {action['character']}"

    if atype == "pick_event_option":
        return f"choose {action['choice_index']}"

    if atype == "pick_neow_blessing":
        return f"choose {action['choice_index']}"

    if atype == "travel_to":
        return f"choose {action['choice_index']}"

    if atype == "take_card":
        return f"choose {action['choice_index']}"

    if atype == "skip_card_reward":
        return "skip"

    if atype == "take_reward":
        return f"choose {action['choice_index']}"

    if atype == "proceed":
        return "proceed"

    if atype == "skip":
        return "skip"

    if atype == "rest":
        return f"choose {action['choice_index']}"

    if atype == "smith":
        return f"choose {action['choice_index']}"

    if atype == "open_chest":
        return f"choose {action['choice_index']}"

    if atype == "buy_card":
        return f"choose {action['choice_index']}"

    if atype == "buy_relic":
        return f"choose {action['choice_index']}"

    if atype == "buy_potion":
        return f"choose {action['choice_index']}"

    if atype == "purge":
        return f"choose {action['choice_index']}"

    if atype == "leave_shop":
        return "return"

    if atype == "play_card":
        idx = action["hand_index"]
        target = action.get("target_index")
        if target is not None:
            return f"play {idx + 1} {target}"
        return f"play {idx + 1}"

    if atype == "end_turn":
        return "end"

    if atype == "pick_boss_relic":
        return f"choose {action['choice_index']}"

    if atype == "skip_boss_relic":
        return "skip"

    if atype == "pick_grid_card":
        return f"choose {action['choice_index']}"

    if atype == "pick_custom_screen_option":
        return f"choose {action['choice_index']}"

    if atype == "pick_hand_card":
        return f"choose {action['choice_index']}"

    if atype == "use_potion":
        slot = action["slot"]
        target = action.get("target_index")
        if target is not None:
            return f"potion use {slot} {target}"
        return f"potion use {slot}"

    if atype == "discard_potion":
        return f"potion discard {action['slot']}"

    if atype == "use_relic":
        return f"relic {action['relic_index']}"

    if atype == "debug":
        return f"debug {action['command']}"

    raise ValueError(f"Unknown action type: {atype}")


def _extract_persistent(game):
    """Extract persistent run state that doesn't change with screens."""
    return {
        "hp": game.get("current_hp"),
        "max_hp": game.get("max_hp"),
        "gold": game.get("gold"),
        "floor": game.get("floor"),
        "act": game.get("act"),
        "ascension": game.get("ascension_level"),
        "deck": [_translate_card(c) for c in game.get("deck", [])],
        "relics": [{"id": r["id"], "name": r["name"]} for r in game.get("relics", [])],
        "potions": [
            None if p["id"] == "Potion Slot" else {"id": p["id"], "name": p["name"]}
            for p in game.get("potions", [])
        ],
    }


def _translate_card(card):
    return {
        "id": card["id"],
        "name": card["name"],
        "cost": card["cost"],
        "type": card["type"],
        "upgraded": card.get("upgrades", 0) > 0,
    }


def _clickable_relic_actions(game):
    """Generate use relic actions for clickable relics that appear usable."""
    actions = []
    relics = game.get("relics", [])
    clickable_index = 0
    for relic in relics:
        if relic.get("clickable", False):
            counter = relic.get("counter", -1)
            pulsing = relic.get("pulsing", False)
            # Usable if pulsing (die-triggered) or has charges (counter > 0)
            if pulsing or counter > 0:
                actions.append({
                    "type": "use_relic",
                    "relic": {"id": relic["id"], "name": relic["name"], "counter": counter},
                    "relic_index": clickable_index,
                })
            clickable_index += 1
    return actions


def _potion_actions(game):
    """Generate potion use/discard actions. Works on any screen."""
    actions = []
    potions = game.get("potions", [])
    combat = game.get("combat_state")
    monsters = []
    if combat:
        monsters = [
            {"name": m["name"], "index": i, "is_gone": m.get("is_gone", False)}
            for i, m in enumerate(combat.get("monsters", []))
        ]

    for slot, potion in enumerate(potions):
        if potion["id"] == "Potion Slot":
            continue
        if potion.get("can_use", False):
            if potion.get("requires_target", False):
                for m in monsters:
                    if not m["is_gone"]:
                        actions.append({
                            "type": "use_potion",
                            "potion": {"id": potion["id"], "name": potion["name"]},
                            "slot": slot,
                            "target_index": m["index"],
                            "target_name": m["name"],
                        })
            else:
                actions.append({
                    "type": "use_potion",
                    "potion": {"id": potion["id"], "name": potion["name"]},
                    "slot": slot,
                })
        if potion.get("can_discard", False):
            actions.append({
                "type": "discard_potion",
                "potion": {"id": potion["id"], "name": potion["name"]},
                "slot": slot,
            })
    return actions


def _translate_screen(game, available_commands=None):
    if available_commands is None:
        available_commands = []
    screen_type = game.get("screen_type", "NONE")
    screen_state = game.get("screen_state", {})
    choice_list = game.get("choice_list", [])

    if screen_type == "EVENT":
        return _translate_event(screen_state, game)

    if screen_type == "MAP":
        return _translate_map(game)

    if screen_type == "CARD_REWARD":
        return _translate_card_reward(screen_state, game)

    if screen_type == "COMBAT_REWARD":
        return _translate_combat_reward(screen_state, game)

    if screen_type == "BOSS_REWARD":
        return _translate_boss_reward(screen_state, game)

    if screen_type == "SHOP_ROOM":
        return {"type": "shop_room"}, [{"type": "proceed"}]

    if screen_type == "SHOP_SCREEN":
        return _translate_shop(screen_state, game)

    if screen_type == "REST":
        return _translate_rest(screen_state, choice_list, available_commands)

    if screen_type == "CHEST":
        return _translate_chest(game, available_commands)

    if screen_type == "GRID":
        return _translate_grid(screen_state, game)

    if screen_type == "HAND_SELECT":
        return _translate_hand_select(screen_state, choice_list, available_commands)

    if screen_type == "GAME_OVER":
        return _translate_game_over(screen_state, game)

    if screen_type == "CUSTOM_SCREEN":
        return _translate_custom_screen(screen_state, choice_list, available_commands)

    if screen_type == "NONE" and game.get("room_phase") == "COMBAT":
        return _translate_combat(game)

    if screen_type == "NONE" and game.get("room_phase") == "COMPLETE":
        return {"type": "complete"}, [{"type": "proceed"}]

    return {"type": "unknown", "raw_screen_type": screen_type}, []


def _translate_event(screen_state, game):
    event_id = screen_state.get("event_id", "")
    options = screen_state.get("options", [])

    # Neow is a special multi-step event
    is_neow = event_id == "Neow Event"
    action_type = "pick_neow_blessing" if is_neow else "pick_event_option"

    actions = []
    for opt in options:
        if not opt.get("disabled", False):
            actions.append({
                "type": action_type,
                "label": opt["label"],
                "choice_index": opt["choice_index"],
            })

    screen = {
        "type": "neow" if is_neow else "event",
        "event_id": event_id,
        "event_name": screen_state.get("event_name", ""),
        "options": [
            {
                "label": opt["label"],
                "disabled": opt.get("disabled", False),
            }
            for opt in options
        ],
    }

    return screen, actions


SYMBOL_TO_KIND = {
    "M": "monster",
    "E": "elite",
    "R": "rest",
    "$": "shop",
    "?": "event",
    "T": "treasure",
    "B": "boss",
}


def _translate_map(game):
    choice_list = game.get("choice_list", [])
    screen_state = game.get("screen_state", {})
    next_nodes = screen_state.get("next_nodes", [])

    # Build lookup from x to next_node for matching with choice_list
    next_by_x = {n["x"]: n for n in next_nodes}

    actions = []
    nodes = []
    for i, choice in enumerate(choice_list):
        # choice is "x=N" or "boss"
        if choice == "boss":
            kind = "boss"
        else:
            x = int(choice.split("=")[1])
            map_node = next_by_x.get(x, {})
            kind = SYMBOL_TO_KIND.get(map_node.get("symbol", "?"), "unknown")

        node_info = {"label": choice, "kind": kind}
        nodes.append(node_info)
        actions.append({
            "type": "travel_to",
            "kind": kind,
            "label": choice,
            "choice_index": i,
        })

    screen = {
        "type": "map",
        "available_nodes": nodes,
    }

    return screen, actions


def _translate_card_reward(screen_state, game):
    cards = screen_state.get("cards", [])

    actions = [
        {
            "type": "take_card",
            "card": _translate_card(card),
            "choice_index": i,
        }
        for i, card in enumerate(cards)
    ]
    actions.append({"type": "skip_card_reward"})

    screen = {
        "type": "card_reward",
        "cards": [_translate_card(c) for c in cards],
    }

    return screen, actions


def _translate_combat_reward(screen_state, game):
    rewards = screen_state.get("rewards", [])

    actions = [
        {
            "type": "take_reward",
            "reward": {
                "type": rewards[i].get("reward_type", "UNKNOWN"),
                "gold": rewards[i].get("gold"),
                "relic": rewards[i].get("relic"),
                "potion": rewards[i].get("potion"),
            },
            "choice_index": i,
        }
        for i in range(len(rewards))
    ]
    actions.append({"type": "proceed"})

    screen = {
        "type": "combat_rewards",
        "rewards": [
            {
                "type": r.get("reward_type", "UNKNOWN"),
                "gold": r.get("gold"),
                "relic": r.get("relic"),
                "potion": r.get("potion"),
            }
            for r in rewards
        ],
    }

    return screen, actions


def _translate_boss_reward(screen_state, game):
    relics = screen_state.get("relics", [])

    actions = [
        {
            "type": "pick_boss_relic",
            "relic": {"id": r["id"], "name": r["name"]},
            "choice_index": i,
        }
        for i, r in enumerate(relics)
    ]
    actions.append({"type": "skip_boss_relic"})

    screen = {
        "type": "boss_relic",
        "relics": [{"id": r["id"], "name": r["name"]} for r in relics],
    }

    return screen, actions


def _translate_shop(screen_state, game):
    actions = []
    cards = []
    relics = []
    potions = []
    purge_cost = screen_state.get("purge_cost")

    # Build display lists (all items, not just affordable)
    for card in screen_state.get("cards", []):
        cards.append({**_translate_card(card), "price": card.get("price")})
    for relic in screen_state.get("relics", []):
        relics.append({"id": relic["id"], "name": relic["name"], "price": relic.get("price")})
    for potion in screen_state.get("potions", []):
        potions.append({"id": potion["id"], "name": potion["name"], "price": potion.get("price")})

    # Build actions from choice_list which has the correct order and only affordable items
    choice_list = game.get("choice_list", [])
    for i, choice in enumerate(choice_list):
        if choice == "purge":
            actions.append({
                "type": "purge",
                "price": purge_cost,
                "choice_index": i,
            })
        else:
            # Match by name against cards, relics, potions
            matched = False
            for card in screen_state.get("cards", []):
                if card["name"].lower() == choice:
                    actions.append({
                        "type": "buy_card",
                        "card": _translate_card(card),
                        "price": card.get("price"),
                        "choice_index": i,
                    })
                    matched = True
                    break
            if not matched:
                for relic in screen_state.get("relics", []):
                    if relic["name"].lower() == choice:
                        actions.append({
                            "type": "buy_relic",
                            "relic": {"id": relic["id"], "name": relic["name"]},
                            "price": relic.get("price"),
                            "choice_index": i,
                        })
                        matched = True
                        break
            if not matched:
                for potion in screen_state.get("potions", []):
                    if potion["name"].lower() == choice:
                        actions.append({
                            "type": "buy_potion",
                            "potion": {"id": potion["id"], "name": potion["name"]},
                            "price": potion.get("price"),
                            "choice_index": i,
                        })
                        break

    actions.append({"type": "leave_shop"})

    screen = {
        "type": "shop",
        "cards": cards,
        "relics": relics,
        "potions": potions,
        "purge_cost": purge_cost,
    }

    return screen, actions


def _translate_rest(screen_state, choice_list, available_commands):
    actions = []
    for i, choice in enumerate(choice_list):
        if choice == "rest":
            actions.append({"type": "rest", "choice_index": i})
        elif choice == "smith":
            actions.append({"type": "smith", "choice_index": i})
        else:
            actions.append({"type": "rest_action", "label": choice, "choice_index": i})

    # After resting/smithing, no choices remain — just proceed
    if not actions and "proceed" in available_commands:
        actions.append({"type": "proceed"})

    screen = {
        "type": "rest",
        "options": choice_list,
    }

    return screen, actions


def _translate_chest(game, available_commands):
    choice_list = game.get("choice_list", [])
    actions = [{"type": "open_chest", "choice_index": i} for i in range(len(choice_list))]

    if not actions and "proceed" in available_commands:
        actions.append({"type": "proceed"})

    return {"type": "treasure"}, actions


def _translate_grid(screen_state, game):
    cards = screen_state.get("cards", [])
    for_purge = screen_state.get("for_purge", False)
    for_upgrade = screen_state.get("for_upgrade", False)
    for_transform = screen_state.get("for_transform", False)
    confirm_up = screen_state.get("confirm_up", False)

    if confirm_up:
        return {"type": "grid_confirm"}, [{"type": "proceed"}]

    if for_purge:
        purpose = "purge"
    elif for_upgrade:
        purpose = "upgrade"
    elif for_transform:
        purpose = "transform"
    else:
        purpose = "select"

    actions = [
        {
            "type": "pick_grid_card",
            "card": _translate_card(card),
            "choice_index": i,
        }
        for i, card in enumerate(cards)
    ]
    # Grid select screens typically allow cancelling
    if "return" in game.get("available_commands", []) or "cancel" in game.get("available_commands", []):
        actions.append({"type": "skip"})

    screen = {
        "type": "grid",
        "purpose": purpose,
        "cards": [_translate_card(c) for c in cards],
    }

    return screen, actions


def _translate_custom_screen(screen_state, choice_list, available_commands):
    screen_enum = screen_state.get("screen_enum", "UNKNOWN")
    choices = screen_state.get("choices", choice_list)

    actions = [
        {
            "type": "pick_custom_screen_option",
            "label": choice,
            "choice_index": i,
        }
        for i, choice in enumerate(choices)
    ]

    if "proceed" in available_commands or "confirm" in available_commands:
        actions.append({"type": "proceed"})

    screen = {
        "type": "custom_screen",
        "screen_enum": screen_enum,
        "options": choices,
    }

    return screen, actions


def _translate_hand_select(screen_state, choice_list, available_commands):
    cards = screen_state.get("hand", [])
    max_cards = screen_state.get("max_cards", 1)
    selected = screen_state.get("selected", [])
    can_pick_zero = screen_state.get("can_pick_zero", False)

    actions = [
        {
            "type": "pick_hand_card",
            "card": _translate_card(card),
            "choice_index": i,
        }
        for i, card in enumerate(cards)
    ]

    if "confirm" in available_commands:
        actions.append({"type": "proceed"})

    screen = {
        "type": "hand_select",
        "max_cards": max_cards,
        "selected": [_translate_card(c) for c in selected],
        "cards": [_translate_card(c) for c in cards],
    }

    return screen, actions


def _translate_game_over(screen_state, game):
    victory = screen_state.get("victory", False)
    score = screen_state.get("score", 0)
    return {"type": "game_over", "victory": victory, "score": score}, [{"type": "proceed"}]


def _translate_combat(game):
    combat = game.get("combat_state", {})

    hand = [_translate_card(c) for c in combat.get("hand", [])]
    monsters = [
        {
            "id": m["id"],
            "name": m["name"],
            "hp": m["current_hp"],
            "max_hp": m["max_hp"],
            "block": m.get("block", 0),
            "intent": m.get("intent", "UNKNOWN"),
            "damage": m.get("move_adjusted_damage", m.get("move_base_damage")),
            "hits": m.get("move_hits", 1),
            "powers": [{"id": p["id"], "amount": p.get("amount", 0)} for p in m.get("powers", [])],
            "is_gone": m.get("is_gone", False),
        }
        for m in combat.get("monsters", [])
    ]
    player = combat.get("player", {})

    actions = []
    for i, card in enumerate(hand):
        is_playable = combat["hand"][i].get("is_playable", False)
        if not is_playable:
            continue
        has_target = combat["hand"][i].get("has_target", False)
        if has_target:
            for j, monster in enumerate(monsters):
                if not monster["is_gone"]:
                    actions.append({
                        "type": "play_card",
                        "card": card,
                        "hand_index": i,
                        "target_index": j,
                        "target_name": monster["name"],
                    })
        else:
            actions.append({
                "type": "play_card",
                "card": card,
                "hand_index": i,
            })

    actions.append({"type": "end_turn"})

    screen = {
        "type": "combat",
        "hand": hand,
        "monsters": monsters,
        "player": {
            "hp": player.get("current_hp"),
            "block": player.get("block", 0),
            "energy": player.get("energy"),
            "powers": [
                {"id": p["id"], "amount": p.get("amount", 0)}
                for p in player.get("powers", [])
            ],
            "orbs": [
                {
                    "name": o.get("name", "?"),
                    "id": o.get("id", "?"),
                    "passive_amount": o.get("passive_amount", 0),
                    "evoke_amount": o.get("evoke_amount", 0),
                }
                for o in player.get("orbs", [])
            ],
        },
        "draw_pile_count": len(combat.get("draw_pile", [])),
        "discard_pile_count": len(combat.get("discard_pile", [])),
        "exhaust_pile_count": len(combat.get("exhaust_pile", [])),
        "turn": combat.get("turn"),
    }

    return screen, actions
