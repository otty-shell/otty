//! The frame: the part of the window that composes several components.
//!
//! The chrome (the title bar and the status bar) and the sidebar (the side
//! panels around the main area) hold their own state and implement
//! [`otty_app_slots::Component`], like the features. They stay in the binary
//! because each one draws parts of several features. `view.rs` and
//! `screens/` stay outside this folder: they hold no state and only compose
//! the window from the frame and the features.

pub(crate) mod chrome;
pub(crate) mod sidebar;
