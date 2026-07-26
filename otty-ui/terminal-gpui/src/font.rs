use gpui::{
    Font, FontFallbacks, FontFeatures, FontStyle, FontWeight, SharedString,
};

use crate::ConfigError;

/// Validated font settings used for terminal shaping and grid metrics.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalFont {
    family: SharedString,
    fallbacks: Vec<SharedString>,
    size: f32,
    line_height: f32,
    weight: FontWeight,
    style: FontStyle,
}

impl TerminalFont {
    /// Primary font family.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Ordered fallback font families.
    pub fn fallbacks(&self) -> &[SharedString] {
        &self.fallbacks
    }

    /// Font size in logical pixels.
    pub fn size(&self) -> f32 {
        self.size
    }

    /// Line height relative to the font size.
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Font weight used for ordinary cells.
    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    /// Font style used for ordinary cells.
    pub fn style(&self) -> FontStyle {
        self.style
    }

    /// Return the GPUI font descriptor for ordinary cells.
    pub fn gpui_font(&self) -> Font {
        Font {
            family: self.family.clone(),
            features: FontFeatures::disable_ligatures(),
            fallbacks: (!self.fallbacks.is_empty()).then(|| {
                FontFallbacks::from_fonts(
                    self.fallbacks.iter().map(ToString::to_string).collect(),
                )
            }),
            weight: self.weight,
            style: self.style,
        }
    }

    /// Construct a monospace font with the default terminal line height.
    pub fn monospace(size: f32) -> Result<Self, ConfigError> {
        Self::try_new("monospace", std::iter::empty::<&str>(), size, 1.2)
    }

    /// Construct fully specified and validated font settings.
    pub fn try_new<I, S>(
        family: impl Into<SharedString>,
        fallbacks: I,
        size: f32,
        line_height: f32,
    ) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        if !size.is_finite() || size <= 0.0 {
            return Err(ConfigError::InvalidFontSize);
        }
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(ConfigError::InvalidLineHeight);
        }

        Ok(Self {
            family: family.into(),
            fallbacks: fallbacks.into_iter().map(Into::into).collect(),
            size,
            line_height,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        })
    }

    /// Override the ordinary cell weight.
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Override the ordinary cell style.
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }
}

impl Default for TerminalFont {
    fn default() -> Self {
        Self::monospace(14.0).expect("default terminal font is valid")
    }
}
