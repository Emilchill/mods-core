//! Mods Core for Cinnabar: one list of your mods. F10 opens it; each mod shows its name,
//! version and description, and a click opens that mod's own menu.
//!
//! Mods join the list through `mods_core.rs`, the protocol file mod creators copy.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

pub mod list;
pub mod mods_core;
mod surface_controls;
pub mod view;

#[cfg(target_arch = "wasm32")]
mod guest;

/// The panel, embedded in the component as its panel bundle.
pub fn menu_panel() -> String {
    view::full()
}

/// The bundle route of the panel's surface.
pub const MENU_ROUTE: &str = view::SCREEN;
