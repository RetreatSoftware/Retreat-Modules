//! The HUD with only Bevy and the contract crates. The picture is checked in the game
//! (Retreat-Arena).

use bevy::prelude::*;
use retreat_engine_api::*;
use retreat_gameplay_api::*;
use retreat_module_combat_hud::*;
use std::time::Duration;

fn weapon(ammo: u32) -> WeaponStatus {
    WeaponStatus {
        kind: WeaponKind::Rifle,
        ammo,
        magazine: 30,
        reload_ticks_left: 0,
        hits_landed: 0,
    }
}

fn carrying(list: impl IntoIterator<Item = WeaponStatus>) -> Weapons {
    let mut weapons = Weapons::default();
    for status in list {
        weapons.put(status);
    }
    weapons
}

fn input() -> HudInput {
    HudInput {
        health: Some(Health::full(100)),
        weapons: Some(carrying([weapon(12)])),
        respawn: None,
        hit: false,
        timestep: Duration::from_secs_f64(1.0 / 64.0),
    }
}

#[test]
fn a_living_armed_player_sees_crosshair_ammo_and_health() {
    let view = hud_view(input());
    assert!(view.crosshair && !view.hit && view.respawn.is_none());
    assert_eq!(view.ammo, [(WeaponKind::Rifle, "Rifle 12/30".to_string())]);
    assert_eq!(view.health.as_deref(), Some("Health 100"));
    let reloading = hud_view(HudInput {
        weapons: Some(carrying([WeaponStatus {
            reload_ticks_left: 5,
            ..weapon(0)
        }])),
        ..input()
    });
    assert_eq!(reloading.ammo[0].1, "Rifle reloading");
}

#[test]
fn the_dead_see_the_countdown_instead_of_the_weapon() {
    let mut health = Health::full(100);
    health.apply_damage(100);
    let seconds = |ticks_left| {
        hud_view(HudInput {
            health: Some(health),
            respawn: Some(RespawnTimer { ticks_left }),
            ..input()
        })
    };
    let view = seconds(192);
    assert!(!view.crosshair && view.ammo.is_empty());
    assert_eq!(view.respawn.as_deref(), Some("Respawn in 3"));
    assert_eq!(seconds(129).respawn.as_deref(), Some("Respawn in 3"));
    assert_eq!(seconds(1).respawn.as_deref(), Some("Respawn in 1"));
}

#[test]
fn every_carried_weapon_is_listed_with_its_ammo() {
    let rockets = WeaponStatus {
        kind: WeaponKind::Launcher,
        magazine: 4,
        ..weapon(3)
    };
    let view = hud_view(HudInput {
        weapons: Some(carrying([rockets, weapon(12)])),
        ..input()
    });
    let lines = |view: &HudView| -> Vec<String> {
        view.ammo.iter().map(|(_, text)| text.clone()).collect()
    };
    assert_eq!(lines(&view), ["Rifle 12/30", "Rockets 3/4"]);
    let only_rockets = hud_view(HudInput {
        weapons: Some(carrying([rockets])),
        ..input()
    });
    assert_eq!(lines(&only_rockets), ["Rockets 3/4"]);
    let none = hud_view(HudInput {
        weapons: Some(Weapons::default()),
        ..input()
    });
    assert!(!none.crosshair && none.ammo.is_empty());
}

#[test]
fn each_element_needs_its_module() {
    let unarmed = hud_view(HudInput {
        weapons: None,
        hit: true,
        ..input()
    });
    assert!(!unarmed.crosshair && !unarmed.hit && unarmed.ammo.is_empty());
    assert!(unarmed.health.is_some());
    let no_health = hud_view(HudInput {
        health: None,
        ..input()
    });
    assert!(no_health.crosshair && no_health.health.is_none());
    assert_eq!(hud_view(HudInput::default()), HudView::default());
}

#[test]
fn the_widgets_follow_the_local_player_and_the_hit_marker_ends() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .add_plugins(CombatHudModulePlugin);
    let player = app
        .world_mut()
        .spawn((PlayerId(0), Health::full(100), carrying([weapon(30)])))
        .id();
    let frame = |app: &mut App| {
        app.world_mut().run_schedule(FixedUpdate);
        app.world_mut().run_schedule(Update);
    };
    let shown = |app: &mut App, id: &str, expected: bool| {
        let line = format!("assert.hud_shown {id} {expected}");
        run_command_line(app.world_mut(), &line).unwrap_or_else(|error| panic!("{error}"));
    };
    frame(&mut app);
    shown(&mut app, HEALTH_WIDGET, false);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    frame(&mut app);
    shown(&mut app, HEALTH_WIDGET, true);
    shown(&mut app, CROSSHAIR_WIDGET, true);
    shown(&mut app, ammo_widget(WeaponKind::Rifle), true);
    shown(&mut app, ammo_widget(WeaponKind::Launcher), false);
    shown(&mut app, HIT_WIDGET, false);
    hit(&mut app, player, 1);
    frame(&mut app);
    shown(&mut app, HIT_WIDGET, true);
    for _ in 0..20 {
        frame(&mut app);
    }
    shown(&mut app, HIT_WIDGET, false);
    // A slow frame runs more ticks than the marker lasts: it is still drawn once.
    hit(&mut app, player, 2);
    for _ in 0..40 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    app.world_mut().run_schedule(Update);
    shown(&mut app, HIT_WIDGET, true);
    frame(&mut app);
    shown(&mut app, HIT_WIDGET, false);
}

fn hit(app: &mut App, player: Entity, hits: u32) {
    let mut weapons = app.world_mut().get_mut::<Weapons>(player).unwrap();
    weapons.of_mut(WeaponKind::Rifle).unwrap().hits_landed = hits;
}
