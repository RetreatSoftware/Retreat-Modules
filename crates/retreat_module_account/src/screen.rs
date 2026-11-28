//! What this machine's player sees: which screen (pure [`account_screen`]), its panels and text
//! lines, and the local commands of the character list (picking is local; Play and Delete go
//! through the routing rule like every account change).

use bevy::prelude::*;
use retreat_engine_api::{
    AppHudExt, AppUiExt, CommandArgs, CommandError, CommandOutcome, CommandResult, HudAnchor,
    HudKind, HudWidget, NetLocalClient, PanelAnchor, PanelDirection, UiButtonEntry, UiHud, UiPanel,
    UiPanels, UiTextField,
};
use retreat_gameplay_api::{CharacterId, SoloPlayer, local_player_id};

use crate::commands::{delete_listed_character, play_character, send_to_server};
use crate::config::AccountConfig;
use crate::session::{AccountSession, session_of};
use retreat_gameplay_api::{Route, route};

const LOGIN_PANEL: &str = "account_login";
const CHARACTERS_PANEL: &str = "account_characters";
const PLAYING_PANEL: &str = "account_playing";
/// Sorts before the list lines (widgets at one anchor stack in id order).
const TITLE_WIDGET: &str = "account_header";
const NOTICE_WIDGET: &str = "account_notice";
/// The list lines are `account_line_0` .. one per allowed character.
const LINE_WIDGET_PREFIX: &str = "account_line_";

const LOGIN_FIELDS: &[UiTextField] = &[UiTextField {
    id: "name",
    placeholder: "Your name",
    max_chars: 32,
    submit_command: "account.login",
    clear_on_submit: false,
}];
const LOGIN_ENTRIES: &[UiButtonEntry] = &[UiButtonEntry {
    id: "log_in",
    label: "Log in",
    command_line: "ui.submit_field account_login name",
}];
const CHARACTER_FIELDS: &[UiTextField] = &[UiTextField {
    id: "new_name",
    placeholder: "New character name",
    max_chars: 32,
    submit_command: "account.create",
    clear_on_submit: true,
}];
const CHARACTER_ENTRIES: &[UiButtonEntry] = &[
    UiButtonEntry {
        id: "create",
        label: "Create character",
        command_line: "ui.submit_field account_characters new_name",
    },
    UiButtonEntry {
        id: "previous",
        label: "Previous",
        command_line: "account.pick_previous",
    },
    UiButtonEntry {
        id: "next",
        label: "Next",
        command_line: "account.pick_next",
    },
    UiButtonEntry {
        id: "play",
        label: "Play",
        command_line: "account.play_picked",
    },
    UiButtonEntry {
        id: "delete",
        label: "Delete",
        command_line: "account.delete_picked",
    },
];
const PLAYING_ENTRIES: &[UiButtonEntry] = &[UiButtonEntry {
    id: "log_out",
    label: "Log out",
    command_line: "account.logout",
}];

/// Which screen this machine's player sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountScreen {
    /// Type a name and log in.
    Login,
    /// The login name went to the server; waiting for the session.
    Joining,
    /// The character list.
    Characters,
    /// Playing a character.
    Playing,
}

impl AccountScreen {
    fn word(self) -> &'static str {
        match self {
            AccountScreen::Login => "login",
            AccountScreen::Joining => "joining",
            AccountScreen::Characters => "characters",
            AccountScreen::Playing => "playing",
        }
    }
}

/// The screen for this machine's `session` (none before the server opened it) when a login name
/// was or was not sent yet. Pure.
pub fn account_screen(session: Option<&AccountSession>, login_sent: bool) -> AccountScreen {
    match session {
        Some(session) if session.playing.is_some() => AccountScreen::Playing,
        Some(_) => AccountScreen::Characters,
        None if login_sent => AccountScreen::Joining,
        None => AccountScreen::Login,
    }
}

/// This machine's local list state (not shared, not simulated).
#[derive(Resource, Debug, Default)]
pub(crate) struct AccountPicker {
    /// Index of the picked character in the session's list.
    pub picked: usize,
    /// The character a first press of Delete chose; a second press deletes it.
    pub delete_armed: Option<CharacterId>,
    /// The login name a network client sent.
    pub login_sent: Option<String>,
    /// What this machine says before the server answers (a bad name, "press Delete again").
    pub local_notice: String,
}

/// The ids of the list lines, made once.
#[derive(Resource, Debug, Clone)]
struct AccountLineIds(Vec<&'static str>);

/// Registers the panels, their fields and the text lines (one list line per allowed character).
pub(crate) fn register_screens(app: &mut App, config: &AccountConfig) {
    app.register_panel(UiPanel {
        id: LOGIN_PANEL,
        title: Some("Log in"),
        anchor: PanelAnchor::Center,
        direction: PanelDirection::Column,
        entries: LOGIN_ENTRIES,
        visible: false,
    })
    .register_text_fields(LOGIN_PANEL, LOGIN_FIELDS)
    .register_panel(UiPanel {
        id: CHARACTERS_PANEL,
        title: Some("Characters"),
        anchor: PanelAnchor::Center,
        direction: PanelDirection::Column,
        entries: CHARACTER_ENTRIES,
        visible: false,
    })
    .register_text_fields(CHARACTERS_PANEL, CHARACTER_FIELDS)
    .register_panel(UiPanel {
        id: PLAYING_PANEL,
        title: None,
        anchor: PanelAnchor::TopRight,
        direction: PanelDirection::Row,
        entries: PLAYING_ENTRIES,
        visible: false,
    });
    let text = |id: &'static str, anchor: HudAnchor, font_size: f32| HudWidget {
        id,
        anchor,
        kind: HudKind::Text { font_size },
        visible: false,
    };
    app.register_hud_widget(text(TITLE_WIDGET, HudAnchor::TopCenter, 24.0))
        .register_hud_widget(text(NOTICE_WIDGET, HudAnchor::BottomCenter, 20.0));
    // Widget ids are `&'static str`: each line's id is made (and leaked) once per app.
    let lines: Vec<&'static str> = (0..config.max_characters)
        .map(|line| &*Box::leak(format!("{LINE_WIDGET_PREFIX}{line}").into_boxed_str()))
        .collect();
    for id in &lines {
        app.register_hud_widget(text(id, HudAnchor::TopCenter, 20.0));
    }
    app.insert_resource(AccountLineIds(lines));
}

/// This machine's session, if the server opened one.
fn local_session(world: &mut World) -> Option<(Entity, AccountSession)> {
    let local = local_player_id(
        *world.resource::<NetLocalClient>(),
        *world.resource::<SoloPlayer>(),
    )?;
    let entity = session_of(world, local)?;
    Some((entity, world.get::<AccountSession>(entity)?.clone()))
}

fn local_screen(world: &mut World) -> (AccountScreen, Option<(Entity, AccountSession)>) {
    let session = local_session(world);
    let sent = world.resource::<AccountPicker>().login_sent.is_some();
    (
        account_screen(session.as_ref().map(|(_, s)| s), sent),
        session,
    )
}

/// Shows or hides `panel` only when that changes (a change rebuilds the UI).
fn show_panel(world: &mut World, panel: &str, shown: bool) {
    if world.resource::<UiPanels>().is_visible(panel) != shown {
        let _ = world.resource_mut::<UiPanels>().set_visible(panel, shown);
    }
}

/// Shows or hides one entry only when that changes.
fn show_entry(world: &mut World, panel: &str, entry: &str, shown: bool) {
    let is_shown = world
        .resource::<UiPanels>()
        .visible_entries(panel)
        .iter()
        .any(|candidate| candidate.id == entry);
    if is_shown != shown {
        let mut panels = world.resource_mut::<UiPanels>();
        let _ = if shown {
            panels.show_entry(panel, entry)
        } else {
            panels.hide_entry(panel, entry)
        };
    }
}

/// Sets a text widget only when its text or visibility changes.
fn set_line(world: &mut World, id: &str, text: &str) {
    let visible = !text.is_empty();
    let unchanged = world
        .resource::<UiHud>()
        .widget(id)
        .is_some_and(|state| state.text == text && state.visible == visible);
    if !unchanged {
        let mut hud = world.resource_mut::<UiHud>();
        let _ = hud.set_text(id, text);
        let _ = hud.set_visible(id, visible);
    }
}

/// Keeps the panels and lines in step with this machine's screen (every frame; changes only when
/// something differs).
pub(crate) fn show_screen(world: &mut World) {
    let (screen, session) = local_screen(world);
    show_panel(world, LOGIN_PANEL, screen == AccountScreen::Login);
    show_panel(world, CHARACTERS_PANEL, screen == AccountScreen::Characters);
    show_panel(world, PLAYING_PANEL, screen == AccountScreen::Playing);
    let lines = world.resource::<AccountLineIds>().0.clone();
    let config = world.resource::<AccountConfig>().clone();
    let picker = world.resource::<AccountPicker>();
    let local_notice = picker.local_notice.clone();
    let login_sent = picker.login_sent.clone().unwrap_or_default();
    let count = session.as_ref().map_or(0, |(_, s)| s.characters.len());
    let picked = picker.picked.min(count.saturating_sub(1));
    let (title, notice) = match (&screen, &session) {
        (AccountScreen::Login, _) => (
            "Log in with a name (no password: development login)".to_string(),
            local_notice,
        ),
        (AccountScreen::Joining, _) => (format!("Logging in as {login_sent}..."), local_notice),
        (AccountScreen::Characters, Some((_, session))) => (
            format!(
                "{}: {} of {} characters in world {}",
                session.account.0,
                session.characters.len(),
                config.max_characters,
                session.world
            ),
            if local_notice.is_empty() {
                session.notice.clone()
            } else {
                local_notice
            },
        ),
        (AccountScreen::Playing, Some((_, session))) => {
            let name = session
                .playing
                .as_ref()
                .and_then(|id| session.character(id))
                .map_or("?", |character| character.name.as_str());
            (
                format!("{name} ({})", session.account.0),
                session.notice.clone(),
            )
        }
        _ => (String::new(), String::new()),
    };
    set_line(world, TITLE_WIDGET, &title);
    set_line(world, NOTICE_WIDGET, &notice);
    let listing = screen == AccountScreen::Characters;
    for (index, id) in lines.iter().enumerate() {
        let text = match (&session, listing) {
            (Some((_, session)), true) => session
                .characters
                .get(index)
                .map(|character| {
                    let marker = if index == picked { ">" } else { " " };
                    format!("{marker} {}", character.name)
                })
                .unwrap_or_default(),
            _ => String::new(),
        };
        set_line(world, id, &text);
    }
    // Buttons that need a character, or a choice between two, are not shown without one.
    show_entry(world, CHARACTERS_PANEL, "play", count > 0);
    show_entry(world, CHARACTERS_PANEL, "delete", count > 0);
    show_entry(world, CHARACTERS_PANEL, "previous", count > 1);
    show_entry(world, CHARACTERS_PANEL, "next", count > 1);
    show_entry(
        world,
        CHARACTERS_PANEL,
        "create",
        count < config.max_characters,
    );
}

/// The local session and the picked character, or an error for the player.
fn picked_character(
    world: &mut World,
    args: &CommandArgs,
) -> Result<(Entity, AccountSession, CharacterId), CommandError> {
    let Some((entity, session)) = local_session(world) else {
        return Err(args.failure("nobody is logged in on this machine"));
    };
    let picked = world.resource::<AccountPicker>().picked;
    let Some(character) = session
        .characters
        .get(picked.min(session.characters.len().saturating_sub(1)))
    else {
        return Err(args.failure("this account has no character yet; create one first"));
    };
    let id = character.id.clone();
    Ok((entity, session, id))
}

fn move_pick(world: &mut World, args: &CommandArgs, step: isize) -> CommandResult {
    args.expect_count(0)?;
    let count = local_session(world).map_or(0, |(_, session)| session.characters.len());
    if count == 0 {
        return Err(args.failure("this account has no character yet; create one first"));
    }
    let mut picker = world.resource_mut::<AccountPicker>();
    let current = picker.picked.min(count - 1) as isize;
    picker.picked = (current + step).rem_euclid(count as isize) as usize;
    picker.delete_armed = None;
    picker.local_notice.clear();
    Ok(CommandOutcome::Continue)
}

/// `account.pick_next`: picks the next character of the list (local).
pub(crate) fn pick_next_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    move_pick(world, args, 1)
}

/// `account.pick_previous`: picks the previous character of the list (local).
pub(crate) fn pick_previous_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    move_pick(world, args, -1)
}

/// `account.play_picked`: plays the picked character (sent to the server on a client).
pub(crate) fn play_picked_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let (entity, session, character) = picked_character(world, args)?;
    world.resource_mut::<AccountPicker>().delete_armed = None;
    match route(world) {
        Route::SendToServer => send_to_server(world, format!("account.play {}", character.0)),
        Route::Ignore => {}
        Route::Decide => play_character(world, session.player, entity, &character)
            .map_err(|why| args.failure(why))?,
    }
    Ok(CommandOutcome::Continue)
}

/// `account.delete_picked`: the first press asks, a second press of the same character deletes it
/// (sent to the server on a client).
pub(crate) fn delete_picked_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(0)?;
    let (entity, session, character) = picked_character(world, args)?;
    let armed = world.resource::<AccountPicker>().delete_armed.clone();
    if armed.as_ref() != Some(&character) {
        let name = session
            .character(&character)
            .map_or(character.0.clone(), |listed| listed.name.clone());
        let mut picker = world.resource_mut::<AccountPicker>();
        picker.delete_armed = Some(character);
        picker.local_notice = format!("Press Delete again to delete {name} for ever");
        return Ok(CommandOutcome::Continue);
    }
    {
        let mut picker = world.resource_mut::<AccountPicker>();
        picker.delete_armed = None;
        picker.local_notice.clear();
        picker.picked = 0;
    }
    match route(world) {
        Route::SendToServer => send_to_server(world, format!("account.delete {}", character.0)),
        Route::Ignore => {}
        Route::Decide => {
            delete_listed_character(world, entity, &character).map_err(|why| args.failure(why))?
        }
    }
    Ok(CommandOutcome::Continue)
}

/// `assert.account_screen <login|joining|characters|playing>`: what this machine shows.
pub(crate) fn assert_screen_command(world: &mut World, args: &CommandArgs) -> CommandResult {
    args.expect_count(1)?;
    let expected: String = args.parse(0, "screen")?;
    let (screen, _) = local_screen(world);
    if screen.word() == expected {
        Ok(CommandOutcome::Continue)
    } else {
        Err(args.failure(format!(
            "this machine shows the {} screen, expected {expected}",
            screen.word()
        )))
    }
}
