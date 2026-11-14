//! The settings a new terminal starts with: the colors of the theme, the
//! terminal font and the session it runs. The terminal feature gets them as
//! a value in `TerminalIntent::OpenTab` and starts the session itself.
//!
//! Building them reads nothing from disk or the environment: the theme and
//! the fonts are in the app, the shell session was built at start (see
//! `otty-app-shell`), and the editor is the saved one.

use std::path::Path;

use otty_app_theme::fonts::FontsConfig;
use otty_app_theme::theme::AppTheme;
use otty_ui_term::settings::{
    BlockSelectionMode, EngineSettings, FontSettings, InteractionSettings,
    LocalSessionOptions, SessionKind, Settings, ThemeSettings,
};

/// Build the settings new terminals start with from the theme and the
/// fonts, before a session is set.
pub(crate) fn base(theme: &AppTheme, fonts: &FontsConfig) -> Settings {
    let font = FontSettings {
        size: fonts.terminal().size(),
        font_type: fonts.terminal().font(),
        ..FontSettings::default()
    };
    let theme = ThemeSettings::new(Box::new(theme.terminal_palette()));

    Settings {
        font,
        theme,
        engine: EngineSettings::default(),
        interaction: InteractionSettings::default()
            .with_block_selection_mode(BlockSelectionMode::CommandOnly),
    }
}

/// Return `base` with the session it starts: a shell, a command or an SSH
/// connection.
pub(crate) fn for_session(base: &Settings, session: SessionKind) -> Settings {
    let mut settings = base.clone();
    settings.engine = settings.engine.clone().with_session(session);

    settings
}

/// Return `session` started in `folder` instead of its own folder: a shell
/// of Open in terminal in Files. An SSH session has no folder on this
/// computer, so it stays as it is.
pub(crate) fn session_in_folder(
    session: SessionKind,
    folder: &Path,
) -> SessionKind {
    match session {
        SessionKind::Local(options) => SessionKind::from_local_options(
            options.with_working_directory(folder.to_path_buf()),
        ),
        ssh => ssh,
    }
}

/// Return the settings of a terminal that opens `file_path` with `editor`,
/// a command line such as `nvim -u NORC`, in the file's folder. `None` when
/// the command line is empty or can not be read; that is logged.
pub(crate) fn for_editor(
    editor: &str,
    base: &Settings,
    file_path: &Path,
) -> Option<Settings> {
    let (program, mut args) = parse_command_line(editor)?;
    args.push(file_path.to_string_lossy().into_owned());

    let mut options = LocalSessionOptions::default()
        .with_program(&program)
        .with_args(args);
    if let Some(parent) = file_path.parent() {
        options = options.with_working_directory(parent.into());
    }

    Some(for_session(base, SessionKind::from_local_options(options)))
}

/// Split a command line into its program and arguments, the way a shell
/// splits words.
fn parse_command_line(input: &str) -> Option<(String, Vec<String>)> {
    let parts = match shell_words::split(input) {
        Ok(parts) => parts,
        Err(err) => {
            log::warn!("default editor parse failed: {err}");
            return None;
        },
    };
    let Some((program, args)) = parts.split_first() else {
        log::warn!("default editor command is empty");
        return None;
    };

    Some((program.clone(), args.to_vec()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_app_theme::theme::BuiltinTheme;

    use super::*;

    fn local_options(settings: &Settings) -> &LocalSessionOptions {
        match &settings.engine.session {
            SessionKind::Local(options) => options,
            SessionKind::Ssh(_) => panic!("expected a local session"),
        }
    }

    #[test]
    fn given_valid_command_line_when_parsed_then_program_and_args_are_returned()
    {
        let parsed =
            parse_command_line("nvim -u NORC").expect("command should parse");

        assert_eq!(parsed.0, "nvim");
        assert_eq!(parsed.1, vec![String::from("-u"), String::from("NORC")]);
    }

    #[test]
    fn given_invalid_command_line_when_parsed_then_none_is_returned() {
        assert!(parse_command_line("nvim '").is_none());
        assert!(parse_command_line("   ").is_none());
    }

    /// New terminals take the colors of the theme and the terminal font.
    #[test]
    fn new_terminals_take_the_theme_colors_and_the_terminal_font() {
        for theme in BuiltinTheme::ALL {
            let fonts = FontsConfig::default();

            let settings = base(&AppTheme::new(theme), &fonts);

            assert_eq!(
                settings.theme.color_pallete.background,
                theme.terminal_palette().background
            );
            assert_eq!(settings.font.size, fonts.terminal().size());
            assert_eq!(settings.font.font_type, fonts.terminal().font());
        }
    }

    /// A file opens in the editor, with the file after the editor's own
    /// arguments, in the file's folder.
    #[test]
    fn a_file_opens_in_the_editor_in_its_folder() {
        let base = base(
            &AppTheme::new(BuiltinTheme::OttyDark),
            &FontsConfig::default(),
        );

        let settings = for_editor(
            "nvim -u NORC",
            &base,
            Path::new("/tmp/project/notes.txt"),
        )
        .expect("the editor command parses");

        let options = local_options(&settings);
        assert_eq!(options.program(), "nvim");
        assert_eq!(options.args(), ["-u", "NORC", "/tmp/project/notes.txt"]);
        assert_eq!(
            options.working_directory(),
            &Some(PathBuf::from("/tmp/project"))
        );
    }

    /// A shell opened in a folder keeps its program and arguments and
    /// starts in that folder.
    #[test]
    fn a_shell_opened_in_a_folder_starts_there() {
        let shell = LocalSessionOptions::default()
            .with_program("/bin/zsh")
            .with_args(vec![String::from("-l")])
            .with_working_directory(PathBuf::from("/home/user"));

        let session = session_in_folder(
            SessionKind::from_local_options(shell),
            Path::new("/tmp/project/src"),
        );

        let SessionKind::Local(options) = session else {
            panic!("expected a local session");
        };
        assert_eq!(options.program(), "/bin/zsh");
        assert_eq!(options.args(), ["-l"]);
        assert_eq!(
            options.working_directory(),
            &Some(PathBuf::from("/tmp/project/src"))
        );
    }
}
