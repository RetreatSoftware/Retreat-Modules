//! Rolling an item instance (pure, from one random source).

use bevy::prelude::*;
use retreat_gameplay_api::{GridItem, GridItemCatalog, ItemBase, ItemRarity, ItemUid, RolledAffix};

/// A random source: `below(n)` is a number in `0..n` (`n > 0`).
pub trait Dice {
    fn below(&mut self, n: u64) -> u64;
}

/// A number in `min..=max`.
fn between(dice: &mut impl Dice, min: i64, max: i64) -> i64 {
    min + dice.below((max - min + 1) as u64) as i64
}

/// Picks a rarity by weight.
pub fn pick_rarity<'a>(rarities: &'a [ItemRarity], dice: &mut impl Dice) -> Option<&'a ItemRarity> {
    let total: u64 = rarities.iter().map(|rarity| u64::from(rarity.weight)).sum();
    if total == 0 {
        return None;
    }
    let mut roll = dice.below(total);
    for rarity in rarities {
        let weight = u64::from(rarity.weight);
        if roll < weight {
            return Some(rarity);
        }
        roll -= weight;
    }
    None
}

/// Rolls an item of `base` and `rarity` at `item_level` (at least 1): as many different affixes
/// from the base's pool as the rarity allows (fewer when the pool is smaller), each value in the
/// affix's range moved up by `per_level` for every level above 1. The level requirement is the
/// item level (placeholder rule, `ItemRollsConfig` docs). The uid is left 0 for the caller.
pub fn roll_item(
    catalog: &GridItemCatalog,
    base: &ItemBase,
    rarity: &ItemRarity,
    item_level: u32,
    dice: &mut impl Dice,
) -> GridItem {
    let item_level = item_level.max(1);
    let count = between(
        dice,
        i64::from(rarity.affixes.0),
        i64::from(rarity.affixes.1),
    ) as usize;
    let mut pool: Vec<&str> = base.affix_pool.iter().map(String::as_str).collect();
    let mut affixes = Vec::new();
    while affixes.len() < count && !pool.is_empty() {
        let index = dice.below(pool.len() as u64) as usize;
        let id = pool.remove(index);
        let Some(affix) = catalog.affix(id) else {
            continue;
        };
        let lift = i64::from(affix.per_level) * i64::from(item_level - 1);
        let value = between(
            dice,
            i64::from(affix.min) + lift,
            i64::from(affix.max) + lift,
        );
        affixes.push(RolledAffix {
            affix: affix.id.clone(),
            value: value as i32,
        });
    }
    affixes.sort_by(|a, b| a.affix.cmp(&b.affix));
    GridItem {
        uid: ItemUid(0),
        base: base.id.clone(),
        rarity: rarity.id.clone(),
        affixes,
        level_requirement: item_level,
        quantity: 1,
        size: base.size,
    }
}

/// Rolls a random rollable base at a random rarity.
pub fn roll_random(
    catalog: &GridItemCatalog,
    item_level: u32,
    dice: &mut impl Dice,
) -> Option<GridItem> {
    let bases: Vec<&ItemBase> = catalog.bases.iter().filter(|base| base.rollable).collect();
    if bases.is_empty() {
        return None;
    }
    let base = bases[dice.below(bases.len() as u64) as usize];
    let rarity = pick_rarity(&catalog.rarities, dice)?;
    Some(roll_item(catalog, base, rarity, item_level, dice))
}

#[cfg(test)]
mod tests {
    use super::*;
    use retreat_gameplay_api::{EquipSlot, ItemAffix};

    /// Returns the listed numbers in turn (each kept below `n`).
    struct Fixed(Vec<u64>, usize);
    impl Dice for Fixed {
        fn below(&mut self, n: u64) -> u64 {
            let value = self.0[self.1 % self.0.len()] % n;
            self.1 += 1;
            value
        }
    }

    fn catalog() -> GridItemCatalog {
        let affix = |id: &str| ItemAffix {
            id: id.to_string(),
            text: format!("+{{}} {id}"),
            min: 1,
            max: 3,
            per_level: 2,
        };
        let rarity = |id: &str, weight: u32, affixes: (u32, u32)| ItemRarity {
            id: id.to_string(),
            name: id.to_string(),
            weight,
            affixes,
            color: [1.0; 3],
        };
        GridItemCatalog {
            bases: vec![ItemBase {
                id: "sword".to_string(),
                name: "Sword".to_string(),
                size: UVec2::new(1, 3),
                slots: vec![EquipSlot::MainHand],
                two_handed: false,
                max_stack: 1,
                rollable: true,
                affix_pool: vec!["strength".to_string(), "speed".to_string()],
                icon: None,
                color: [0.5; 3],
            }],
            rarities: vec![rarity("common", 3, (0, 0)), rarity("rare", 1, (2, 2))],
            affixes: vec![affix("strength"), affix("speed")],
        }
    }

    #[test]
    fn rarity_follows_the_weights() {
        let catalog = catalog();
        let pick = |roll: u64| {
            pick_rarity(&catalog.rarities, &mut Fixed(vec![roll], 0))
                .unwrap()
                .id
                .clone()
        };
        assert_eq!(pick(0), "common");
        assert_eq!(pick(2), "common");
        assert_eq!(pick(3), "rare");
    }

    #[test]
    fn a_rare_item_rolls_different_affixes_lifted_by_its_level() {
        let catalog = catalog();
        let item = roll_item(
            &catalog,
            &catalog.bases[0],
            &catalog.rarities[1],
            4,
            &mut Fixed(vec![0, 0, 2, 0, 0], 0),
        );
        assert_eq!(item.level_requirement, 4);
        assert_eq!(item.size, UVec2::new(1, 3));
        assert_eq!(item.affixes.len(), 2);
        assert_ne!(item.affixes[0].affix, item.affixes[1].affix);
        for affix in &item.affixes {
            // 1..=3 lifted by 2 per level above 1: 7..=9.
            assert!((7..=9).contains(&affix.value), "{affix:?}");
        }
        let common = roll_item(
            &catalog,
            &catalog.bases[0],
            &catalog.rarities[0],
            1,
            &mut Fixed(vec![1], 0),
        );
        assert!(common.affixes.is_empty());
    }
}
