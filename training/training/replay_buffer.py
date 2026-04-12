"""Ring buffer for training data."""

import torch


class ReplayBuffer:
    def __init__(self, capacity: int, feature_dim: int):
        self.capacity = capacity
        self.features = torch.zeros(capacity, feature_dim)
        self.values = torch.zeros(capacity)
        self.size = 0
        self.pos = 0

    def add_batch(self, features: torch.Tensor, values: torch.Tensor):
        """Add a batch of (features, values) to the buffer."""
        n = features.shape[0]
        if n == 0:
            return
        for i in range(n):
            self.features[self.pos] = features[i]
            self.values[self.pos] = values[i]
            self.pos = (self.pos + 1) % self.capacity
            self.size = min(self.size + 1, self.capacity)

    def sample(self, batch_size: int) -> tuple[torch.Tensor, torch.Tensor]:
        """Sample a random batch."""
        indices = torch.randint(0, self.size, (min(batch_size, self.size),))
        return self.features[indices], self.values[indices]

    def __len__(self) -> int:
        return self.size
