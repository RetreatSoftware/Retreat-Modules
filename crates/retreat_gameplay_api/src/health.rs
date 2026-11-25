//! Hit points. Whole numbers, so every machine computes the same result.

use bevy::prelude::*;

/// A character's hit points. `current == 0` is the one definition of "dead": there is no separate
/// dead flag.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    current: u32,
    max: u32,
}

/// What one [`Health::apply_damage`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageOutcome {
    /// Hit points actually removed (never more than were left).
    pub applied: u32,
    /// True only for the damage that took the last hit point.
    pub killed: bool,
}

impl Health {
    /// Full health. `max` is raised to 1 if 0, so a new character is never born dead.
    pub fn full(max: u32) -> Self {
        let max = max.max(1);
        Self { current: max, max }
    }

    pub fn current(&self) -> u32 {
        self.current
    }

    pub fn max(&self) -> u32 {
        self.max
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    /// Removes up to `amount` hit points. Damage to the dead does nothing.
    pub fn apply_damage(&mut self, amount: u32) -> DamageOutcome {
        let applied = amount.min(self.current);
        self.current -= applied;
        DamageOutcome {
            applied,
            killed: applied > 0 && self.current == 0,
        }
    }

    /// Back to full health (respawn, match restart).
    pub fn restore(&mut self) {
        self.current = self.max;
    }
}
