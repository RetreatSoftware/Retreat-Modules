//! The weapon rules and the shared inventory, with only Bevy and the contract crates.

use bevy::math::ops;
use bevy::prelude::*;
use retreat_gameplay_api::*;
use retreat_weapon_kit::*;
use std::time::Duration;

const TICKS: WeaponTicks = WeaponTicks {
    fire_interval: 10,
    reload: 20,
};

#[test]
fn seconds_become_at_least_one_tick() {
    let ticks = WeaponTicks::of(0.15, 0.0, Duration::from_secs_f64(1.0 / 64.0));
    assert_eq!((ticks.fire_interval, ticks.reload), (10, 1));
}

#[test]
fn a_shot_needs_a_round_and_a_finished_cooldown() {
    let mut weapon = loaded(WeaponKind::Rifle, 2);
    let mut cooldown = 0;
    assert!(try_fire(
        &mut weapon,
        &mut cooldown,
        TICKS,
        &mut unlimited(&ItemCatalog::default())
    ));
    assert_eq!((weapon.ammo, cooldown), (1, 10));
    assert!(!try_fire(
        &mut weapon,
        &mut cooldown,
        TICKS,
        &mut unlimited(&ItemCatalog::default())
    ));
    // A late input a few ticks early is accepted and the wait is added to the next one.
    cooldown = LATE_INPUT_TOLERANCE_TICKS;
    assert!(try_fire(
        &mut weapon,
        &mut cooldown,
        TICKS,
        &mut unlimited(&ItemCatalog::default())
    ));
    assert_eq!(cooldown, LATE_INPUT_TOLERANCE_TICKS + 10);
    cooldown = 0;
    assert!(
        !try_fire(
            &mut weapon,
            &mut cooldown,
            TICKS,
            &mut unlimited(&ItemCatalog::default())
        ),
        "empty"
    );
}

#[test]
fn an_empty_weapon_reloads_by_itself_unless_its_owner_is_dead() {
    let mut weapon = loaded(WeaponKind::Launcher, 4);
    weapon.ammo = 0;
    let mut cooldown = 2;
    advance_weapon(
        &mut weapon,
        &mut cooldown,
        TICKS,
        false,
        &mut unlimited(&ItemCatalog::default()),
    );
    assert_eq!((weapon.reload_ticks_left, cooldown), (0, 1));
    for _ in 0..=TICKS.reload {
        advance_weapon(
            &mut weapon,
            &mut cooldown,
            TICKS,
            true,
            &mut unlimited(&ItemCatalog::default()),
        );
    }
    assert_eq!((weapon.ammo, weapon.reload_ticks_left, cooldown), (4, 0, 0));
}

#[test]
fn this_machine_sends_a_shot_only_when_the_command_will_find_it_ready() {
    let mut weapon = loaded(WeaponKind::Rifle, 2);
    assert!(ready_to_send(&weapon, 1, 1, None));
    assert!(!ready_to_send(&weapon, 1, 0, None));
    weapon.ammo = 0;
    weapon.reload_ticks_left = 1;
    assert!(ready_to_send(&weapon, 0, 1, None));
    assert!(!ready_to_send(&weapon, 0, 0, None));
}

#[test]
fn two_weapons_equipped_in_one_tick_both_stay_in_a_fixed_order() {
    fn equip_both(mut commands: Commands, players: Query<Entity, With<PlayerId>>) {
        for player in &players {
            equip(&mut commands, player, loaded(WeaponKind::Launcher, 4));
            equip(&mut commands, player, loaded(WeaponKind::Rifle, 30));
        }
    }
    let mut app = App::new();
    app.add_systems(Update, equip_both);
    let player = app.world_mut().spawn(PlayerId(0)).id();
    app.update();
    let kinds: Vec<WeaponKind> = app
        .world()
        .get::<Weapons>(player)
        .unwrap()
        .iter()
        .map(|weapon| weapon.kind)
        .collect();
    assert_eq!(kinds, [WeaponKind::Rifle, WeaponKind::Launcher]);
}

#[test]
fn aiming_points_from_the_eyes_at_the_target() {
    let ahead = aim_angles(Vec3::ZERO, Vec3::NEG_Z).unwrap();
    assert_eq!((ahead.yaw, ahead.pitch), (0.0, 0.0));
    let left_up = aim_angles(Vec3::ZERO, Vec3::new(-1.0, 1.0, 0.0)).unwrap();
    assert!((left_up.yaw - std::f32::consts::FRAC_PI_2).abs() < 1.0e-6);
    assert!((left_up.pitch - ops::atan2(1.0, 1.0)).abs() < 1.0e-6);
    assert_eq!(aim_angles(Vec3::ONE, Vec3::ONE), None);
}

#[test]
fn the_click_that_takes_the_cursor_does_not_shoot() {
    let mut memory = TriggerMemory::default();
    // The press that asks for the cursor, held for a while.
    assert!(!memory.fires(true, true));
    assert!(!memory.fires(true, true));
    assert!(!memory.fires(false, true));
    assert!(memory.fires(true, true));
    // After the cursor was given back, the next click only takes it again.
    assert!(!memory.fires(false, false));
    assert!(!memory.fires(true, true));
    // A game that never captures the cursor shoots at once.
    assert!(TriggerMemory::default().fires(true, false));
}

fn unlimited(catalog: &ItemCatalog) -> AmmoSource<'_> {
    AmmoSource {
        inventory: None,
        catalog,
        weapon: WeaponKind::Rifle,
    }
}

#[test]
fn a_reload_takes_from_the_reserve_what_there_is_and_stops_when_it_is_empty() {
    let rounds = ItemDefinition::new("rounds", "Rounds", ItemKind::Ammo(WeaponKind::Rifle), 90);
    let catalog = ItemCatalog(vec![rounds.clone()]);
    let mut inventory = Inventory::new(1);
    inventory.add(&rounds, 3);
    let mut source = AmmoSource {
        inventory: Some(&mut inventory),
        catalog: &catalog,
        weapon: WeaponKind::Rifle,
    };
    let mut weapon = loaded(WeaponKind::Rifle, 30);
    weapon.ammo = 0;
    let mut cooldown = 0;
    for _ in 0..=TICKS.reload {
        advance_weapon(&mut weapon, &mut cooldown, TICKS, true, &mut source);
    }
    assert_eq!((weapon.ammo, source.reserve()), (3, Some(0)));
    assert!(!try_reload(&mut weapon, TICKS, source.reserve()));
    weapon.ammo = 0;
    advance_weapon(&mut weapon, &mut cooldown, TICKS, true, &mut source);
    assert_eq!(weapon.reload_ticks_left, 0);
    assert!(!ready_to_send(&weapon, 0, 1, source.reserve()));
    assert!(!try_fire(&mut weapon, &mut cooldown, TICKS, &mut source));
}
