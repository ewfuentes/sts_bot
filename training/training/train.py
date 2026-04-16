"""Self-play training loop: MCTS with batched neural network leaf evaluation."""

import argparse
import time

import torch

from sts_simulator import MctsWorker

from .model import StateEncoder, StateEncoderConfig
from .replay_buffer import ReplayBuffer

import tqdm


def compute_targets(
    states: list, actions: list, game_results: list,
) -> list[float]:
    """Map raw game results to training target values.

    Value = floor + hp/max_hp. Same formula as the MCTS rollout heuristic.
    Each game's target is repeated for every trajectory step in that game.
    """
    targets = []
    result_idx = 0
    for i in range(len(states)):
        r = game_results[result_idx]
        hp_frac = r["hp"] / r["max_hp"] if r["max_hp"] > 0 else 0.0
        targets.append(r["floor"] + hp_frac)
        # Advance to next game result at trajectory boundary
        if actions[i] is None and result_idx + 1 < len(game_results):
            result_idx += 1
    return targets


def self_play_epoch(
    worker: MctsWorker,
    model: StateEncoder,
    num_iterations_per_step: int,
    num_games: int,
) -> tuple[list, list, list]:
    """Run all games to completion using NN leaf evaluation."""
    step = 0
    pbar = tqdm.tqdm(total=num_games, desc="Self-play", unit="game")
    prev_active = num_games
    while worker.active_game_count() > 0:
        for _ in range(num_iterations_per_step):
            leaf_states = worker.select_leaves()
            if leaf_states:
                with torch.no_grad():
                    mean, _log_var = model(leaf_states)
                    values = mean.cpu().tolist()
                worker.backprop(values)
        worker.step_games()
        step += 1
        active = worker.active_game_count()
        finished = prev_active - active
        if finished > 0:
            pbar.update(finished)
            prev_active = active
        pbar.set_postfix(active=active, step=step)
    pbar.update(prev_active)  # flush any remaining
    pbar.close()
    return worker.get_training_data()


def gaussian_nll_loss(mean: torch.Tensor, log_var: torch.Tensor, target: torch.Tensor) -> torch.Tensor:
    """Gaussian negative log-likelihood loss."""
    return 0.5 * (log_var + (target - mean) ** 2 / log_var.exp()).mean()


def main():
    parser = argparse.ArgumentParser(description="StS MCTS Training")
    parser.add_argument("--num-games", type=int, default=32)
    parser.add_argument("--num-roots", type=int, default=10)
    parser.add_argument("--exploration-constant", type=float, default=1.41)
    parser.add_argument("--iterations-per-step", type=int, default=100)
    parser.add_argument("--num-epochs", type=int, default=100)
    parser.add_argument("--train-steps-per-epoch", type=int, default=50)
    parser.add_argument("--batch-size", type=int, default=256)
    parser.add_argument("--lr", type=float, default=1e-3)
    parser.add_argument("--buffer-capacity", type=int, default=100_000)
    parser.add_argument("--combat-only", action="store_true", default=False)
    parser.add_argument("--encounter", type=str, default="BoardGame:Jaw Worm (Easy)")
    parser.add_argument("--checkpoint-dir", type=str, default="checkpoints")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--model-dim", type=int, default=64)
    parser.add_argument("--num-heads", type=int, default=4)
    parser.add_argument("--num-layers", type=int, default=2)
    args = parser.parse_args()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Using device: {device}")

    config = StateEncoderConfig(
        model_dim=args.model_dim,
        num_heads=args.num_heads,
        num_layers=args.num_layers,
    )
    model = StateEncoder(config).to(device)
    optimizer = torch.optim.Adam(model.parameters(), lr=args.lr)
    replay_buffer = ReplayBuffer(capacity=args.buffer_capacity)

    num_params = sum(p.numel() for p in model.parameters())
    print(f"Model: {num_params} parameters, dim={args.model_dim}, "
          f"heads={args.num_heads}, layers={args.num_layers}")

    for epoch in tqdm.tqdm(range(args.num_epochs), desc="Epoch Idx"):
        t0 = time.time()

        # Self-play phase
        worker = MctsWorker(
            num_games=args.num_games,
            num_roots=args.num_roots,
            exploration_constant=args.exploration_constant,
            seed=epoch * 10000,
            combat_only=args.combat_only,
            encounter=args.encounter,
            max_steps=args.max_steps,
        )

        model.eval()
        states, actions, game_results = self_play_epoch(
            worker, model, args.iterations_per_step, args.num_games)

        # Compute training targets from raw game results
        targets = compute_targets(states, actions, game_results)

        # Add to replay buffer
        if states:
            replay_buffer.add_batch(states, targets)

        # Per-game stats
        if game_results:
            wins = sum(1 for r in game_results if r["victory"])
            timed_out = sum(1 for r in game_results if r["timed_out"])
            avg_floor = sum(r["floor"] for r in game_results) / len(game_results)
            print(f"  games={len(game_results)}, wins={wins}/{len(game_results)}, "
                  f"timed_out={timed_out}, avg_floor={avg_floor:.1f}, "
                  f"floors={[r['floor'] for r in game_results]}")

        selfplay_time = time.time() - t0

        # Training phase
        if len(replay_buffer) < args.batch_size:
            print(f"Epoch {epoch}: {len(states)} states, buffer={len(replay_buffer)}, "
                  f"selfplay={selfplay_time:.1f}s (skipping training, buffer too small)")
            continue

        model.train()
        t1 = time.time()
        total_loss = 0.0
        for _ in range(args.train_steps_per_epoch):
            batch_states, batch_targets = replay_buffer.sample(args.batch_size)
            batch_target_tensor = torch.tensor(batch_targets, dtype=torch.float32, device=device)

            mean, log_var = model(batch_states)
            loss = gaussian_nll_loss(mean, log_var, batch_target_tensor)

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            total_loss += loss.item()

        avg_loss = total_loss / args.train_steps_per_epoch
        train_time = time.time() - t1

        print(f"Epoch {epoch}: {len(states)} states, loss={avg_loss:.4f}, "
              f"buffer={len(replay_buffer)}, selfplay={selfplay_time:.1f}s, train={train_time:.1f}s")

        # Checkpoint every 10 epochs
        if (epoch + 1) % 10 == 0:
            import os
            os.makedirs(args.checkpoint_dir, exist_ok=True)
            path = os.path.join(args.checkpoint_dir, f"model_epoch{epoch+1}.pt")
            torch.save({"config": config, "state_dict": model.state_dict()}, path)
            print(f"  Saved checkpoint: {path}")


if __name__ == "__main__":
    main()
