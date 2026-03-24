#!/usr/bin/env python3
"""TUI for playing Slay the Spire through the CommunicationMod socket bridge."""

import json
import socket
import sys
import threading

from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical
from textual.widgets import Footer, Header, Input, Label, ListItem, ListView, RichLog, Static
from textual.suggester import SuggestFromList

from translator import translate_state, to_commod_command

try:
    from sts_simulator import GameState as SimGameState
except ImportError:
    SimGameState = None

SOCKET_PATH = "/tmp/sts_commod.sock"


def _load_debug_suggestions():
    suggestions = ["abandon", "kill all", "gold 100", "hp 999", "maxhp 999"]
    try:
        import os
        ids_path = os.path.join(os.path.dirname(__file__), "bg_ids.json")
        with open(ids_path) as f:
            ids = json.load(f)
        for card in ids.get("cards", []):
            suggestions.append(f"deck add {card}")
            suggestions.append(f"deck remove {card}")
        for relic in ids.get("relics", []):
            suggestions.append(f"relic add {relic}")
            suggestions.append(f"relic remove {relic}")
        for potion in ids.get("potions", []):
            suggestions.append(f"potion add {potion}")
        for event in ids.get("events", []):
            suggestions.append(f"event {event}")
        for encounter in ids.get("encounters", []):
            suggestions.append(f"fight {encounter}")
    except Exception:
        pass
    return suggestions


DEBUG_SUGGESTIONS = _load_debug_suggestions()


class GameClient:
    """Manages the socket connection to the CommunicationMod bridge.

    A background thread reads all incoming state updates. The latest
    ready_for_command=true state is always available. When a command is
    sent, we wait for a new state update to arrive.
    """

    def __init__(self, on_state_update=None):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.connect(SOCKET_PATH)
        self._buf = b""
        self._lock = threading.Lock()
        self._latest_raw = None
        self._update_event = threading.Event()
        self._on_state_update = on_state_update
        self._reader = threading.Thread(target=self._read_loop, daemon=True)
        self._reader.start()

    def _read_loop(self):
        while True:
            try:
                while b"\n" not in self._buf:
                    chunk = self.sock.recv(65536)
                    if not chunk:
                        return
                    self._buf += chunk
                line, self._buf = self._buf.split(b"\n", 1)
                raw = json.loads(line)
                if raw.get("ready_for_command", False):
                    with self._lock:
                        self._latest_raw = raw
                    self._update_event.set()
                    if self._on_state_update:
                        self._on_state_update(raw)
            except Exception:
                return

    def _wait_for_update(self, timeout=10):
        """Wait for the next ready state update."""
        self._update_event.wait(timeout=timeout)
        self._update_event.clear()
        with self._lock:
            return self._latest_raw

    def get_state(self):
        self.sock.sendall(b"state\n")
        raw = self._wait_for_update()
        if raw is None:
            raise ConnectionError("No response from game (timeout)")
        return raw, translate_state(raw)

    def send_command(self, action, raw_state):
        """Send a command without waiting for a response."""
        cmd = to_commod_command(action, raw_state)
        self.sock.sendall((cmd + "\n").encode())

    def perform(self, action, raw_state):
        cmd = to_commod_command(action, raw_state)
        # Clear so we wait for a fresh update after our command
        self._update_event.clear()
        with self._lock:
            self._latest_raw = None
        self.sock.sendall((cmd + "\n").encode())
        raw = self._wait_for_update()
        return raw, translate_state(raw)

    def close(self):
        self.sock.close()


class StatusPanel(Static):
    """Displays persistent game state: HP, gold, floor, etc."""

    def __init__(self, **kwargs):
        super().__init__(markup=False, **kwargs)

    def update_state(self, state):
        screen_type = state.get("screen", {}).get("type", "")
        if screen_type == "main_menu":
            self.update("Slay the Spire — Main Menu")
            return

        hp = state.get("hp", "?")
        max_hp = state.get("max_hp", "?")
        gold = state.get("gold", "?")
        floor = state.get("floor", "?")
        act = state.get("act", "?")
        ascension = state.get("ascension", 0)

        relics = ", ".join(r["name"] for r in state.get("relics", []))
        potions_list = state.get("potions", [])
        potions = ", ".join(
            p["name"] if p else "empty"
            for p in potions_list
        )
        deck_size = len(state.get("deck", []))

        lines = [
            f"HP: {hp}/{max_hp}  Gold: {gold}  Floor: {floor}  Act: {act}  Asc: {ascension}",
            f"Deck: {deck_size} cards  Relics: {relics}",
            f"Potions: [{potions}]",
        ]
        self.update("\n".join(lines))


class ScreenPanel(Static):
    """Displays the current screen context."""

    def __init__(self, **kwargs):
        super().__init__(markup=False, **kwargs)

    def update_screen(self, screen):
        screen_type = screen.get("type", "unknown")

        if screen_type == "combat":
            lines = self._render_combat(screen)
        elif screen_type == "neow":
            lines = ["=== Neow's Blessing ==="]
            for opt in screen.get("options", []):
                disabled = " (disabled)" if opt.get("disabled") else ""
                lines.append(f"  • {opt['label']}{disabled}")
        elif screen_type == "event":
            lines = [f"=== Event: {screen.get('event_name', '?')} ==="]
            for opt in screen.get("options", []):
                disabled = " (disabled)" if opt.get("disabled") else ""
                lines.append(f"  • {opt['label']}{disabled}")
        elif screen_type == "map":
            lines = ["=== Choose your path ==="]
            for node in screen.get("available_nodes", []):
                lines.append(f"  • {node['kind']} ({node['label']})")
        elif screen_type == "card_reward":
            lines = ["=== Card Reward ==="]
            for card in screen.get("cards", []):
                lines.append(f"  • {card['name']} ({card['type']}, cost {card['cost']})")
        elif screen_type == "combat_rewards":
            lines = ["=== Rewards ==="]
            for r in screen.get("rewards", []):
                if r["type"] == "GOLD":
                    lines.append(f"  • {r.get('gold', '?')} Gold")
                elif r["type"] == "CARD":
                    lines.append(f"  • Card reward")
                elif r["type"] == "RELIC":
                    lines.append(f"  • Relic: {r.get('relic', {}).get('name', '?')}")
                elif r["type"] == "POTION":
                    lines.append(f"  • Potion: {r.get('potion', {}).get('name', '?')}")
                else:
                    lines.append(f"  • {r['type']}")
        elif screen_type == "shop":
            lines = self._render_shop(screen)
        elif screen_type == "rest":
            lines = ["=== Rest Site ==="]
            for opt in screen.get("options", []):
                lines.append(f"  • {opt}")
        elif screen_type == "treasure":
            lines = ["=== Treasure! ==="]
        elif screen_type == "boss_relic":
            lines = ["=== Boss Relic Reward ==="]
            for r in screen.get("relics", []):
                lines.append(f"  • {r['name']}")
        elif screen_type == "game_over":
            if screen.get("victory"):
                lines = ["=== Victory! ==="]
            else:
                lines = ["=== Defeated ==="]
        elif screen_type == "main_menu":
            lines = ["=== Main Menu ==="]
        elif screen_type == "grid":
            purpose = screen.get("purpose", "select")
            lines = [f"=== Select a card to {purpose} ==="]
            for card in screen.get("cards", []):
                upgraded = "+" if card.get("upgraded") else ""
                lines.append(f"  • {card['name']}{upgraded} ({card['type']}, cost {card['cost']})")
        elif screen_type == "hand_select":
            max_cards = screen.get("max_cards", 1)
            lines = [f"=== Select {max_cards} card(s) from hand ==="]
            for card in screen.get("cards", []):
                upgraded = "+" if card.get("upgraded") else ""
                lines.append(f"  • {card['name']}{upgraded} ({card['type']}, cost {card['cost']})")
        elif screen_type == "grid_confirm":
            lines = ["=== Confirm selection ==="]
        elif screen_type == "complete":
            lines = ["Room complete. Proceed."]
        elif screen_type == "custom_screen":
            lines = [f"=== {screen.get('screen_enum', 'Custom Screen')} ==="]
            for opt in screen.get("options", []):
                lines.append(f"  • {opt}")
        elif screen_type == "error":
            lines = [f"ERROR: {screen.get('message', 'Unknown error')}"]
        else:
            lines = [f"Screen: {screen_type}"]

        self.update("\n".join(lines))

    def _render_combat(self, screen):
        player = screen.get("player", {})
        lines = [
            f"=== Combat ===  "
            f"Energy: {player.get('energy', '?')}  "
            f"Block: {player.get('block', 0)}  "
            f"Turn: {screen.get('turn', '?')}",
            "",
        ]

        # Orbs
        orbs = player.get("orbs", [])
        if orbs:
            orb_strs = []
            for o in orbs:
                if o["name"] == "Orb Slot":
                    orb_strs.append("(empty)")
                else:
                    orb_strs.append(f"{o['name']} (P:{o.get('passive_amount', 0)} E:{o.get('evoke_amount', 0)})")
            lines.append(f"Orbs: {', '.join(orb_strs)}")

        # Player powers
        powers = player.get("powers", [])
        if powers:
            pstr = ", ".join(f"{p['id']}({p['amount']})" for p in powers)
            lines.append(f"Buffs: {pstr}")

        # Monsters
        lines.append("Monsters:")
        for m in screen.get("monsters", []):
            if m.get("is_gone"):
                continue
            mpowers = ""
            if m.get("powers"):
                mpowers = " " + ", ".join(
                    f"{p['id']}({p['amount']})" for p in m["powers"]
                )
            damage = m.get("damage")
            hits = m.get("hits", 1)
            intent = m.get("intent", "?")
            if damage and damage > 0:
                dmg_str = f"{damage}x{hits}" if hits > 1 else str(damage)
                intent_str = f"{intent} ({dmg_str} dmg)"
            else:
                intent_str = intent
            lines.append(
                f"  {m['name']}: {m['hp']}/{m['max_hp']} HP "
                f"(block {m['block']}) {intent_str}{mpowers}"
            )

        # Hand
        lines.append("")
        lines.append("Hand:")
        for card in screen.get("hand", []):
            upgraded = "+" if card.get("upgraded") else ""
            lines.append(f"  {card['name']}{upgraded} (cost {card['cost']}, {card['type']})")

        lines.append("")
        lines.append(
            f"Draw: {screen.get('draw_pile_count', '?')}  "
            f"Discard: {screen.get('discard_pile_count', '?')}  "
            f"Exhaust: {screen.get('exhaust_pile_count', '?')}"
        )

        return lines

    def _render_shop(self, screen):
        lines = ["=== Shop ==="]
        if screen.get("cards"):
            lines.append("Cards:")
            for c in screen["cards"]:
                lines.append(f"  {c['name']} ({c['type']}, cost {c['cost']}) - {c.get('price', '?')}g")
        if screen.get("relics"):
            lines.append("Relics:")
            for r in screen["relics"]:
                lines.append(f"  {r['name']} - {r.get('price', '?')}g")
        if screen.get("potions"):
            lines.append("Potions:")
            for p in screen["potions"]:
                lines.append(f"  {p['name']} - {p.get('price', '?')}g")
        purge = screen.get("purge_cost")
        if purge and purge > 0:
            lines.append(f"Purge a card: {purge}g")
        return lines


class ActionList(ListView):
    """Selectable list of available actions."""

    pass


class STSApp(App):
    """Slay the Spire TUI."""

    CSS = """
    #status {
        height: 4;
        padding: 0 1;
        background: $surface;
        border-bottom: solid $primary;
    }
    #main {
        height: 1fr;
    }
    #screen-panel {
        width: 1fr;
        padding: 1;
    }
    #action-panel {
        width: 40;
        border-left: solid $primary;
    }
    #debug-input {
        height: 3;
        padding: 0 1;
        background: $surface;
        border-top: solid $primary;
        display: none;
    }
    #debug-input.visible {
        display: block;
    }
    #log {
        height: 12;
        padding: 0 1;
        background: $surface;
        border-top: solid $primary;
    }
    """

    BINDINGS = [
        ("q", "quit", "Quit"),
        ("d", "toggle_debug", "Debug"),
        ("s", "sim_sync", "Sync sim"),
        ("v", "sim_verify", "Verify sim"),
        ("i", "sim_inspect", "Inspect sim"),
    ]

    # Actions the simulator knows how to apply
    SIM_ACTIONS = {
        "pick_neow_blessing", "pick_event_option", "travel_to",
        "take_card", "skip_card_reward", "take_reward",
        "pick_boss_relic", "skip_boss_relic",
        "buy_card", "buy_relic", "buy_potion", "purge", "leave_shop",
        "rest", "smith", "open_chest",
        "pick_grid_card", "pick_hand_card",
        "pick_custom_screen_option", "proceed", "skip",
    }

    def __init__(self):
        super().__init__()
        self.client = None
        self.raw_state = None
        self.translated = None
        self.actions = []
        self.sim = None  # PyO3 GameState, set on sync

    def compose(self) -> ComposeResult:
        yield Header()
        yield StatusPanel(id="status")
        with Horizontal(id="main"):
            yield ScreenPanel(id="screen-panel")
            yield ActionList(id="action-panel")
        yield Input(
            placeholder="debug command (e.g., event BGFaceTrader, fight Cultist, abandon)",
            id="debug-input",
            suggester=SuggestFromList(DEBUG_SUGGESTIONS, case_sensitive=False),
        )
        yield RichLog(id="log", markup=False)
        yield Footer()

    def on_mount(self):
        self.title = "Slay the Spire"
        try:
            self.client = GameClient(on_state_update=self._on_async_state)
            self._log("Connected to game")
            self._refresh_state()
        except Exception as e:
            self._log(f"Connection failed: {e}")

    def _on_async_state(self, raw):
        """Called from background thread when a new state arrives."""
        self.call_from_thread(self._apply_state, raw)

    def on_list_view_selected(self, event: ListView.Selected):
        idx = event.list_view.index
        if idx is None or idx >= len(self.actions):
            return

        action = self.actions[idx]
        self._log(f"> {self._format_action(action)}")

        try:
            self.client.send_command(action, self.raw_state)
            # Display will be updated by the async state callback
        except Exception as e:
            self._log(f"Error: {e}")

        # Forward to simulator if synced and action type is supported
        self._sim_apply(action)

    def _apply_state(self, raw):
        """Apply a raw state update to the display."""
        self.raw_state = raw
        self.translated = translate_state(raw)
        self._update_display()

    def _refresh_state(self):
        self.raw_state, self.translated = self.client.get_state()
        self._update_display()

    def _update_display(self):
        self.actions = self.translated.get("actions", [])

        self.query_one("#status", StatusPanel).update_state(self.translated)
        self.query_one("#screen-panel", ScreenPanel).update_screen(
            self.translated.get("screen", {})
        )

        action_list = self.query_one("#action-panel", ActionList)
        action_list.clear()
        for action in self.actions:
            action_list.append(ListItem(Label(self._format_action(action))))

    def _format_action(self, action):
        atype = action.get("type", "?")

        if atype == "start_run":
            return f"Start Run: {action.get('label', action.get('character', '?'))}"
        if atype == "pick_neow_blessing":
            return f"Neow: {action.get('label', '?')}"
        if atype == "pick_event_option":
            return f"Event: {action.get('label', '?')}"
        if atype == "travel_to":
            return f"Travel: {action.get('kind', '?')} ({action.get('label', '')})"
        if atype == "play_card":
            card = action.get("card", {})
            target = action.get("target_name")
            s = f"Play {card.get('name', '?')} (cost {card.get('cost', '?')})"
            if target:
                s += f" → {target}"
            return s
        if atype == "end_turn":
            return "End Turn"
        if atype == "take_card":
            card = action.get("card", {})
            return f"Take {card.get('name', '?')} ({card.get('type', '')}, cost {card.get('cost', '?')})"
        if atype == "skip_card_reward":
            return "Skip card reward"
        if atype == "take_reward":
            r = action.get("reward", {})
            if r.get("type") == "GOLD":
                return f"Take {r.get('gold', '?')} gold"
            if r.get("type") == "CARD":
                return "View card reward"
            if r.get("type") == "RELIC":
                return f"Take relic: {r.get('relic', {}).get('name', '?')}"
            if r.get("type") == "POTION":
                return f"Take potion: {r.get('potion', {}).get('name', '?')}"
            return f"Take {r.get('type', '?')}"
        if atype == "rest":
            return "Rest (heal)"
        if atype == "smith":
            return "Smith (upgrade)"
        if atype == "buy_card":
            card = action.get("card", {})
            return f"Buy {card.get('name', '?')} ({action.get('price', '?')}g)"
        if atype == "buy_relic":
            return f"Buy {action.get('relic', {}).get('name', '?')} ({action.get('price', '?')}g)"
        if atype == "buy_potion":
            return f"Buy {action.get('potion', {}).get('name', '?')} ({action.get('price', '?')}g)"
        if atype == "purge":
            return f"Purge a card ({action.get('price', '?')}g)"
        if atype == "leave_shop":
            return "Leave shop"
        if atype == "open_chest":
            return "Open chest"
        if atype == "pick_boss_relic":
            return f"Take {action.get('relic', {}).get('name', '?')}"
        if atype == "skip_boss_relic":
            return "Skip boss relic"
        if atype == "pick_grid_card":
            card = action.get("card", {})
            return f"Select {card.get('name', '?')} ({card.get('type', '')}, cost {card.get('cost', '?')})"
        if atype == "pick_hand_card":
            card = action.get("card", {})
            return f"Discard {card.get('name', '?')} ({card.get('type', '')}, cost {card.get('cost', '?')})"
        if atype == "pick_custom_screen_option":
            return action.get("label", "?")
        if atype == "use_potion":
            potion = action.get("potion", {})
            target = action.get("target_name")
            s = f"Use {potion.get('name', '?')}"
            if target:
                s += f" → {target}"
            return s
        if atype == "discard_potion":
            potion = action.get("potion", {})
            return f"Discard {potion.get('name', '?')}"
        if atype == "use_relic":
            relic = action.get("relic", {})
            counter = relic.get("counter", 0)
            return f"Use {relic.get('name', '?')} (x{counter})"
        if atype == "proceed":
            return "Proceed"
        if atype == "skip":
            return "Skip"

        return f"{atype}: {action}"

    def action_toggle_debug(self):
        debug_input = self.query_one("#debug-input", Input)
        debug_input.toggle_class("visible")
        if debug_input.has_class("visible"):
            debug_input.focus()
        else:
            self.query_one("#action-panel", ActionList).focus()

    def on_input_changed(self, event: Input.Changed):
        """Close debug pane on escape by checking if input lost focus."""
        pass

    def key_escape(self):
        debug_input = self.query_one("#debug-input", Input)
        if debug_input.has_class("visible"):
            debug_input.remove_class("visible")
            self.query_one("#action-panel", ActionList).focus()

    def on_input_submitted(self, event: Input.Submitted):
        if event.input.id == "debug-input":
            cmd = event.value.strip()
            if cmd:
                self._log(f"[debug] {cmd}")
                try:
                    self.client.send_command(
                        {"type": "debug", "command": cmd}, self.raw_state
                    )
                except Exception as e:
                    self._log(f"Debug error: {e}")
            event.input.value = ""

    def on_key(self, event):
        """Prevent tab from shifting focus when debug pane is open."""
        debug_input = self.query_one("#debug-input", Input)
        if debug_input.has_class("visible") and event.key == "tab":
            event.prevent_default()
            event.stop()

    def _sim_apply(self, action):
        """Forward an action to the simulator if it's synced and the action is supported."""
        if self.sim is None:
            return
        atype = action.get("type", "")
        if atype not in self.SIM_ACTIONS:
            return
        try:
            self.sim.apply(json.dumps(action))
        except BaseException as e:
            self._log(f"[sim] apply error: {e}")

    def action_sim_sync(self):
        """Sync: load current live game state into the simulator."""
        if SimGameState is None:
            self._log("[sim] sts_simulator not installed")
            return
        if self.translated is None:
            self._log("[sim] No game state to sync")
            return
        try:
            state_json = json.dumps(self.translated)
            self.sim = SimGameState.from_json(state_json)
            self._log("[sim] Synced — simulator loaded with live state")
        except Exception as e:
            self._log(f"[sim] Sync error: {e}")

    def action_sim_verify(self):
        """Verify: compare simulator state with current live game state."""
        if self.sim is None:
            self._log("[sim] Not synced — press 's' first")
            return
        if self.translated is None:
            self._log("[sim] No game state to compare")
            return
        try:
            sim_state = json.loads(self.sim.to_json())
            live = self.translated
            diffs = []
            for key in ("hp", "max_hp", "gold", "floor"):
                sv = sim_state.get(key)
                lv = live.get(key)
                if sv != lv:
                    diffs.append(f"  {key}: sim={sv} live={lv}")
            # Compare deck sizes and contents
            sim_deck = sorted(c["id"] for c in sim_state.get("deck", []))
            live_deck = sorted(c["id"] for c in live.get("deck", []))
            if sim_deck != live_deck:
                diffs.append(f"  deck: sim={len(sim_deck)} cards, live={len(live_deck)} cards")
                added = [c for c in sim_deck if c not in live_deck]
                removed = [c for c in live_deck if c not in sim_deck]
                if added:
                    diffs.append(f"    sim has extra: {added}")
                if removed:
                    diffs.append(f"    live has extra: {removed}")
            # Compare relics
            sim_relics = sorted(r["id"] for r in sim_state.get("relics", []))
            live_relics = sorted(r["id"] for r in live.get("relics", []))
            if sim_relics != live_relics:
                diffs.append(f"  relics: sim={sim_relics} live={live_relics}")
            # Compare screen type
            sim_screen = sim_state.get("screen", {}).get("type", "?")
            live_screen = live.get("screen", {}).get("type", "?")
            if sim_screen != live_screen:
                diffs.append(f"  screen: sim={sim_screen} live={live_screen}")
            if diffs:
                self._log("[sim] MISMATCH:\n" + "\n".join(diffs))
            else:
                self._log("[sim] OK — states match")
        except Exception as e:
            self._log(f"[sim] Verify error: {e}")

    def action_sim_inspect(self):
        """Inspect: dump simulator state to log."""
        if self.sim is None:
            self._log("[sim] Not synced — press 's' first")
            return
        try:
            s = json.loads(self.sim.to_json())
            deck = [c["id"] + ("+" if c.get("upgraded") else "") for c in s.get("deck", [])]
            relics = [r["id"] for r in s.get("relics", [])]
            potions = [p["id"] if p else "empty" for p in s.get("potions", [])]
            screen_type = s.get("screen", {}).get("type", "?")
            lines = [
                f"[sim] HP:{s['hp']}/{s['max_hp']} Gold:{s['gold']} Floor:{s['floor']}",
                f"  Deck({len(deck)}): {', '.join(deck)}",
                f"  Relics: {', '.join(relics)}",
                f"  Potions: [{', '.join(potions)}]",
                f"  Screen: {screen_type}",
            ]
            self._log("\n".join(lines))
        except BaseException as e:
            self._log(f"[sim] Inspect error: {e}")

    def _log(self, msg):
        log_widget = self.query_one("#log", RichLog)
        log_widget.write(msg)

    def on_unmount(self):
        if self.client:
            self.client.close()


if __name__ == "__main__":
    app = STSApp()
    app.run()
