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
        covered: false,
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

const TICK: Duration = Duration::from_nanos(15_625_000);

fn names() -> PlayerNames {
    PlayerNames(vec!["Red".to_string(), "Blue".to_string()])
}

fn board() -> Scoreboard {
    let mut board = Scoreboard::default();
    board.entry_mut(PlayerId(0)).deaths = 2;
    let blue = board.entry_mut(PlayerId(1));
    (blue.points, blue.kills) = (2, 2);
    board
}

fn match_input(phase: MatchPhase) -> MatchInput {
    MatchInput {
        state: Some(MatchState { phase }),
        board: Some(board()),
        own: Some(PlayerId(0)),
        scoreboard_wanted: false,
        timestep: TICK,
    }
}

fn words(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn a_running_match_shows_its_clock_and_the_scoreboard_only_when_asked() {
    let running = match_input(MatchPhase::Running {
        ticks_left: 64 * 125 - 1,
    });
    let view = match_view(&running, &names());
    assert_eq!(view.clock.as_deref(), Some("2:05"));
    assert!(view.result.is_none() && view.score_head.is_none() && view.score_rows.is_empty());
    let asked = match_view(
        &MatchInput {
            scoreboard_wanted: true,
            ..running
        },
        &names(),
    );
    assert_eq!(
        words(asked.score_head.as_deref().unwrap()),
        "Player Pts K D"
    );
    let rows: Vec<String> = asked.score_rows.iter().map(|row| words(row)).collect();
    assert_eq!(rows, ["Blue 2 2 0", "> Red 0 0 2"]);
    let widths: Vec<usize> = asked
        .score_rows
        .iter()
        .chain(&asked.score_head)
        .map(String::len)
        .collect();
    assert!(widths.iter().all(|width| *width == widths[0]), "{widths:?}");
}

#[test]
fn a_finished_match_shows_the_winner_the_scores_and_the_countdown() {
    let view = match_view(
        &match_input(MatchPhase::Finished {
            winner: Some(PlayerId(1)),
            ticks_until_restart: 65,
        }),
        &names(),
    );
    assert_eq!(view.result.as_deref(), Some("Blue wins"));
    assert_eq!(view.clock.as_deref(), Some("Next match in 2"));
    assert_eq!(view.score_rows.len(), 2);
    let draw = match_view(
        &match_input(MatchPhase::Finished {
            winner: None,
            ticks_until_restart: 1,
        }),
        &names(),
    );
    assert_eq!(draw.result.as_deref(), Some("Draw"));
    assert_eq!(
        match_view(&MatchInput::default(), &names()),
        MatchView::default()
    );
}

#[test]
fn the_scoreboard_clears_the_middle_of_the_screen() {
    let mut health = Health::full(100);
    health.apply_damage(100);
    let view = hud_view(HudInput {
        health: Some(health),
        respawn: Some(RespawnTimer { ticks_left: 10 }),
        hit: true,
        covered: true,
        ..input()
    });
    assert!(!view.crosshair && !view.hit && view.respawn.is_none());
    assert!(view.health.is_some());
}

#[test]
fn the_kill_feed_shows_new_deaths_for_a_while_and_skips_old_ones() {
    let mut log = KillLog::default();
    log.push(PlayerId(0), Some(PlayerId(1)));
    let mut feed = KillFeed::default();
    // Joining late: what happened before is not news.
    feed.tick(Some(&log), 3);
    assert_eq!(feed.latest(4).count(), 0);
    log.push(PlayerId(1), Some(PlayerId(1)));
    log.push(PlayerId(0), None);
    feed.tick(Some(&log), 3);
    let lines = |feed: &KillFeed, most| -> Vec<String> {
        feed.latest(most)
            .map(|entry| kill_line(entry, &names()))
            .collect()
    };
    assert_eq!(lines(&feed, 4), ["Blue blew up", "Red fell"]);
    assert_eq!(lines(&feed, 1), ["Red fell"]);
    feed.tick(Some(&log), 3);
    feed.tick(Some(&log), 3);
    assert_eq!(lines(&feed, 4).len(), 2);
    feed.tick(Some(&log), 3);
    assert!(lines(&feed, 4).is_empty());
    log.push(PlayerId(0), Some(PlayerId(1)));
    feed.tick(Some(&log), 3);
    assert_eq!(lines(&feed, 4), ["Blue killed Red"]);
    feed.tick(None, 3);
    assert!(lines(&feed, 4).is_empty());
}

#[test]
fn the_match_widgets_follow_the_match_and_the_scoreboard_switch() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CommandRegistry>()
        .init_resource::<InputActions>()
        .add_plugins(CombatHudModulePlugin);
    app.world_mut()
        .insert_resource(SoloPlayer(Some(PlayerId(0))));
    app.world_mut().spawn((
        MatchState {
            phase: MatchPhase::Running { ticks_left: 640 },
        },
        board(),
        KillLog::default(),
    ));
    let line = |app: &mut App, line: &str| {
        run_command_line(app.world_mut(), line).unwrap_or_else(|error| panic!("{error}"));
    };
    app.world_mut().run_schedule(Update);
    line(&mut app, "assert.hud_text hud_match_clock 0:10");
    line(&mut app, "assert.hud_shown hud_match_score_head false");
    line(&mut app, "hud.scoreboard on");
    app.world_mut().run_schedule(Update);
    line(
        &mut app,
        "assert.hud_text hud_match_score_row0 Player 1 2 2 0",
    );
    line(
        &mut app,
        "assert.hud_text hud_match_score_row1 > Player 0 0 0 2",
    );
    line(&mut app, "assert.hud_shown hud_match_score_row2 false");
    line(&mut app, "hud.scoreboard off");
    app.world_mut().run_schedule(Update);
    line(&mut app, "assert.hud_shown hud_match_score_row0 false");
}
