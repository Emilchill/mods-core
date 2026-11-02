//! Host glue: hears the mods' info cues, keeps the list, runs the panel and sends open requests.

use crate::list::List;
use crate::mods_core;
use crate::surface_controls::ROWS;
use crate::view;
use mod_api::bindings::{
    Guest,
    cinnabar::extension::{events, input, panel, touch},
};
use std::cell::RefCell;

struct Mod;

#[derive(Default)]
struct State {
    list: List,
    frame: u32,
    /// First mod row in view.
    top: usize,
    was_open: bool,
    /// The panel shows old values.
    dirty: bool,
    /// Mods Core's panel is away while another mod's menu is open (see `mods_core.rs`).
    away: bool,
    /// Frame the panel went away, and the last frame a mod said its menu was open.
    away_since: u32,
    last_busy: u32,
    /// The on-screen "Mods" button for touch controls is set.
    touch_set: bool,
    /// Mods Core's own panel is known to be set (the bundle may be ignored by a newer Cinnabar).
    panel_set: bool,
    /// An F10 press that Cinnabar's own `toggle_key` handling has not answered yet: (frame, panel_open then).
    f10_at: Option<(u32, bool)>,
}

/// The touch button that opens the list on phones and tablets (no F10 there).
const TOUCH_ID: &str = "modscore-open";

/// Frames to wait, after the panel went away and after the last "busy", before taking it back.
const BACK_AFTER: u32 = 12;

/// Frames after which a panel that cannot be taken back is given up on (and set again from scratch).
const GIVE_UP_AFTER: u32 = 180;

/// Frames to wait for Cinnabar to act on F10 itself before this mod opens or closes the panel.
const F10_WAIT: u32 = 3;

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

impl Guest for Mod {
    fn init() {
        STATE.with_borrow_mut(|state| state.dirty = true);
    }

    fn frame() {
        STATE.with_borrow_mut(State::frame);
    }
}

impl State {
    fn frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        let frame = self.frame;
        // Touch controls: a "Mods" button (players can move it in the touch layout editor).
        if !self.touch_set {
            let button = touch::Button {
                id: TOUCH_ID.into(),
                label: "Mods".into(),
                anchor: touch::Anchor::Right,
                offset: touch::Point { x: 6.0, y: -64.0 },
                size: touch::Point { x: 40.0, y: 22.0 },
            };
            self.touch_set = touch::set_buttons(&[button]).is_ok();
        }
        let tapped = touch::read_presses().is_ok_and(|taps| taps.iter().any(|id| id == TOUCH_ID));
        let mut open_requested = false;
        for cue in events::poll() {
            if let Some(chunk) = mods_core::read_chunk(&cue.name, &cue.values) {
                self.dirty |= self.list.take(&chunk, frame);
            } else if cue.name.starts_with(mods_core::OPEN_PREFIX) {
                open_requested = true;
            } else if cue.name.starts_with(mods_core::BUSY_PREFIX) {
                self.last_busy = frame;
            }
        }
        // Make sure the panel exists even if this Cinnabar ignored the embedded bundle.
        if !self.panel_set && !self.away {
            self.panel_set = panel::set_content(&view::full()).is_ok();
            self.dirty |= self.panel_set;
        }
        // A mod's menu is opening: step aside, so it is the only menu (see `mods_core.rs`).
        if open_requested && !self.away && panel::set_content("").is_ok() {
            self.away = true;
            self.away_since = frame;
            self.was_open = false;
        }
        if self.away {
            let quiet = frame.wrapping_sub(self.away_since) > BACK_AFTER && frame.wrapping_sub(self.last_busy) > BACK_AFTER;
            if quiet {
                if panel::set_content(&view::full()).is_ok() {
                    self.away = false;
                    self.dirty = true;
                } else if frame.wrapping_sub(self.away_since) > GIVE_UP_AFTER {
                    // Never stay stuck: start over and set the panel again.
                    self.away = false;
                    self.panel_set = false;
                }
            }
            return;
        }
        if frame % 30 == 0 {
            self.dirty |= self.list.forget(frame);
        }
        let Ok(controls) = input::read_selected_controls(&input::Selection {
            keys_pressed: Some(vec![view::TOGGLE_KEY.to_owned()]),
            keys_held: Some(Vec::new()),
            keys_admitted: Some(Vec::new()),
            events: true,
        }) else {
            return;
        };
        // F10 fallback: if Cinnabar did not open or close the panel itself, do it here.
        if controls.keys_pressed.iter().any(|k| k == view::TOGGLE_KEY) {
            self.f10_at = Some((frame, controls.panel_open));
        }
        if let Some((at, was_open)) = self.f10_at {
            if controls.panel_open != was_open {
                self.f10_at = None;
            } else if frame.wrapping_sub(at) >= F10_WAIT {
                let _ = panel::set_open(!was_open);
                self.f10_at = None;
            }
        }
        // A tap on the touch button opens or closes the list, as F10 does.
        if tapped {
            let _ = panel::set_open(!controls.panel_open);
        }
        let count = self.list.entries.len();
        let before = self.top;
        if controls.panel_open {
            if controls.wheel.is_finite() && controls.wheel > 0.0 {
                self.top = self.top.saturating_sub(1);
            } else if controls.wheel.is_finite() && controls.wheel < 0.0 {
                self.top += 1;
            }
        }
        for event in &controls.events {
            match event.id.as_str() {
                "up" => self.top = self.top.saturating_sub(ROWS),
                "down" => self.top += ROWS,
                id => {
                    let row = id.strip_prefix("row_").and_then(|n| n.parse::<usize>().ok());
                    let top = view::clamp_top(self.top, count);
                    if let Some(entry) = row.and_then(|r| self.list.entries.get(top + r)) {
                        let _ = events::emit(&mods_core::open_cue(&entry.slug), &[]);
                        // Step aside now, so the chosen menu is the only one when it opens.
                        if panel::set_content("").is_ok() {
                            self.away = true;
                            self.away_since = frame;
                            self.was_open = false;
                            return;
                        }
                    }
                }
            }
        }
        self.top = view::clamp_top(self.top, count);
        self.dirty |= self.top != before || (controls.panel_open && !self.was_open);
        self.was_open = controls.panel_open;
        // Values go only to an open panel; opening sends them.
        if !controls.panel_open {
            return;
        }
        if self.dirty && panel::update_content(&view::patch(&self.list.entries, self.top)).is_ok() {
            self.dirty = false;
        } else if self.dirty {
            // No panel yet (the bundle was not taken): send the whole one, values next frame.
            let _ = panel::set_content(&view::full());
        }
    }
}

mod_api::bindings::export!(Mod with_types_in mod_api::bindings);
