use super::model::SettingsData;
pub(crate) use super::model::is_valid_hex_color;
use super::state::SettingsState;
use super::storage::{SettingsLoadStatus, load_settings};

/// Return the human-readable label for a palette entry by index.
pub(crate) fn palette_label(index: usize) -> Option<&'static str> {
    PALETTE_LABELS.get(index).copied()
}

const PALETTE_LABELS: [&str; 29] = [
    "Foreground",
    "Background",
    "Black",
    "Red",
    "Green",
    "Yellow",
    "Blue",
    "Magenta",
    "Cyan",
    "White",
    "Bright Black",
    "Bright Red",
    "Bright Green",
    "Bright Yellow",
    "Bright Blue",
    "Bright Magenta",
    "Bright Cyan",
    "Bright White",
    "Bright Foreground",
    "Dim Black",
    "Dim Red",
    "Dim Green",
    "Dim Yellow",
    "Dim Blue",
    "Dim Magenta",
    "Dim Cyan",
    "Dim White",
    "Dim Foreground",
    "Overlay",
];

/// Load settings state synchronously from persistent storage.
pub(crate) fn load_initial_settings_state() -> SettingsState {
    let data = match load_settings() {
        Ok(load) => {
            let (data, status) = load.into_parts();
            if let SettingsLoadStatus::Invalid(message) = status {
                log::warn!("settings file invalid: {message}");
            }
            data
        },
        Err(err) => {
            log::warn!("settings read failed: {err}");
            SettingsData::default()
        },
    };
    SettingsState::from_settings(data)
}
