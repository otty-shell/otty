use std::collections::HashMap;

use super::id::{
    BlockId, ProtocolSequence, ShellInstanceId, TerminalSessionId,
};
use super::model::{BlockOutcome, BlockRecord, BlockState};

/// Current shell-integration capability state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum IntegrationStatus {
    /// No successful protocol handshake has arrived yet.
    #[default]
    Pending,
    /// A supported protocol version is active.
    Active(u16),
    /// Events remain usable, but lifecycle recovery was required.
    Degraded(DegradedReason),
    /// The shell reported a protocol version the model cannot apply.
    UnsupportedVersion(u16),
    /// The configured shell has no lifecycle integration.
    Unsupported(String),
}

/// User-visible reason for a degraded shell integration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DegradedReason {
    /// One or more ordered shell events were not observed.
    SequenceGap {
        /// Sequence that should have arrived next.
        expected: ProtocolSequence,
        /// Sequence that was actually received.
        received: ProtocolSequence,
    },
    /// A lifecycle event arrived before the shell handshake.
    MissingShellHello,
    /// The shell integration explicitly reported an error code.
    IntegrationError(String),
}

/// Typed shell lifecycle event after transport decoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// Register a root or nested shell context.
    ShellHello {
        /// Parent shell for a nested context.
        parent_shell_instance_id: Option<ShellInstanceId>,
        /// Shell implementation name.
        shell: String,
        /// Optional implementation version.
        shell_version: Option<String>,
    },
    /// Prepare the stable block that will own the next command.
    PromptPrepare {
        /// Stable block identity.
        block_id: BlockId,
        /// Working directory before command execution.
        cwd: Option<String>,
        /// Timestamp at which prompt preparation began.
        prepared_at: Option<i64>,
    },
    /// Start execution in a prepared block.
    CommandStart {
        /// Stable block identity.
        block_id: BlockId,
        /// Best available canonical command text.
        command: Option<String>,
        /// Working directory before command execution.
        cwd: Option<String>,
        /// Execution start timestamp.
        started_at: Option<i64>,
    },
    /// Finish one exact command block, including a successful empty submission.
    CommandEnd {
        /// Stable block identity.
        block_id: BlockId,
        /// Process completion result.
        outcome: BlockOutcome,
        /// Working directory after command execution.
        cwd: Option<String>,
        /// Execution finish timestamp.
        finished_at: Option<i64>,
    },
    /// Patch context fields that changed outside a normal prompt transition.
    ContextUpdate {
        /// Current shell working directory.
        cwd: Option<String>,
    },
    /// Close the current shell context.
    ShellExit {
        /// Shell process status, when available.
        status: Option<i32>,
        /// Shell exit timestamp.
        finished_at: Option<i64>,
    },
    /// Record a safe shell-side integration error code.
    IntegrationError {
        /// Stable reason code without command or output contents.
        code: String,
    },
}

/// Non-fatal lifecycle anomaly retained for diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleDiagnostic {
    /// The event belongs to another terminal session.
    ForeignSession {
        /// Rejected session identity.
        received: TerminalSessionId,
    },
    /// The same sequence was received more than once.
    DuplicateSequence {
        /// Shell that emitted the event.
        shell_instance_id: ShellInstanceId,
        /// Repeated sequence.
        received: ProtocolSequence,
    },
    /// A sequence older than the latest applied event was received.
    StaleSequence {
        /// Shell that emitted the event.
        shell_instance_id: ShellInstanceId,
        /// Stale sequence.
        received: ProtocolSequence,
    },
    /// The ordered event stream contains a gap.
    SequenceGap {
        /// Shell that emitted the event.
        shell_instance_id: ShellInstanceId,
        /// Sequence expected next.
        expected: ProtocolSequence,
        /// Sequence actually received.
        received: ProtocolSequence,
    },
    /// A missing command completion was synthesized at the next prompt.
    SynthesizedCompletion {
        /// Block completed with an unknown outcome.
        block_id: BlockId,
    },
    /// A command start created a missing prepared block.
    RecoveredCommandStart {
        /// Recovered block identity.
        block_id: BlockId,
    },
    /// Completion referenced a block that is not retained.
    OrphanCommandEnd {
        /// Unknown block identity.
        block_id: BlockId,
    },
    /// A lifecycle event could not be applied in the current block state.
    InvalidTransition {
        /// Block targeted by the invalid transition.
        block_id: BlockId,
        /// State observed by the reducer.
        state: BlockState,
    },
    /// An event targeted a block owned by another shell context.
    ForeignShellBlock {
        /// Stable identity of the rejected block target.
        block_id: BlockId,
        /// Shell context that owns the block.
        expected_shell_instance_id: ShellInstanceId,
        /// Shell context that emitted the rejected event.
        received_shell_instance_id: ShellInstanceId,
    },
    /// A non-handshake event arrived before shell registration.
    MissingShellHello {
        /// Unregistered shell identity.
        shell_instance_id: ShellInstanceId,
    },
}

/// Protocol envelope fields paired with one semantic lifecycle event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleInput {
    session_id: TerminalSessionId,
    shell_instance_id: ShellInstanceId,
    sequence: ProtocolSequence,
    event: LifecycleEvent,
}

impl LifecycleInput {
    /// Build validated reducer input from a decoded protocol envelope.
    pub fn new(
        session_id: TerminalSessionId,
        shell_instance_id: ShellInstanceId,
        sequence: ProtocolSequence,
        event: LifecycleEvent,
    ) -> Self {
        Self {
            session_id,
            shell_instance_id,
            sequence,
            event,
        }
    }
}

/// Summary of model changes caused by one lifecycle event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifecycleUpdate {
    changed_blocks: Vec<BlockId>,
    ignored: bool,
}

impl LifecycleUpdate {
    /// Return block identities whose state or metadata changed.
    pub fn changed_blocks(&self) -> &[BlockId] {
        &self.changed_blocks
    }

    /// Return whether the event was rejected or idempotently ignored.
    pub fn is_ignored(&self) -> bool {
        self.ignored
    }

    fn ignored() -> Self {
        Self {
            changed_blocks: Vec::new(),
            ignored: true,
        }
    }

    fn changed(block_id: BlockId) -> Self {
        Self {
            changed_blocks: vec![block_id],
            ignored: false,
        }
    }
}

#[derive(Clone, Debug)]
struct ShellLifecycle {
    parent_shell_instance_id: Option<ShellInstanceId>,
    shell: Option<String>,
    shell_version: Option<String>,
    last_sequence: Option<ProtocolSequence>,
    active_block_id: Option<BlockId>,
    cwd: Option<String>,
    did_hello: bool,
    did_exit: bool,
}

impl ShellLifecycle {
    fn new() -> Self {
        Self {
            parent_shell_instance_id: None,
            shell: None,
            shell_version: None,
            last_sequence: None,
            active_block_id: None,
            cwd: None,
            did_hello: false,
            did_exit: false,
        }
    }
}

/// Deterministic state machine for ordered shell lifecycle events.
pub struct LifecycleReducer {
    session_id: TerminalSessionId,
    shells: HashMap<ShellInstanceId, ShellLifecycle>,
    blocks: HashMap<BlockId, BlockRecord>,
    diagnostics: Vec<LifecycleDiagnostic>,
    status: IntegrationStatus,
}

impl LifecycleReducer {
    /// Return a retained block by stable identity.
    pub fn block(&self, block_id: &BlockId) -> Option<&BlockRecord> {
        self.blocks.get(block_id)
    }

    /// Return non-fatal lifecycle diagnostics without command or output data.
    pub fn diagnostics(&self) -> &[LifecycleDiagnostic] {
        &self.diagnostics
    }

    /// Return current integration status.
    pub fn status(&self) -> &IntegrationStatus {
        &self.status
    }

    /// Create a reducer accepting only one registered terminal session.
    pub fn new(session_id: TerminalSessionId) -> Self {
        Self {
            session_id,
            shells: HashMap::new(),
            blocks: HashMap::new(),
            diagnostics: Vec::new(),
            status: IntegrationStatus::Pending,
        }
    }

    /// Record an unsupported major only for this reducer's registered session.
    pub fn mark_unsupported_version(
        &mut self,
        received_session_id: TerminalSessionId,
        version: u16,
    ) -> bool {
        if received_session_id != self.session_id {
            self.diagnostics.push(LifecycleDiagnostic::ForeignSession {
                received: received_session_id,
            });
            return false;
        }

        self.status = IntegrationStatus::UnsupportedVersion(version);
        true
    }

    /// Validate ordering and apply one semantic lifecycle event.
    pub fn apply(&mut self, input: LifecycleInput) -> LifecycleUpdate {
        let LifecycleInput {
            session_id,
            shell_instance_id,
            sequence,
            event,
        } = input;

        if session_id != self.session_id {
            self.diagnostics.push(LifecycleDiagnostic::ForeignSession {
                received: session_id,
            });
            return LifecycleUpdate::ignored();
        }

        if !self.accept_sequence(&shell_instance_id, sequence) {
            return LifecycleUpdate::ignored();
        }

        if !matches!(event, LifecycleEvent::ShellHello { .. })
            && !self
                .shells
                .get(&shell_instance_id)
                .is_some_and(|shell| shell.did_hello)
        {
            self.status =
                IntegrationStatus::Degraded(DegradedReason::MissingShellHello);
            self.diagnostics
                .push(LifecycleDiagnostic::MissingShellHello {
                    shell_instance_id: shell_instance_id.clone(),
                });
        }

        match event {
            LifecycleEvent::ShellHello {
                parent_shell_instance_id,
                shell,
                shell_version,
            } => self.apply_shell_hello(
                shell_instance_id,
                parent_shell_instance_id,
                shell,
                shell_version,
            ),
            LifecycleEvent::PromptPrepare {
                block_id,
                cwd,
                prepared_at,
            } => self.apply_prompt_prepare(
                shell_instance_id,
                block_id,
                cwd,
                prepared_at,
            ),
            LifecycleEvent::CommandStart {
                block_id,
                command,
                cwd,
                started_at,
            } => self.apply_command_start(
                shell_instance_id,
                block_id,
                command,
                cwd,
                started_at,
            ),
            LifecycleEvent::CommandEnd {
                block_id,
                outcome,
                cwd,
                finished_at,
            } => self.apply_command_end(
                &shell_instance_id,
                block_id,
                outcome,
                cwd,
                finished_at,
            ),
            LifecycleEvent::ContextUpdate { cwd } => {
                self.apply_context_update(&shell_instance_id, cwd)
            },
            LifecycleEvent::ShellExit {
                status,
                finished_at,
            } => self.apply_shell_exit(&shell_instance_id, status, finished_at),
            LifecycleEvent::IntegrationError { code } => {
                self.status = IntegrationStatus::Degraded(
                    DegradedReason::IntegrationError(code),
                );
                LifecycleUpdate::default()
            },
        }
    }

    fn accept_sequence(
        &mut self,
        shell_instance_id: &ShellInstanceId,
        received: ProtocolSequence,
    ) -> bool {
        let previous = self
            .shells
            .get(shell_instance_id)
            .and_then(|shell| shell.last_sequence);

        if let Some(previous) = previous {
            if received == previous {
                self.diagnostics
                    .push(LifecycleDiagnostic::DuplicateSequence {
                        shell_instance_id: shell_instance_id.clone(),
                        received,
                    });
                return false;
            }
            if received < previous {
                self.diagnostics.push(LifecycleDiagnostic::StaleSequence {
                    shell_instance_id: shell_instance_id.clone(),
                    received,
                });
                return false;
            }
        }

        let expected = previous
            .and_then(ProtocolSequence::next)
            .unwrap_or_else(|| ProtocolSequence::new(1));
        if received > expected {
            let reason = DegradedReason::SequenceGap { expected, received };
            self.status = IntegrationStatus::Degraded(reason);
            self.diagnostics.push(LifecycleDiagnostic::SequenceGap {
                shell_instance_id: shell_instance_id.clone(),
                expected,
                received,
            });
        }

        self.shells
            .entry(shell_instance_id.clone())
            .or_insert_with(ShellLifecycle::new)
            .last_sequence = Some(received);
        true
    }

    fn apply_shell_hello(
        &mut self,
        shell_instance_id: ShellInstanceId,
        parent_shell_instance_id: Option<ShellInstanceId>,
        shell: String,
        shell_version: Option<String>,
    ) -> LifecycleUpdate {
        let context = self
            .shells
            .entry(shell_instance_id)
            .or_insert_with(ShellLifecycle::new);
        context.parent_shell_instance_id = parent_shell_instance_id;
        context.shell = Some(shell);
        context.shell_version = shell_version;
        context.did_hello = true;
        context.did_exit = false;

        if !matches!(self.status, IntegrationStatus::Degraded(_)) {
            self.status = IntegrationStatus::Active(2);
        }

        LifecycleUpdate::default()
    }

    fn apply_prompt_prepare(
        &mut self,
        shell_instance_id: ShellInstanceId,
        block_id: BlockId,
        cwd: Option<String>,
        prepared_at: Option<i64>,
    ) -> LifecycleUpdate {
        if self.reject_foreign_shell_block(&shell_instance_id, &block_id) {
            return LifecycleUpdate::ignored();
        }

        let mut changed_blocks = Vec::new();

        match self.blocks.get_mut(&block_id) {
            Some(block) if block.state() == &BlockState::BeforeExecution => {
                let is_active = self
                    .shells
                    .get(&shell_instance_id)
                    .and_then(|shell| shell.active_block_id.as_ref())
                    == Some(&block_id);
                if !is_active {
                    self.diagnostics.push(
                        LifecycleDiagnostic::InvalidTransition {
                            block_id: block_id.clone(),
                            state: block.state().clone(),
                        },
                    );
                    return LifecycleUpdate::ignored();
                }

                block.patch_prompt(cwd.clone());
                changed_blocks.push(block_id.clone());
            },
            Some(block) => {
                self.diagnostics
                    .push(LifecycleDiagnostic::InvalidTransition {
                        block_id: block_id.clone(),
                        state: block.state().clone(),
                    });
                return LifecycleUpdate::ignored();
            },
            None => {
                if let Some(recovered) = self.finish_active_for_recovery(
                    &shell_instance_id,
                    &block_id,
                    prepared_at,
                ) {
                    changed_blocks.push(recovered);
                }

                self.blocks.insert(
                    block_id.clone(),
                    BlockRecord::prepared(
                        block_id.clone(),
                        shell_instance_id.clone(),
                        cwd.clone(),
                    ),
                );
                changed_blocks.push(block_id.clone());
            },
        }

        let context = self
            .shells
            .entry(shell_instance_id)
            .or_insert_with(ShellLifecycle::new);
        context.active_block_id = Some(block_id);
        if cwd.is_some() {
            context.cwd = cwd;
        }

        LifecycleUpdate {
            changed_blocks,
            ignored: false,
        }
    }

    fn apply_command_start(
        &mut self,
        shell_instance_id: ShellInstanceId,
        block_id: BlockId,
        command: Option<String>,
        cwd: Option<String>,
        started_at: Option<i64>,
    ) -> LifecycleUpdate {
        if self.reject_foreign_shell_block(&shell_instance_id, &block_id) {
            return LifecycleUpdate::ignored();
        }

        let mut changed_blocks = Vec::new();
        match self.blocks.get_mut(&block_id) {
            Some(block) if block.state() == &BlockState::BeforeExecution => {
                let is_active = self
                    .shells
                    .get(&shell_instance_id)
                    .and_then(|shell| shell.active_block_id.as_ref())
                    == Some(&block_id);
                if !is_active {
                    self.diagnostics.push(
                        LifecycleDiagnostic::InvalidTransition {
                            block_id: block_id.clone(),
                            state: block.state().clone(),
                        },
                    );
                    return LifecycleUpdate::ignored();
                }

                block.start(command, cwd.clone(), started_at);
                changed_blocks.push(block_id.clone());
            },
            Some(block) if block.state() == &BlockState::Executing => {
                return LifecycleUpdate::ignored();
            },
            Some(block) => {
                self.diagnostics
                    .push(LifecycleDiagnostic::InvalidTransition {
                        block_id,
                        state: block.state().clone(),
                    });
                return LifecycleUpdate::ignored();
            },
            None => {
                if let Some(recovered) = self.finish_active_for_recovery(
                    &shell_instance_id,
                    &block_id,
                    started_at,
                ) {
                    changed_blocks.push(recovered);
                }

                self.diagnostics.push(
                    LifecycleDiagnostic::RecoveredCommandStart {
                        block_id: block_id.clone(),
                    },
                );
                self.blocks.insert(
                    block_id.clone(),
                    BlockRecord::executing(
                        block_id.clone(),
                        shell_instance_id.clone(),
                        command,
                        cwd.clone(),
                        started_at,
                    ),
                );
                changed_blocks.push(block_id.clone());
            },
        }

        let context = self
            .shells
            .entry(shell_instance_id)
            .or_insert_with(ShellLifecycle::new);
        context.active_block_id = Some(block_id.clone());
        if cwd.is_some() {
            context.cwd = cwd;
        }

        LifecycleUpdate {
            changed_blocks,
            ignored: false,
        }
    }

    fn apply_command_end(
        &mut self,
        shell_instance_id: &ShellInstanceId,
        block_id: BlockId,
        outcome: BlockOutcome,
        cwd: Option<String>,
        finished_at: Option<i64>,
    ) -> LifecycleUpdate {
        if self.reject_foreign_shell_block(shell_instance_id, &block_id) {
            return LifecycleUpdate::ignored();
        }

        let Some(block) = self.blocks.get_mut(&block_id) else {
            self.diagnostics
                .push(LifecycleDiagnostic::OrphanCommandEnd { block_id });
            return LifecycleUpdate::ignored();
        };

        let is_active = self
            .shells
            .get(shell_instance_id)
            .and_then(|shell| shell.active_block_id.as_ref())
            == Some(&block_id);
        let is_successful_empty_completion =
            is_active && outcome == BlockOutcome::Exited(0);

        match block.state().clone() {
            BlockState::Executing | BlockState::BackgroundRunning => {
                block.finish(outcome, cwd.clone(), finished_at);
            },
            BlockState::BeforeExecution if is_successful_empty_completion => {
                block.finish(outcome, cwd.clone(), finished_at);
            },
            BlockState::Finished(_) | BlockState::BackgroundFinished => {
                return LifecycleUpdate::ignored();
            },
            _ => {
                self.diagnostics
                    .push(LifecycleDiagnostic::InvalidTransition {
                        block_id,
                        state: block.state().clone(),
                    });
                return LifecycleUpdate::ignored();
            },
        }

        if let Some(context) = self.shells.get_mut(shell_instance_id) {
            if context.active_block_id.as_ref() == Some(&block_id) {
                context.active_block_id = None;
            }
            if cwd.is_some() {
                context.cwd = cwd;
            }
        }

        LifecycleUpdate::changed(block_id)
    }

    fn apply_context_update(
        &mut self,
        shell_instance_id: &ShellInstanceId,
        cwd: Option<String>,
    ) -> LifecycleUpdate {
        let Some(context) = self.shells.get_mut(shell_instance_id) else {
            return LifecycleUpdate::ignored();
        };

        if cwd.is_some() {
            context.cwd = cwd;
        }

        LifecycleUpdate::default()
    }

    fn apply_shell_exit(
        &mut self,
        shell_instance_id: &ShellInstanceId,
        _status: Option<i32>,
        finished_at: Option<i64>,
    ) -> LifecycleUpdate {
        let Some(context) = self.shells.get_mut(shell_instance_id) else {
            return LifecycleUpdate::ignored();
        };

        context.did_exit = true;
        let active_block_id = context.active_block_id.take();
        let Some(active_block_id) = active_block_id else {
            return LifecycleUpdate::default();
        };
        let Some(block) = self.blocks.get_mut(&active_block_id) else {
            return LifecycleUpdate::default();
        };

        if !matches!(block.state(), BlockState::Finished(_)) {
            block.finish(BlockOutcome::ShellExited, None, finished_at);
            return LifecycleUpdate::changed(active_block_id);
        }

        LifecycleUpdate::default()
    }

    fn reject_foreign_shell_block(
        &mut self,
        received_shell_instance_id: &ShellInstanceId,
        block_id: &BlockId,
    ) -> bool {
        let expected_shell_instance_id = self
            .blocks
            .get(block_id)
            .map(|block| block.shell_instance_id().clone());
        let Some(expected_shell_instance_id) = expected_shell_instance_id
        else {
            return false;
        };
        if expected_shell_instance_id == *received_shell_instance_id {
            return false;
        }

        self.diagnostics
            .push(LifecycleDiagnostic::ForeignShellBlock {
                block_id: block_id.clone(),
                expected_shell_instance_id,
                received_shell_instance_id: received_shell_instance_id.clone(),
            });

        true
    }

    fn finish_active_for_recovery(
        &mut self,
        shell_instance_id: &ShellInstanceId,
        next_block_id: &BlockId,
        finished_at: Option<i64>,
    ) -> Option<BlockId> {
        let active_block_id = self
            .shells
            .get(shell_instance_id)
            .and_then(|shell| shell.active_block_id.clone())?;
        if active_block_id == *next_block_id {
            return None;
        }

        let active = self.blocks.get_mut(&active_block_id)?;
        if active.state() != &BlockState::Executing {
            return None;
        }

        active.finish(BlockOutcome::Unknown, None, finished_at);
        self.diagnostics
            .push(LifecycleDiagnostic::SynthesizedCompletion {
                block_id: active_block_id.clone(),
            });

        Some(active_block_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DegradedReason, IntegrationStatus, LifecycleDiagnostic, LifecycleEvent,
        LifecycleInput, LifecycleReducer,
    };
    use crate::block::id::{
        BlockId, ProtocolSequence, ShellInstanceId, TerminalSessionId,
    };
    use crate::block::{BlockOutcome, BlockState};

    fn session_id() -> TerminalSessionId {
        TerminalSessionId::new("session")
    }

    fn shell_id() -> ShellInstanceId {
        ShellInstanceId::new("shell")
    }

    fn block_id(sequence: u64) -> BlockId {
        BlockId::terminal(&session_id(), &shell_id(), sequence)
    }

    fn input(sequence: u64, event: LifecycleEvent) -> LifecycleInput {
        LifecycleInput::new(
            session_id(),
            shell_id(),
            ProtocolSequence::new(sequence),
            event,
        )
    }

    fn hello(sequence: u64) -> LifecycleInput {
        input(
            sequence,
            LifecycleEvent::ShellHello {
                parent_shell_instance_id: None,
                shell: "bash".to_string(),
                shell_version: Some("5.2".to_string()),
            },
        )
    }

    #[test]
    fn normal_lifecycle_preserves_start_metadata_on_completion() {
        let mut reducer = LifecycleReducer::new(session_id());
        let id = block_id(1);
        let next_id = block_id(2);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::PromptPrepare {
                block_id: id.clone(),
                cwd: Some("/before".to_string()),
                prepared_at: Some(10),
            },
        ));
        reducer.apply(input(
            3,
            LifecycleEvent::CommandStart {
                block_id: id.clone(),
                command: Some("printf ok".to_string()),
                cwd: Some("/before".to_string()),
                started_at: Some(11),
            },
        ));
        reducer.apply(input(
            4,
            LifecycleEvent::CommandEnd {
                block_id: id.clone(),
                outcome: BlockOutcome::Exited(7),
                cwd: Some("/after".to_string()),
                finished_at: Some(20),
            },
        ));
        reducer.apply(input(
            5,
            LifecycleEvent::PromptPrepare {
                block_id: next_id.clone(),
                cwd: Some(String::from("/after")),
                prepared_at: Some(21),
            },
        ));

        let block = reducer.block(&id).expect("block should exist");
        assert_eq!(
            block.state(),
            &BlockState::Finished(BlockOutcome::Exited(7))
        );
        assert_eq!(block.metadata().command(), Some("printf ok"));
        assert_eq!(block.metadata().cwd_before(), Some("/before"));
        assert_eq!(block.metadata().cwd_after(), Some("/after"));
        assert_eq!(block.metadata().started_at(), Some(11));
        assert_eq!(block.metadata().finished_at(), Some(20));
        assert_eq!(
            reducer.block(&next_id).map(|block| block.state()),
            Some(&BlockState::BeforeExecution),
        );
    }

    #[test]
    fn empty_enter_without_command_start_finishes_only_its_own_block() {
        let mut reducer = LifecycleReducer::new(session_id());
        let previous = block_id(1);
        let empty = block_id(2);
        let next = block_id(3);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::PromptPrepare {
                block_id: previous.clone(),
                cwd: Some(String::from("/before-true")),
                prepared_at: Some(2),
            },
        ));
        reducer.apply(input(
            3,
            LifecycleEvent::CommandStart {
                block_id: previous.clone(),
                command: Some(String::from("true")),
                cwd: None,
                started_at: Some(3),
            },
        ));
        reducer.apply(input(
            4,
            LifecycleEvent::CommandEnd {
                block_id: previous.clone(),
                outcome: BlockOutcome::Exited(0),
                cwd: Some(String::from("/after-true")),
                finished_at: Some(4),
            },
        ));
        let previous_after_completion = reducer
            .block(&previous)
            .cloned()
            .expect("previous command block should exist");

        reducer.apply(input(
            5,
            LifecycleEvent::PromptPrepare {
                block_id: empty.clone(),
                cwd: Some(String::from("/after-true")),
                prepared_at: Some(5),
            },
        ));
        let completion = reducer.apply(input(
            6,
            LifecycleEvent::CommandEnd {
                block_id: empty.clone(),
                outcome: BlockOutcome::Exited(0),
                cwd: Some(String::from("/after-empty")),
                finished_at: Some(6),
            },
        ));
        reducer.apply(input(
            7,
            LifecycleEvent::PromptPrepare {
                block_id: next.clone(),
                cwd: Some(String::from("/after-empty")),
                prepared_at: Some(7),
            },
        ));

        assert!(!completion.is_ignored());
        assert_eq!(completion.changed_blocks(), std::slice::from_ref(&empty));
        assert_eq!(reducer.block(&previous), Some(&previous_after_completion));

        let empty_block = reducer
            .block(&empty)
            .expect("empty command block should exist");
        assert_eq!(
            empty_block.state(),
            &BlockState::Finished(BlockOutcome::Exited(0))
        );
        assert_eq!(empty_block.metadata().command(), None);
        assert_eq!(empty_block.metadata().started_at(), None);
        assert_eq!(empty_block.metadata().finished_at(), Some(6));
        assert_eq!(empty_block.metadata().exit_code(), Some(0));
        assert_eq!(
            reducer.block(&next).map(|block| block.state()),
            Some(&BlockState::BeforeExecution)
        );
    }

    #[test]
    fn duplicate_and_stale_sequences_do_not_change_a_finished_block() {
        let mut reducer = LifecycleReducer::new(session_id());
        let id = block_id(1);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::CommandStart {
                block_id: id.clone(),
                command: Some("true".to_string()),
                cwd: None,
                started_at: Some(2),
            },
        ));
        reducer.apply(input(
            3,
            LifecycleEvent::CommandEnd {
                block_id: id.clone(),
                outcome: BlockOutcome::Exited(0),
                cwd: None,
                finished_at: Some(3),
            },
        ));
        reducer.apply(input(
            3,
            LifecycleEvent::CommandEnd {
                block_id: id.clone(),
                outcome: BlockOutcome::Exited(99),
                cwd: None,
                finished_at: Some(99),
            },
        ));
        reducer.apply(input(
            2,
            LifecycleEvent::CommandEnd {
                block_id: id.clone(),
                outcome: BlockOutcome::Exited(98),
                cwd: None,
                finished_at: Some(98),
            },
        ));

        let block = reducer.block(&id).expect("block should exist");
        assert_eq!(
            block.state(),
            &BlockState::Finished(BlockOutcome::Exited(0))
        );
        assert_eq!(block.metadata().finished_at(), Some(3));
        assert!(reducer.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            LifecycleDiagnostic::DuplicateSequence { received, .. }
                if received.value() == 3
        )));
        assert!(reducer.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            LifecycleDiagnostic::StaleSequence { received, .. }
                if received.value() == 2
        )));
    }

    #[test]
    fn sequence_gap_marks_integration_degraded_but_applies_event() {
        let mut reducer = LifecycleReducer::new(session_id());
        let id = block_id(1);

        reducer.apply(hello(1));
        reducer.apply(input(
            3,
            LifecycleEvent::PromptPrepare {
                block_id: id.clone(),
                cwd: Some("/tmp".to_string()),
                prepared_at: Some(3),
            },
        ));

        assert!(reducer.block(&id).is_some());
        assert_eq!(
            reducer.status(),
            &IntegrationStatus::Degraded(DegradedReason::SequenceGap {
                expected: ProtocolSequence::new(2),
                received: ProtocolSequence::new(3),
            })
        );
    }

    #[test]
    fn prompt_prepare_recovers_missing_command_end() {
        let mut reducer = LifecycleReducer::new(session_id());
        let running = block_id(1);
        let next = block_id(2);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::CommandStart {
                block_id: running.clone(),
                command: Some("sleep 1".to_string()),
                cwd: None,
                started_at: Some(2),
            },
        ));
        reducer.apply(input(
            3,
            LifecycleEvent::PromptPrepare {
                block_id: next.clone(),
                cwd: None,
                prepared_at: Some(3),
            },
        ));

        assert_eq!(
            reducer.block(&running).map(|block| block.state()),
            Some(&BlockState::Finished(BlockOutcome::Unknown))
        );
        assert_eq!(
            reducer.block(&next).map(|block| block.state()),
            Some(&BlockState::BeforeExecution)
        );
        assert!(reducer.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            LifecycleDiagnostic::SynthesizedCompletion { block_id }
                if block_id == &running
        )));
    }

    #[test]
    fn stale_prepared_block_id_does_not_finish_active_neighbor() {
        let mut reducer = LifecycleReducer::new(session_id());
        let stale = block_id(1);
        let active = block_id(2);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::PromptPrepare {
                block_id: stale.clone(),
                cwd: None,
                prepared_at: None,
            },
        ));
        reducer.apply(input(
            3,
            LifecycleEvent::PromptPrepare {
                block_id: active.clone(),
                cwd: None,
                prepared_at: None,
            },
        ));
        reducer.apply(input(
            4,
            LifecycleEvent::CommandStart {
                block_id: active.clone(),
                command: Some(String::from("sleep 10")),
                cwd: None,
                started_at: None,
            },
        ));

        let update = reducer.apply(input(
            5,
            LifecycleEvent::PromptPrepare {
                block_id: stale.clone(),
                cwd: None,
                prepared_at: None,
            },
        ));

        assert!(update.is_ignored());
        assert_eq!(
            reducer.block(&active).map(|block| block.state()),
            Some(&BlockState::Executing),
        );
        assert_eq!(
            reducer.block(&stale).map(|block| block.state()),
            Some(&BlockState::BeforeExecution),
        );
        assert!(matches!(
            reducer.diagnostics().last(),
            Some(LifecycleDiagnostic::InvalidTransition {
                block_id,
                state: BlockState::BeforeExecution,
            }) if block_id == &stale
        ));

        let update = reducer.apply(input(
            6,
            LifecycleEvent::CommandStart {
                block_id: stale.clone(),
                command: Some(String::from("must stay stale")),
                cwd: None,
                started_at: None,
            },
        ));

        assert!(update.is_ignored());
        assert_eq!(
            reducer.block(&active).map(|block| block.state()),
            Some(&BlockState::Executing),
        );
        assert_eq!(
            reducer.block(&stale).map(|block| block.state()),
            Some(&BlockState::BeforeExecution),
        );
    }

    #[test]
    fn command_start_recovers_missing_end_and_prepare_together() {
        let mut reducer = LifecycleReducer::new(session_id());
        let previous = block_id(1);
        let recovered = block_id(2);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::CommandStart {
                block_id: previous.clone(),
                command: Some(String::from("first")),
                cwd: None,
                started_at: Some(2),
            },
        ));

        let update = reducer.apply(input(
            4,
            LifecycleEvent::CommandStart {
                block_id: recovered.clone(),
                command: Some(String::from("second")),
                cwd: None,
                started_at: Some(4),
            },
        ));

        assert_eq!(
            reducer.block(&previous).map(|block| block.state()),
            Some(&BlockState::Finished(BlockOutcome::Unknown)),
        );
        assert_eq!(
            reducer.block(&recovered).map(|block| block.state()),
            Some(&BlockState::Executing),
        );
        assert_eq!(
            update.changed_blocks(),
            &[previous.clone(), recovered.clone()],
        );
        assert!(reducer.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            LifecycleDiagnostic::SynthesizedCompletion { block_id }
                if block_id == &previous
        )));
        assert!(reducer.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            LifecycleDiagnostic::RecoveredCommandStart { block_id }
                if block_id == &recovered
        )));
    }

    #[test]
    fn command_start_without_prepare_creates_recovered_block() {
        let mut reducer = LifecycleReducer::new(session_id());
        let id = block_id(1);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::CommandStart {
                block_id: id.clone(),
                command: Some("echo recovered".to_string()),
                cwd: None,
                started_at: Some(2),
            },
        ));

        assert_eq!(
            reducer.block(&id).map(|block| block.state()),
            Some(&BlockState::Executing)
        );
        assert!(reducer.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            LifecycleDiagnostic::RecoveredCommandStart { block_id }
                if block_id == &id
        )));
    }

    #[test]
    fn command_end_for_unknown_id_does_not_finish_neighbor() {
        let mut reducer = LifecycleReducer::new(session_id());
        let running = block_id(1);
        let unknown = block_id(999);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::CommandStart {
                block_id: running.clone(),
                command: Some("sleep 1".to_string()),
                cwd: None,
                started_at: Some(2),
            },
        ));
        reducer.apply(input(
            3,
            LifecycleEvent::CommandEnd {
                block_id: unknown.clone(),
                outcome: BlockOutcome::Exited(0),
                cwd: None,
                finished_at: Some(3),
            },
        ));

        assert_eq!(
            reducer.block(&running).map(|block| block.state()),
            Some(&BlockState::Executing)
        );
        assert!(reducer.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            LifecycleDiagnostic::OrphanCommandEnd { block_id }
                if block_id == &unknown
        )));
    }

    #[test]
    fn shell_exit_finishes_active_block() {
        let mut reducer = LifecycleReducer::new(session_id());
        let id = block_id(1);

        reducer.apply(hello(1));
        reducer.apply(input(
            2,
            LifecycleEvent::CommandStart {
                block_id: id.clone(),
                command: Some("exec false".to_string()),
                cwd: None,
                started_at: Some(2),
            },
        ));
        reducer.apply(input(
            3,
            LifecycleEvent::ShellExit {
                status: Some(1),
                finished_at: Some(3),
            },
        ));

        assert_eq!(
            reducer.block(&id).map(|block| block.state()),
            Some(&BlockState::Finished(BlockOutcome::ShellExited))
        );
    }

    #[test]
    fn event_from_another_terminal_session_is_rejected() {
        let mut reducer = LifecycleReducer::new(session_id());
        let foreign = LifecycleInput::new(
            TerminalSessionId::new("foreign"),
            shell_id(),
            ProtocolSequence::new(1),
            LifecycleEvent::ShellHello {
                parent_shell_instance_id: None,
                shell: "bash".to_string(),
                shell_version: None,
            },
        );

        reducer.apply(foreign);

        assert_eq!(reducer.status(), &IntegrationStatus::Pending);
        assert!(matches!(
            reducer.diagnostics().last(),
            Some(LifecycleDiagnostic::ForeignSession { .. })
        ));
    }

    #[test]
    fn unsupported_version_status_requires_the_registered_session() {
        let mut reducer = LifecycleReducer::new(session_id());

        assert!(!reducer.mark_unsupported_version(
            TerminalSessionId::new("foreign"),
            7,
        ));
        assert_eq!(reducer.status(), &IntegrationStatus::Pending);

        assert!(reducer.mark_unsupported_version(session_id(), 7));
        assert_eq!(reducer.status(), &IntegrationStatus::UnsupportedVersion(7));
    }

    #[test]
    fn integration_status_transition_matrix_is_explicit() {
        struct Case {
            name: &'static str,
            send_hello: bool,
            event: LifecycleEvent,
            expected: IntegrationStatus,
        }

        let cases = [
            Case {
                name: "event before hello degrades integration",
                send_hello: false,
                event: LifecycleEvent::ContextUpdate {
                    cwd: Some(String::from("/tmp")),
                },
                expected: IntegrationStatus::Degraded(
                    DegradedReason::MissingShellHello,
                ),
            },
            Case {
                name: "integration error degrades active integration",
                send_hello: true,
                event: LifecycleEvent::IntegrationError {
                    code: String::from("hook-lost"),
                },
                expected: IntegrationStatus::Degraded(
                    DegradedReason::IntegrationError(String::from("hook-lost")),
                ),
            },
        ];

        for case in cases {
            let mut reducer = LifecycleReducer::new(session_id());
            let sequence = if case.send_hello {
                reducer.apply(hello(1));
                2
            } else {
                1
            };

            reducer.apply(input(sequence, case.event));

            assert_eq!(reducer.status(), &case.expected, "{}", case.name);
        }
    }

    #[test]
    fn interruption_transition_matrix_is_deterministic() {
        struct Case {
            name: &'static str,
            start_command: bool,
            outcome: Option<BlockOutcome>,
            expected_state: BlockState,
        }

        let cases = [
            Case {
                name: "ctrl-c before command start",
                start_command: false,
                outcome: None,
                expected_state: BlockState::BeforeExecution,
            },
            Case {
                name: "ctrl-c after command start",
                start_command: true,
                outcome: Some(BlockOutcome::Signaled(2)),
                expected_state: BlockState::Finished(BlockOutcome::Signaled(2)),
            },
        ];

        for case in cases {
            let mut reducer = LifecycleReducer::new(session_id());
            let current = block_id(1);
            let next = block_id(2);
            let mut sequence = 1;
            reducer.apply(hello(sequence));
            sequence += 1;
            reducer.apply(input(
                sequence,
                LifecycleEvent::PromptPrepare {
                    block_id: current.clone(),
                    cwd: Some(String::from("/before")),
                    prepared_at: Some(sequence as i64),
                },
            ));

            if case.start_command {
                sequence += 1;
                reducer.apply(input(
                    sequence,
                    LifecycleEvent::CommandStart {
                        block_id: current.clone(),
                        command: Some(String::from("sleep 10")),
                        cwd: None,
                        started_at: Some(sequence as i64),
                    },
                ));
            }
            if let Some(outcome) = case.outcome {
                sequence += 1;
                reducer.apply(input(
                    sequence,
                    LifecycleEvent::CommandEnd {
                        block_id: current.clone(),
                        outcome,
                        cwd: None,
                        finished_at: Some(sequence as i64),
                    },
                ));
            }

            sequence += 1;
            reducer.apply(input(
                sequence,
                LifecycleEvent::PromptPrepare {
                    block_id: next.clone(),
                    cwd: None,
                    prepared_at: Some(sequence as i64),
                },
            ));

            assert_eq!(
                reducer.block(&current).map(|block| block.state()),
                Some(&case.expected_state),
                "{}",
                case.name,
            );
            assert_eq!(
                reducer.block(&next).map(|block| block.state()),
                Some(&BlockState::BeforeExecution),
                "{}",
                case.name,
            );
        }
    }

    #[test]
    fn invalid_transition_matrix_emits_payload_free_diagnostics() {
        enum InvalidEvent {
            EndBeforeStart,
            StartAfterFinish,
            PrepareAfterFinish,
        }

        for case in [
            InvalidEvent::EndBeforeStart,
            InvalidEvent::StartAfterFinish,
            InvalidEvent::PrepareAfterFinish,
        ] {
            let mut reducer = LifecycleReducer::new(session_id());
            let id = block_id(1);
            reducer.apply(hello(1));
            reducer.apply(input(
                2,
                LifecycleEvent::PromptPrepare {
                    block_id: id.clone(),
                    cwd: None,
                    prepared_at: None,
                },
            ));

            let invalid = match case {
                InvalidEvent::EndBeforeStart => LifecycleEvent::CommandEnd {
                    block_id: id.clone(),
                    outcome: BlockOutcome::Cancelled,
                    cwd: None,
                    finished_at: None,
                },
                InvalidEvent::StartAfterFinish
                | InvalidEvent::PrepareAfterFinish => {
                    reducer.apply(input(
                        3,
                        LifecycleEvent::CommandStart {
                            block_id: id.clone(),
                            command: Some(String::from("secret command")),
                            cwd: None,
                            started_at: None,
                        },
                    ));
                    reducer.apply(input(
                        4,
                        LifecycleEvent::CommandEnd {
                            block_id: id.clone(),
                            outcome: BlockOutcome::Exited(0),
                            cwd: None,
                            finished_at: None,
                        },
                    ));
                    match case {
                        InvalidEvent::StartAfterFinish => {
                            LifecycleEvent::CommandStart {
                                block_id: id.clone(),
                                command: Some(String::from(
                                    "must not be logged",
                                )),
                                cwd: None,
                                started_at: None,
                            }
                        },
                        InvalidEvent::PrepareAfterFinish => {
                            LifecycleEvent::PromptPrepare {
                                block_id: id.clone(),
                                cwd: None,
                                prepared_at: None,
                            }
                        },
                        InvalidEvent::EndBeforeStart => unreachable!(),
                    }
                },
            };
            let sequence = match case {
                InvalidEvent::EndBeforeStart => 3,
                _ => 5,
            };

            let update = reducer.apply(input(sequence, invalid));

            assert!(update.is_ignored());
            assert!(matches!(
                reducer.diagnostics().last(),
                Some(LifecycleDiagnostic::InvalidTransition {
                    block_id,
                    ..
                }) if block_id == &id
            ));
        }
    }

    #[test]
    fn block_events_cannot_cross_shell_ownership() {
        let mut reducer = LifecycleReducer::new(session_id());
        let root = ShellInstanceId::new("root");
        let child = ShellInstanceId::new("child");
        let root_block = BlockId::terminal(&session_id(), &root, 1);

        reducer.apply(LifecycleInput::new(
            session_id(),
            root.clone(),
            ProtocolSequence::new(1),
            LifecycleEvent::ShellHello {
                parent_shell_instance_id: None,
                shell: String::from("bash"),
                shell_version: None,
            },
        ));
        reducer.apply(LifecycleInput::new(
            session_id(),
            root.clone(),
            ProtocolSequence::new(2),
            LifecycleEvent::CommandStart {
                block_id: root_block.clone(),
                command: Some(String::from("sleep 10")),
                cwd: None,
                started_at: None,
            },
        ));
        reducer.apply(LifecycleInput::new(
            session_id(),
            child.clone(),
            ProtocolSequence::new(1),
            LifecycleEvent::ShellHello {
                parent_shell_instance_id: Some(root.clone()),
                shell: String::from("bash"),
                shell_version: None,
            },
        ));

        let update = reducer.apply(LifecycleInput::new(
            session_id(),
            child.clone(),
            ProtocolSequence::new(2),
            LifecycleEvent::CommandEnd {
                block_id: root_block.clone(),
                outcome: BlockOutcome::Exited(0),
                cwd: None,
                finished_at: None,
            },
        ));

        assert!(update.is_ignored());
        assert_eq!(
            reducer.block(&root_block).map(|block| block.state()),
            Some(&BlockState::Executing),
        );
        assert!(matches!(
            reducer.diagnostics().last(),
            Some(LifecycleDiagnostic::ForeignShellBlock {
                block_id,
                expected_shell_instance_id,
                received_shell_instance_id,
            }) if block_id == &root_block
                && expected_shell_instance_id == &root
                && received_shell_instance_id == &child
        ));
    }

    #[test]
    fn nested_and_root_shell_exit_finish_only_owned_active_blocks() {
        let mut reducer = LifecycleReducer::new(session_id());
        let root = ShellInstanceId::new("root");
        let child = ShellInstanceId::new("child");
        let root_block = BlockId::terminal(&session_id(), &root, 1);
        let child_block = BlockId::terminal(&session_id(), &child, 1);

        for (shell, parent, block) in [
            (root.clone(), None, root_block.clone()),
            (child.clone(), Some(root.clone()), child_block.clone()),
        ] {
            reducer.apply(LifecycleInput::new(
                session_id(),
                shell.clone(),
                ProtocolSequence::new(1),
                LifecycleEvent::ShellHello {
                    parent_shell_instance_id: parent,
                    shell: String::from("bash"),
                    shell_version: None,
                },
            ));
            reducer.apply(LifecycleInput::new(
                session_id(),
                shell,
                ProtocolSequence::new(2),
                LifecycleEvent::CommandStart {
                    block_id: block,
                    command: Some(String::from("sleep 10")),
                    cwd: None,
                    started_at: None,
                },
            ));
        }

        reducer.apply(LifecycleInput::new(
            session_id(),
            child.clone(),
            ProtocolSequence::new(3),
            LifecycleEvent::ShellExit {
                status: Some(1),
                finished_at: Some(30),
            },
        ));

        assert_eq!(
            reducer.block(&child_block).map(|block| block.state()),
            Some(&BlockState::Finished(BlockOutcome::ShellExited)),
        );
        assert_eq!(
            reducer.block(&root_block).map(|block| block.state()),
            Some(&BlockState::Executing),
        );

        reducer.apply(LifecycleInput::new(
            session_id(),
            root,
            ProtocolSequence::new(3),
            LifecycleEvent::ShellExit {
                status: Some(0),
                finished_at: Some(31),
            },
        ));

        assert_eq!(
            reducer.block(&root_block).map(|block| block.state()),
            Some(&BlockState::Finished(BlockOutcome::ShellExited)),
        );
    }
}
