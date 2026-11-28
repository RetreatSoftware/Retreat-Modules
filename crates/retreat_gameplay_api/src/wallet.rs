//! The safe balance and service spots (crawler plan, phase 2).
//!
//! Currency is both (decided with the user): gold found is a carried item (lost on death, phase 4)
//! until it is deposited into the character's safe balance in the city. The `wallet` module owns
//! [`Wallet`]; deposits and withdrawals change the bag and the balance in one
//! [`crate::ItemTransaction`].
//!
//! [`ServiceSpots`] are map data: places in the world where a service is used (the town chest, the
//! deposit counter), each a kind and a radius.

use bevy::prelude::*;
use retreat_engine_api::NetState;
use serde::{Deserialize, Serialize};

use crate::CharacterId;

/// A character's safe balance (on its entity `wallet_<character>`; shared, private to the owner).
#[derive(Component, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wallet {
    pub character: CharacterId,
    pub balance: u64,
}

impl NetState for Wallet {
    const ID: &'static str = "gameplay.wallet";
}

/// The entity name of a character's wallet.
pub fn wallet_entity_name(character: &CharacterId) -> String {
    format!("wallet_{}", character.0)
}

/// A place where a service is used.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceSpot {
    /// What is used there (`town_chest`, `deposit`): the module of that service looks for its kind.
    pub kind: String,
    /// Where (feet height), and how near a character's feet must be.
    pub position: Vec3,
    pub radius: f32,
}

/// Every service spot of the loaded map (a resource, map data).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct ServiceSpots(pub Vec<ServiceSpot>);

impl ServiceSpots {
    /// Whether `feet` is within reach of a spot of `kind` (horizontal distance).
    pub fn near(&self, kind: &str, feet: Vec3) -> bool {
        self.0
            .iter()
            .any(|spot| spot.kind == kind && spot.position.xz().distance(feet.xz()) <= spot.radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spot_is_near_within_its_radius_of_its_kind_only() {
        let spots = ServiceSpots(vec![ServiceSpot {
            kind: "deposit".to_string(),
            position: Vec3::new(5.0, 0.0, 0.0),
            radius: 2.0,
        }]);
        assert!(spots.near("deposit", Vec3::new(6.5, 1.0, 0.5)));
        assert!(!spots.near("deposit", Vec3::new(8.0, 0.0, 0.0)));
        assert!(!spots.near("town_chest", Vec3::new(5.0, 0.0, 0.0)));
    }
}
