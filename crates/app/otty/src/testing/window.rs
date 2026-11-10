//! A new app for tests, built without reading files.

use otty_app_theme::theme::BuiltinTheme;

use crate::app::App;
use crate::modes::types::DEFAULT_PINNED;

/// A shell without OTTY shell integration, so building its session writes
/// no files.
const TEST_SHELL_PATH: &str = "/opt/otty-test/otty-test-shell";

/// A new app as on a first start: the default theme, Terminal mode and the
/// default pinned modes. It reads no settings or modes file.
pub(crate) fn app() -> App {
    App::with_saved(BuiltinTheme::default(), TEST_SHELL_PATH, &DEFAULT_PINNED)
}
