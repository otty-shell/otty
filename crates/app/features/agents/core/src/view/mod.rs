mod account_page;
pub(crate) mod add_account_dialog;
mod blocks;
mod chips;
mod entry_icon;
pub(crate) mod found_report;
pub(crate) mod key_dialog;
pub(crate) mod keyring_dialog;
pub(crate) mod left_panel;
mod limits;
pub(crate) mod main_area;
pub(crate) mod menus;
mod money;
mod needs;
mod overview;
mod profile_page;
pub(crate) mod project;
mod projects_section;
pub(crate) mod remove_dialog;
pub(crate) mod rename_dialog;
pub(crate) mod stats_dialog;
pub(crate) mod status_bar;
mod usage_block;

#[cfg(feature = "test-ids")]
pub(crate) use projects_section::NEW_SESSION_BUTTON_ID;
