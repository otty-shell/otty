mod blocks;
mod dashboard;
pub(crate) mod found_report;
pub(crate) mod left_panel;
mod limits;
pub(crate) mod main_area;
pub(crate) mod menus;
mod money;
mod profile_column;
pub(crate) mod project;
mod projects_section;
pub(crate) mod remove_dialog;
mod sources;
pub(crate) mod stats_dialog;
pub(crate) mod status_bar;
mod tokens;
mod top;

#[cfg(feature = "test-ids")]
pub(crate) use projects_section::NEW_SESSION_BUTTON_ID;
