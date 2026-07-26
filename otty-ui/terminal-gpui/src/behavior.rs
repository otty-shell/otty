use crate::ConfigError;

/// Policy used when activating a terminal hyperlink.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LinkPolicy {
    /// Ask the host to route the link.
    #[default]
    EmitOnly,
    /// Open the URI and also notify the host.
    OpenAndEmit,
    /// Ignore link activation.
    Disabled,
}

/// Policy used when the terminal requests an audible bell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BellPolicy {
    /// Notify both the system and the host.
    SystemAndEmit,
    /// Notify only the host.
    #[default]
    EmitOnly,
    /// Ignore bell requests.
    Disabled,
}

/// Policy used for terminal context-menu requests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ContextMenuPolicy {
    /// Emit a host-facing request.
    #[default]
    Emit,
    /// Disable context-menu requests.
    Disabled,
}

/// Determines whether block chrome is painted by the widget or its host.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BlockUiMode {
    /// Paint block highlights and dividers inside the terminal.
    #[default]
    Internal,
    /// Expose geometry for an overlay owned by the host.
    ExternalOverlay,
}

/// Input and host-integration policies for one terminal entity.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalBehavior {
    focus_on_click: bool,
    copy_on_select: bool,
    middle_click_paste: bool,
    scroll_multiplier: f32,
    link_policy: LinkPolicy,
    bell_policy: BellPolicy,
    context_menu_policy: ContextMenuPolicy,
    block_ui_mode: BlockUiMode,
}

impl TerminalBehavior {
    /// Whether a primary click should focus the terminal.
    pub fn focus_on_click(&self) -> bool {
        self.focus_on_click
    }

    /// Whether completing a selection copies it immediately.
    pub fn copy_on_select(&self) -> bool {
        self.copy_on_select
    }

    /// Whether middle click pastes the native clipboard.
    pub fn middle_click_paste(&self) -> bool {
        self.middle_click_paste
    }

    /// Multiplier applied to line and pixel scroll input.
    pub fn scroll_multiplier(&self) -> f32 {
        self.scroll_multiplier
    }

    /// Active hyperlink policy.
    pub fn link_policy(&self) -> LinkPolicy {
        self.link_policy
    }

    /// Active bell policy.
    pub fn bell_policy(&self) -> BellPolicy {
        self.bell_policy
    }

    /// Active context-menu policy.
    pub fn context_menu_policy(&self) -> ContextMenuPolicy {
        self.context_menu_policy
    }

    /// Active block rendering mode.
    pub fn block_ui_mode(&self) -> BlockUiMode {
        self.block_ui_mode
    }

    /// Create behavior settings with a validated scroll multiplier.
    pub fn try_new(scroll_multiplier: f32) -> Result<Self, ConfigError> {
        if !scroll_multiplier.is_finite() || scroll_multiplier <= 0.0 {
            return Err(ConfigError::InvalidScrollMultiplier);
        }

        Ok(Self {
            scroll_multiplier,
            ..Self::default()
        })
    }

    /// Change focus-on-click behavior.
    pub fn with_focus_on_click(mut self, enabled: bool) -> Self {
        self.focus_on_click = enabled;
        self
    }

    /// Change copy-on-select behavior.
    pub fn with_copy_on_select(mut self, enabled: bool) -> Self {
        self.copy_on_select = enabled;
        self
    }

    /// Change middle-click paste behavior.
    pub fn with_middle_click_paste(mut self, enabled: bool) -> Self {
        self.middle_click_paste = enabled;
        self
    }

    /// Change the line/pixel scroll multiplier after validation.
    pub fn with_scroll_multiplier(
        mut self,
        multiplier: f32,
    ) -> Result<Self, ConfigError> {
        if !multiplier.is_finite() || multiplier <= 0.0 {
            return Err(ConfigError::InvalidScrollMultiplier);
        }

        self.scroll_multiplier = multiplier;
        Ok(self)
    }

    /// Change hyperlink routing.
    pub fn with_link_policy(mut self, policy: LinkPolicy) -> Self {
        self.link_policy = policy;
        self
    }

    /// Change bell routing.
    pub fn with_bell_policy(mut self, policy: BellPolicy) -> Self {
        self.bell_policy = policy;
        self
    }

    /// Change context-menu routing.
    pub fn with_context_menu_policy(
        mut self,
        policy: ContextMenuPolicy,
    ) -> Self {
        self.context_menu_policy = policy;
        self
    }

    /// Change block UI ownership.
    pub fn with_block_ui_mode(mut self, mode: BlockUiMode) -> Self {
        self.block_ui_mode = mode;
        self
    }
}

impl Default for TerminalBehavior {
    fn default() -> Self {
        Self {
            focus_on_click: true,
            copy_on_select: false,
            middle_click_paste: true,
            scroll_multiplier: 1.0,
            link_policy: LinkPolicy::default(),
            bell_policy: BellPolicy::default(),
            context_menu_policy: ContextMenuPolicy::default(),
            block_ui_mode: BlockUiMode::default(),
        }
    }
}
