use std::collections::HashMap;

use super::id::{
    BlockId, ProtocolSequence, ShellInstanceId, TerminalSessionId,
};

/// Terminal execution state governed by the lifecycle reducer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockState {
    /// Prompt and command input exist, but execution has not started.
    BeforeExecution,
    /// The command currently owns ordinary PTY output.
    Executing,
    /// The command completed with a stable outcome.
    Finished(BlockOutcome),
    /// A background command is still producing output.
    BackgroundRunning,
    /// A background command has stopped producing output.
    BackgroundFinished,
    /// Content is not connected to a running shell command.
    Static,
}

/// Stable completion result for a terminal block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockOutcome {
    /// The process exited with the supplied status code.
    Exited(i32),
    /// The process was terminated by the supplied signal number.
    Signaled(i32),
    /// Execution was cancelled before a process outcome was available.
    Cancelled,
    /// The owning shell exited before sending command completion.
    ShellExited,
    /// Completion was recovered without an exact process outcome.
    Unknown,
}

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
    /// Finish one exact command block.
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
    /// A non-handshake event arrived before shell registration.
    MissingShellHello {
        /// Unregistered shell identity.
        shell_instance_id: ShellInstanceId,
    },
}

/// Metadata accumulated by sparse lifecycle patches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMetadata {
    command: Option<String>,
    cwd_before: Option<String>,
    cwd_after: Option<String>,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    exit_code: Option<i32>,
}

impl BlockMetadata {
    /// Return the canonical command text when known.
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    /// Return the working directory captured before execution.
    pub fn cwd_before(&self) -> Option<&str> {
        self.cwd_before.as_deref()
    }

    /// Return the working directory captured after execution.
    pub fn cwd_after(&self) -> Option<&str> {
        self.cwd_after.as_deref()
    }

    /// Return the execution start timestamp.
    pub fn started_at(&self) -> Option<i64> {
        self.started_at
    }

    /// Return the execution finish timestamp.
    pub fn finished_at(&self) -> Option<i64> {
        self.finished_at
    }

    /// Return the process exit code when completion used an exit status.
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    fn patch_prompt(&mut self, cwd: Option<String>) {
        if let Some(cwd) = cwd {
            self.cwd_before = Some(cwd);
        }
    }

    fn patch_start(
        &mut self,
        command: Option<String>,
        cwd: Option<String>,
        started_at: Option<i64>,
    ) {
        if let Some(command) = command {
            self.command = Some(command);
        }
        if let Some(cwd) = cwd {
            self.cwd_before = Some(cwd);
        }
        if let Some(started_at) = started_at {
            self.started_at = Some(started_at);
        }
    }

    fn patch_completion(
        &mut self,
        outcome: &BlockOutcome,
        cwd: Option<String>,
        finished_at: Option<i64>,
    ) {
        if let Some(cwd) = cwd {
            self.cwd_after = Some(cwd);
        }
        if let Some(finished_at) = finished_at {
            self.finished_at = Some(finished_at);
        }
        if let BlockOutcome::Exited(exit_code) = outcome {
            self.exit_code = Some(*exit_code);
        }
    }
}

/// Canonical lifecycle record for one terminal block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRecord {
    id: BlockId,
    shell_instance_id: ShellInstanceId,
    state: BlockState,
    metadata: BlockMetadata,
}

impl BlockRecord {
    /// Return the stable block identity.
    pub fn id(&self) -> &BlockId {
        &self.id
    }

    /// Return the shell context that owns this block.
    pub fn shell_instance_id(&self) -> &ShellInstanceId {
        &self.shell_instance_id
    }

    /// Return the current lifecycle state.
    pub fn state(&self) -> &BlockState {
        &self.state
    }

    /// Return the merged block metadata.
    pub fn metadata(&self) -> &BlockMetadata {
        &self.metadata
    }

    fn prepared(
        id: BlockId,
        shell_instance_id: ShellInstanceId,
        cwd: Option<String>,
    ) -> Self {
        let mut metadata = BlockMetadata::default();
        metadata.patch_prompt(cwd);

        Self {
            id,
            shell_instance_id,
            state: BlockState::BeforeExecution,
            metadata,
        }
    }

    fn executing(
        id: BlockId,
        shell_instance_id: ShellInstanceId,
        command: Option<String>,
        cwd: Option<String>,
        started_at: Option<i64>,
    ) -> Self {
        let mut record = Self::prepared(id, shell_instance_id, None);
        record.metadata.patch_start(command, cwd, started_at);
        record.state = BlockState::Executing;
        record
    }

    fn finish(
        &mut self,
        outcome: BlockOutcome,
        cwd: Option<String>,
        finished_at: Option<i64>,
    ) {
        self.metadata.patch_completion(&outcome, cwd, finished_at);
        self.state = BlockState::Finished(outcome);
    }
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
        let active_block_id = self
            .shells
            .get(&shell_instance_id)
            .and_then(|shell| shell.active_block_id.clone());
        let mut changed_blocks = Vec::new();

        if let Some(active_block_id) = active_block_id
            && active_block_id != block_id
            && let Some(active) = self.blocks.get_mut(&active_block_id)
            && active.state == BlockState::Executing
        {
            active.finish(BlockOutcome::Unknown, None, prepared_at);
            changed_blocks.push(active_block_id.clone());
            self.diagnostics
                .push(LifecycleDiagnostic::SynthesizedCompletion {
                    block_id: active_block_id,
                });
        }

        match self.blocks.get_mut(&block_id) {
            Some(block) if block.state == BlockState::BeforeExecution => {
                block.metadata.patch_prompt(cwd.clone());
                changed_blocks.push(block_id.clone());
            },
            Some(block) => {
                self.diagnostics
                    .push(LifecycleDiagnostic::InvalidTransition {
                        block_id: block_id.clone(),
                        state: block.state.clone(),
                    });
                return LifecycleUpdate::ignored();
            },
            None => {
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
        match self.blocks.get_mut(&block_id) {
            Some(block) if block.state == BlockState::BeforeExecution => {
                block.metadata.patch_start(command, cwd.clone(), started_at);
                block.state = BlockState::Executing;
            },
            Some(block) if block.state == BlockState::Executing => {
                return LifecycleUpdate::ignored();
            },
            Some(block) => {
                self.diagnostics
                    .push(LifecycleDiagnostic::InvalidTransition {
                        block_id,
                        state: block.state.clone(),
                    });
                return LifecycleUpdate::ignored();
            },
            None => {
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

        LifecycleUpdate::changed(block_id)
    }

    fn apply_command_end(
        &mut self,
        shell_instance_id: &ShellInstanceId,
        block_id: BlockId,
        outcome: BlockOutcome,
        cwd: Option<String>,
        finished_at: Option<i64>,
    ) -> LifecycleUpdate {
        let Some(block) = self.blocks.get_mut(&block_id) else {
            self.diagnostics
                .push(LifecycleDiagnostic::OrphanCommandEnd { block_id });
            return LifecycleUpdate::ignored();
        };

        match block.state {
            BlockState::Executing | BlockState::BackgroundRunning => {
                block.finish(outcome, cwd.clone(), finished_at);
            },
            BlockState::Finished(_) | BlockState::BackgroundFinished => {
                return LifecycleUpdate::ignored();
            },
            _ => {
                self.diagnostics
                    .push(LifecycleDiagnostic::InvalidTransition {
                        block_id,
                        state: block.state.clone(),
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

        if !matches!(block.state, BlockState::Finished(_)) {
            block.finish(BlockOutcome::ShellExited, None, finished_at);
            return LifecycleUpdate::changed(active_block_id);
        }

        LifecycleUpdate::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlockOutcome, BlockState, DegradedReason, IntegrationStatus,
        LifecycleDiagnostic, LifecycleEvent, LifecycleInput, LifecycleReducer,
    };
    use crate::block::id::{
        BlockId, ProtocolSequence, ShellInstanceId, TerminalSessionId,
    };

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
}
