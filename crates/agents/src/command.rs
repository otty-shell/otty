//! The command that starts an agent in a terminal tab.

/// A program with its arguments and environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommand {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl AgentCommand {
    /// The program to run: a name to look up on `PATH`, or a path.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The arguments of the program.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The environment variables to set, as names and values.
    pub fn env(&self) -> &[(String, String)] {
        &self.env
    }

    /// A command that runs `program` with no arguments and no variables.
    ///
    /// ```
    /// use otty_agents::AgentCommand;
    ///
    /// let command = AgentCommand::new("codex")
    ///     .with_arg("resume")
    ///     .with_arg("a1")
    ///     .with_env("CODEX_HOME", "/home/user/.codex-work");
    ///
    /// assert_eq!(command.program(), "codex");
    /// assert_eq!(command.args(), ["resume", "a1"]);
    /// ```
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    /// The same command with one more argument.
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// The same command with one more environment variable.
    pub fn with_env(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.env.push((name.into(), value.into()));
        self
    }
}
