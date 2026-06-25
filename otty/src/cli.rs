use clap::Parser;

/// Command-line options supported by the OTTY executable.
#[derive(Debug, Parser)]
#[command(version, about = env!("CARGO_PKG_DESCRIPTION"))]
pub(crate) struct Cli {}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Cli;

    #[test]
    fn given_no_flags_when_parsed_then_command_line_is_accepted() {
        let result = Cli::try_parse_from(["otty"]);

        assert!(result.is_ok());
    }

    #[test]
    fn given_removed_loader_check_flag_when_parsed_then_error_is_returned() {
        let result = Cli::try_parse_from(["otty", "--check-loader"]);

        assert!(result.is_err());
    }

    #[test]
    fn given_version_flag_when_parsed_then_version_is_displayed() {
        let error = Cli::try_parse_from(["otty", "--version"])
            .expect_err("version flag should stop normal startup");

        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayVersion);
        assert!(error.to_string().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn given_unknown_flag_when_parsed_then_error_is_returned() {
        let result = Cli::try_parse_from(["otty", "--unknown"]);

        assert!(result.is_err());
    }
}
