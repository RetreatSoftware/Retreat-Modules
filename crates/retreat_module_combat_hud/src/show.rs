//! Putting the views on screen.

use crate::config::CombatHudConfig;
use crate::looks::readable;
use crate::match_view::{KillFeed, MatchInput, PlayerLine, kill_line, match_view};
use crate::view::{HudInput, hud_view};
use crate::widgets::{
    CLOCK_WIDGET, CROSSHAIR_WIDGET, HEALTH_WIDGET, HIT_WIDGET, HudLines, RESPAWN_WIDGET,
    RESULT_WIDGET, SCORE_HEAD_WIDGET, ammo_widget,
};
use bevy::prelude::*;
use retreat_engine_api::{
    EffectTime, HudStyle, InputActions, NetLocalClient, NetReplaying, RetiredEntity, SoundEvents,
    UiHud,
};
use retreat_gameplay_api::{
    Health, KillLog, MatchState, PlayerId, PlayerRoster, RespawnTimer, Scoreboard, SoloPlayer,
    WeaponKind, Weapons, local_player_id, ticks_from_seconds,
};

/// This machine's running hit marker. Not shared state.
#[derive(Resource, Debug, Default)]
pub(crate) struct HitMarker {
    /// The hit count last seen; `None` before the player has a weapon.
    seen: Option<u32>,
    ticks_left: u32,
    /// Started and not yet on screen: a frame can cover more ticks than the marker lasts, and it
    /// must still be seen for one frame.
    unshown: bool,
}

/// This machine's kill feed.
#[derive(Resource, Debug, Default)]
pub(crate) struct ShownKills(pub KillFeed);

/// Switched on by `hud.scoreboard on`, as the held button is.
#[derive(Resource, Debug, Default)]
pub(crate) struct ScoreboardSwitch(pub bool);

/// Starts the marker when the server counts a new hit. In ticks, so a script sees the same thing.
pub(crate) fn time_hit_marker(
    players: Query<(&PlayerId, &Weapons), Without<RetiredEntity>>,
    (client, solo): (Res<NetLocalClient>, Res<SoloPlayer>),
    replaying: Res<NetReplaying>,
    config: Res<CombatHudConfig>,
    time: Res<Time<Fixed>>,
    mut marker: ResMut<HitMarker>,
    mut sounds: ResMut<SoundEvents>,
) {
    if replaying.0 {
        return;
    }
    let local = local_player_id(*client, *solo);
    let hits = players
        .iter()
        .find(|(player, _)| Some(**player) == local)
        .map(|(_, weapons)| weapons.iter().map(|weapon| weapon.hits_landed).sum::<u32>());
    marker.ticks_left = marker.ticks_left.saturating_sub(1);
    if let (Some(before), Some(now)) = (marker.seen, hits)
        && now > before
    {
        marker.ticks_left = ticks_from_seconds(config.hit_marker_seconds, time.timestep()).max(1);
        marker.unshown = true;
        if let Some(cue) = &config.hit_sound {
            sounds.play(cue.event(format!("hud/hit/{now}"), None, EffectTime::Present));
        }
    }
    marker.seen = hits;
}

/// Ages the kill feed and adds the deaths the match logged.
pub(crate) fn time_kill_feed(
    logs: Query<&KillLog, Without<RetiredEntity>>,
    replaying: Res<NetReplaying>,
    config: Res<CombatHudConfig>,
    time: Res<Time<Fixed>>,
    mut kills: ResMut<ShownKills>,
) {
    if replaying.0 {
        return;
    }
    let lifetime = ticks_from_seconds(config.kill_feed_seconds, time.timestep()).max(1);
    kills.0.tick(logs.iter().next(), lifetime);
}

type Players<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerId,
        Option<&'static Health>,
        Option<&'static Weapons>,
        Option<&'static RespawnTimer>,
    ),
    Without<RetiredEntity>,
>;

/// Everything the HUD reads.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Sources<'w, 's> {
    players: Players<'w, 's>,
    matches:
        Query<'w, 's, (&'static MatchState, Option<&'static Scoreboard>), Without<RetiredEntity>>,
    client: Res<'w, NetLocalClient>,
    solo: Res<'w, SoloPlayer>,
    roster: Res<'w, PlayerRoster>,
    actions: Option<Res<'w, InputActions>>,
    switch: Res<'w, ScoreboardSwitch>,
    kills: Res<'w, ShownKills>,
    config: Res<'w, CombatHudConfig>,
    lines: Res<'w, HudLines>,
    time: Res<'w, Time<Fixed>>,
}

pub(crate) fn show_hud(sources: Sources, mut marker: ResMut<HitMarker>, mut hud: ResMut<UiHud>) {
    let local = local_player_id(*sources.client, *sources.solo);
    // A server has nobody to show anything to.
    let playing = local.and_then(|local| {
        sources
            .players
            .iter()
            .find(|(player, ..)| **player == local)
    });
    let current = local.and_then(|_| sources.matches.iter().next());
    let held = sources
        .actions
        .as_ref()
        .is_some_and(|actions| actions.is_held("hud_scoreboard"));
    let table = match_view(
        &MatchInput {
            state: current.map(|(state, _)| *state),
            board: current.and_then(|(_, board)| board.cloned()),
            own: local,
            scoreboard_wanted: held || sources.switch.0,
            timestep: sources.time.timestep(),
        },
        &sources.roster,
    );
    let view = hud_view(
        playing.map_or_else(HudInput::default, |(_, health, weapons, respawn)| {
            HudInput {
                health: health.copied(),
                weapons: weapons.cloned(),
                respawn: respawn.copied(),
                hit: marker.ticks_left > 0 || marker.unshown,
                covered: table.score_head.is_some(),
                timestep: sources.time.timestep(),
            }
        }),
    );
    marker.unshown = false;
    // Every text has the panel; a line about a player has the player's color.
    let panel = Some(sources.config.panel);
    let plain = |text: Option<String>| {
        text.map(|text| {
            let style = HudStyle { panel, ..default() };
            (text, style)
        })
    };
    let colored = |line: Option<PlayerLine>| {
        line.map(|line| {
            let color = line.player.map_or(Color::WHITE, |player| {
                readable(sources.roster.color(player))
            });
            (line.text, HudStyle { color, panel })
        })
    };
    // The hit marker sits on the crosshair: no box over what is aimed at.
    let hit = view.hit.then(|| {
        let text = sources.config.hit_marker_text.clone();
        (text, HudStyle::default())
    });
    let mut texts = vec![
        (HIT_WIDGET, hit),
        (HEALTH_WIDGET, plain(view.health)),
        (RESPAWN_WIDGET, plain(view.respawn)),
        (CLOCK_WIDGET, plain(table.clock)),
        (RESULT_WIDGET, colored(table.result)),
        (SCORE_HEAD_WIDGET, plain(table.score_head)),
    ];
    texts.extend(WeaponKind::ALL.map(|kind| {
        let line = view.ammo.iter().find(|(found, _)| *found == kind);
        (ammo_widget(kind), plain(line.map(|(_, text)| text.clone())))
    }));
    let mut rows = table.score_rows.into_iter();
    let score_rows = sources.lines.score_rows.iter();
    texts.extend(score_rows.map(|id| (*id, colored(rows.next()))));
    let mut kills = sources
        .kills
        .0
        .latest(sources.lines.kills.len())
        .map(|entry| kill_line(entry, &sources.roster))
        .filter(|_| local.is_some());
    let kill_lines = sources.lines.kills.iter();
    texts.extend(kill_lines.map(|id| (*id, colored(kills.next()))));
    for (id, line) in texts {
        show(&mut hud, id, line.is_some());
        if let Some((text, style)) = line {
            if !hud.has_text(id, &text) {
                hud.set_text(id, text).expect("registered by this module");
            }
            if !hud.has_style(id, style) {
                hud.set_style(id, style).expect("registered by this module");
            }
        }
    }
    show(&mut hud, CROSSHAIR_WIDGET, view.crosshair);
}

/// Changes visibility only when it differs, so the HUD is not rebuilt every frame.
fn show(hud: &mut ResMut<UiHud>, id: &str, visible: bool) {
    if hud.widget(id).is_some_and(|state| state.visible != visible) {
        hud.set_visible(id, visible)
            .expect("registered by this module");
    }
}
