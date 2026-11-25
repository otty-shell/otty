//! How a failed count of the tokens is written, for the usage block and
//! the money of a profile or an account.

use otty_agents::SourceError;
use otty_app_i18n::{self as i18n, Key};

use super::super::failure_text::source_error_text;

/// Why the latest count of the tokens failed: "Could not count the
/// tokens: …".
pub(crate) fn count_failed_text(error: &SourceError) -> String {
    i18n::with_error(Key::TplAgentsTokensFailed, &source_error_text(error))
}
