//! Canned programs and hidden terminals, and what the plugin started and
//! sent to them. No program ever runs: a declared program answers with
//! what the test gave for it.

use std::collections::VecDeque;

use otty_plugin_manifest::Permissions;

use crate::HostError;
use crate::programs::Output;

/// The canned programs of one test, and the talks and terminals the
/// plugin started.
#[derive(Debug, Default)]
pub(super) struct Programs {
    runs: Vec<(Invocation, Output)>,
    talks: Vec<(Invocation, Vec<String>)>,
    terminals: Vec<(Invocation, Vec<String>)>,
    running_talks: Vec<VecDeque<String>>,
    running_terminals: Vec<Screens>,
    started: Vec<StartedProgram>,
    sent: Vec<String>,
}

impl Programs {
    /// The programs the plugin started, in order.
    pub(super) fn started(&self) -> &[StartedProgram] {
        &self.started
    }

    /// The lines sent to talks and the keys typed in terminals, in order.
    pub(super) fn sent(&self) -> &[String] {
        &self.sent
    }

    /// Every run of `program` with exactly `args` gives `output`.
    pub(super) fn answer_run(
        &mut self,
        program: &str,
        args: &[&str],
        output: Output,
    ) {
        self.runs.push((Invocation::new(program, args), output));
    }

    /// Every talk with `program` and exactly `args` writes `lines`, one
    /// for each `next_line`, and then ends.
    pub(super) fn answer_talk(
        &mut self,
        program: &str,
        args: &[&str],
        lines: &[&str],
    ) {
        self.talks
            .push((Invocation::new(program, args), to_strings(lines)));
    }

    /// Every hidden terminal of `program` with exactly `args` shows
    /// `screens`, one for each `wait_for_change`, and then stays still.
    pub(super) fn answer_terminal(
        &mut self,
        program: &str,
        args: &[&str],
        screens: &[&str],
    ) {
        self.terminals
            .push((Invocation::new(program, args), to_strings(screens)));
    }

    /// The canned output of `program` with `args`, after the checks OTTY
    /// makes.
    pub(super) fn run(
        &mut self,
        permissions: &Permissions,
        program: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Result<Output, HostError> {
        check(
            permissions,
            permissions.may_run(program, args),
            program,
            env,
        )?;
        self.note_start(program, args, env);

        canned(&self.runs, program, args).cloned()
    }

    /// Starts the canned talk of `program` with `args`, after the checks
    /// OTTY makes; its id.
    pub(super) fn start_talk(
        &mut self,
        permissions: &Permissions,
        program: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Result<usize, HostError> {
        check(
            permissions,
            permissions.may_run(program, args),
            program,
            env,
        )?;
        self.note_start(program, args, env);

        let lines = canned(&self.talks, program, args)?.clone();
        self.running_talks.push(lines.into());

        Ok(self.running_talks.len() - 1)
    }

    /// Notes `line` sent to the talk `id`.
    pub(super) fn send_line(
        &mut self,
        id: usize,
        line: &str,
    ) -> Result<(), HostError> {
        self.running_talks
            .get(id)
            .ok_or_else(|| unknown("talk", id))?;

        self.sent.push(line.to_string());

        Ok(())
    }

    /// The next canned line of the talk `id`; `None` once all were given.
    pub(super) fn next_line(
        &mut self,
        id: usize,
    ) -> Result<Option<String>, HostError> {
        let lines = self
            .running_talks
            .get_mut(id)
            .ok_or_else(|| unknown("talk", id))?;

        Ok(lines.pop_front())
    }

    /// Starts the canned hidden terminal of `program` with `args`, after
    /// the checks OTTY makes; its id.
    pub(super) fn start_terminal(
        &mut self,
        permissions: &Permissions,
        program: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Result<usize, HostError> {
        let may_start = permissions.may_start_terminal(program, args);
        check(permissions, may_start, program, env)?;
        self.note_start(program, args, env);

        let screens = canned(&self.terminals, program, args)?.clone();
        self.running_terminals.push(Screens {
            next: screens.into(),
            shown: String::new(),
        });

        Ok(self.running_terminals.len() - 1)
    }

    /// Notes `keys` typed in the terminal `id`.
    pub(super) fn send_keys(
        &mut self,
        id: usize,
        keys: &str,
    ) -> Result<(), HostError> {
        self.running_terminals
            .get(id)
            .ok_or_else(|| unknown("hidden terminal", id))?;

        self.sent.push(keys.to_string());

        Ok(())
    }

    /// The screen the terminal `id` shows now.
    pub(super) fn screen(&self, id: usize) -> Result<String, HostError> {
        let screens = self
            .running_terminals
            .get(id)
            .ok_or_else(|| unknown("hidden terminal", id))?;

        Ok(screens.shown.clone())
    }

    /// Moves the terminal `id` to its next canned screen; whether there
    /// was one.
    pub(super) fn wait_for_change(
        &mut self,
        id: usize,
    ) -> Result<bool, HostError> {
        let screens = self
            .running_terminals
            .get_mut(id)
            .ok_or_else(|| unknown("hidden terminal", id))?;

        let Some(next) = screens.next.pop_front() else {
            return Ok(false);
        };
        screens.shown = next;

        Ok(true)
    }

    fn note_start(
        &mut self,
        program: &str,
        args: &[String],
        env: &[(String, String)],
    ) {
        self.started.push(StartedProgram {
            program: program.to_string(),
            args: args.to_vec(),
            env: env.to_vec(),
        });
    }
}

/// A program the plugin started: run, talked to or in a hidden terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartedProgram {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl StartedProgram {
    /// The program.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Its arguments.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The variables the plugin set, as names and values.
    pub fn env(&self) -> &[(String, String)] {
        &self.env
    }
}

/// A program with its arguments, as a canned answer names it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Invocation {
    program: String,
    args: Vec<String>,
}

impl Invocation {
    fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.to_string(),
            args: to_strings(args),
        }
    }

    fn is(&self, program: &str, args: &[String]) -> bool {
        self.program == program && self.args == args
    }
}

/// The screens a canned hidden terminal has still to show, and the one it
/// shows.
#[derive(Debug)]
struct Screens {
    next: VecDeque<String>,
    shown: String,
}

/// Whether the part may start `program` (`allowed`) with the variables
/// `env`: a variable it does not list is refused, as OTTY refuses it.
fn check(
    permissions: &Permissions,
    allowed: bool,
    program: &str,
    env: &[(String, String)],
) -> Result<(), HostError> {
    if !allowed {
        return Err(HostError::NotAllowed(program.to_string()));
    }
    if let Some((name, _)) =
        env.iter().find(|(name, _)| !permissions.may_read_var(name))
    {
        return Err(HostError::NotAllowed(format!("variable {name}")));
    }

    Ok(())
}

/// The canned answer for `program` with `args` among `answers`.
fn canned<'answers, T>(
    answers: &'answers [(Invocation, T)],
    program: &str,
    args: &[String],
) -> Result<&'answers T, HostError> {
    answers
        .iter()
        .find(|(invocation, _)| invocation.is(program, args))
        .map(|(_, answer)| answer)
        .ok_or_else(|| {
            HostError::Failed(format!(
                "no canned answer for {program} {}",
                args.join(" ")
            ))
        })
}

fn unknown(what: &str, id: usize) -> HostError {
    HostError::NotFound(format!("{what} {id}"))
}

fn to_strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_string()).collect()
}
