# Slay the Spire Board Game Simulator

Simulator for the **Slay the Spire Board Game mod** (not vanilla StS). The mod source is at `~/code/StSBoardGameMod/` and is the ground truth for mechanics. The goal is to build an RL training agent using MCTS and learned value functions.

## Project Structure

Rust workspace with three crates:

- **simulator/** — Game logic (state machine, effects, cards, monsters, events, relics)
- **tui/** — Terminal UI for interactive play
- **mcts/** — Generic MCTS algorithm with tic-tac-toe test domain

Python glue code in `glue/` (CommunicationMod bridge, translator).

## Architecture

### Effect Queue
The effect queue lives on `GameState` (not `Screen::Combat`), enabling effects both inside and outside combat. All game state changes should go through effects where possible. `drain_effect_queue` processes effects until the queue is empty or a sub-decision screen pauses it.

### Screen Stack
Navigation uses push/pop. Map stays under Combat and sub-screens (HandSelect, TargetSelect, ChoiceSelect, Grid, etc.). `pop_screen()` checks if the revealed Map has no available nodes and triggers `GameOver { victory: true }`. Combat-only states (no map) end in `GameOver` when combat finishes.

### Trigger System
Powers and relics share the `Trigger` enum (renamed from `PowerTrigger`). Triggers include: `StartOfCombat`, `EndOfCombat`, `PlayerEndOfTurn`, `PlayerStartOfTurn`, `OnExhaust`, `OnGainBlock`, `MonsterOnDeath`, `PlayerOnPlay { card_type }`, etc. `collect_all_triggered_effects` scans player and monster powers; `collect_relic_triggered_effects` scans relics.

### Card Play
`build_card_play_effects` is the shared function for all card-play paths (PlayCard, PlayLastDrawnFromHand/Havoc, PickAutoPlay/Distilled Chaos). It handles: effects, repeat (Double Tap/Burst), tick-down, disposition, and PlayerOnPlay triggers. XCost cards get raw effects queued; repeat and tick-down happen after energy selection.

### Borrow Checker Patterns
`find_combat_in(&mut self.screen)` is a free function (not a method) to allow simultaneous borrows of `self.screen` and `self.effect_queue`. Use this instead of `self.find_combat_mut()` when both screen and queue access is needed.

### Combat End
`Effect::CombatOver` is queued when all monsters die. It queues end-of-combat relic effects, then calls `finish_combat()` which pops the combat screen and pushes rewards. Since the combat screen is gone, the finalization check won't re-trigger.

### MCTS
Arena-based tree (`Vec<MctsNode>` with index references). Polynomial UCB bonus per Shah et al. 2022 — `t^(1/4) / s^(1/2)` not logarithmic. `mcts_adapter.rs` wraps StS GameState for the MCTS trait. Root parallelism: 10 parallel trees with different determinizations, aggregated by visit count.

## Board Game Differences from Vanilla
- Rest heals flat 3 HP (not 30%)
- Card damage values are different (e.g., Strike does 1, Defend gives 1 block)
- Die roll (1-6) each turn affects cards, relics, and events
- Player max HP is ~8-10 (not 70+)
- Weak/Vulnerable tick down per attack, not per turn

## What's Implemented
- 66 Ironclad cards with rarities (Basic/Common/Uncommon/Rare)
- 21 Act 1 monsters (all encounters including bosses)
- 21 potions (all Act 1)
- 12 Act 1 events (5 easy, 4 medium, 3 die-roll)
- 11 relics with trigger system
- Act 1 board game map with branching paths
- Shop, Rest, Treasure rooms
- Reward system (cards, relics, potions, gold)
- Golden Ticket mechanic
- Gambler's Brew die-roll modification phase
- `GameState::new_ironclad_game(seed)` for fresh games

## What's Missing
- ~67 relics (die-controlled, clickable, boss relics with energy)
- Neow blessings
- Damage bonus relics (Strike Dummy, Wrist Blade) — needs per-card bonus in damage pipeline
- A3+ events
- Other characters (Silent, Defect, Watcher)
- RL training infrastructure

## Conventions
- Use Edit tool for changes so user can review (no batch scripts)
- Don't suggest stopping or wrapping up — keep working until the user decides
- Board game mod source at ~/code/StSBoardGameMod/ is the authority for game mechanics
- Card images from TTS mod are at tts_card_art/ and tts_monster_cards/
