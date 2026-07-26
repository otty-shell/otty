use crate::ConfigError;

/// Framing and spacing used when embedding a terminal in a parent view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalAppearance {
    padding: f32,
    border_width: f32,
    corner_radius: f32,
    focused_border_width: f32,
}

impl TerminalAppearance {
    /// Inner spacing around the terminal grid.
    pub fn padding(&self) -> f32 {
        self.padding
    }

    /// Border width when the widget is not focused.
    pub fn border_width(&self) -> f32 {
        self.border_width
    }

    /// Radius applied to the widget frame.
    pub fn corner_radius(&self) -> f32 {
        self.corner_radius
    }

    /// Border width when the widget owns focus.
    pub fn focused_border_width(&self) -> f32 {
        self.focused_border_width
    }

    /// Build validated widget geometry in logical pixels.
    pub fn try_new(
        padding: f32,
        border_width: f32,
        corner_radius: f32,
        focused_border_width: f32,
    ) -> Result<Self, ConfigError> {
        if !padding.is_finite() || padding < 0.0 {
            return Err(ConfigError::InvalidPadding);
        }
        if !border_width.is_finite()
            || border_width < 0.0
            || !focused_border_width.is_finite()
            || focused_border_width < 0.0
        {
            return Err(ConfigError::InvalidBorderWidth);
        }
        if !corner_radius.is_finite() || corner_radius < 0.0 {
            return Err(ConfigError::InvalidCornerRadius);
        }

        Ok(Self {
            padding,
            border_width,
            corner_radius,
            focused_border_width,
        })
    }
}

impl Default for TerminalAppearance {
    fn default() -> Self {
        Self {
            padding: 9.0,
            border_width: 1.0,
            corner_radius: 8.0,
            focused_border_width: 2.0,
        }
    }
}
