"""Ring buffer for training data, storing GameState objects and scalar targets."""

import random


class ReplayBuffer:
    def __init__(self, capacity: int):
        self.capacity = capacity
        self.states: list = []
        self.targets: list[float] = []
        self.pos = 0

    def add(self, state, target: float):
        if len(self.states) < self.capacity:
            self.states.append(state)
            self.targets.append(target)
        else:
            self.states[self.pos] = state
            self.targets[self.pos] = target
        self.pos = (self.pos + 1) % self.capacity

    def add_batch(self, states: list, targets: list[float]):
        for s, t in zip(states, targets):
            self.add(s, t)

    def sample(self, batch_size: int) -> tuple[list, list[float]]:
        """Sample a random batch. Returns (states, targets)."""
        n = min(batch_size, len(self))
        indices = random.sample(range(len(self)), n)
        return [self.states[i] for i in indices], [self.targets[i] for i in indices]

    def __len__(self) -> int:
        return len(self.states)
