use otty_libterm::TerminalSize;
use otty_libterm::surface::{Column, Line, Point};

use crate::ConfigError;

/// Measured logical-pixel dimensions of one terminal cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellMetrics {
    width: f32,
    height: f32,
}

impl CellMetrics {
    /// Cell width in logical pixels.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Cell height in logical pixels.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Create validated cell metrics.
    pub fn try_new(width: f32, height: f32) -> Result<Self, ConfigError> {
        if !width.is_finite()
            || width <= 0.0
            || !height.is_finite()
            || height <= 0.0
        {
            return Err(ConfigError::InvalidCellMetrics);
        }

        Ok(Self { width, height })
    }

    /// Snap both dimensions to the device-pixel grid.
    pub fn snapped(&self, scale_factor: f32) -> Self {
        if !scale_factor.is_finite() || scale_factor <= 0.0 {
            return *self;
        }

        Self {
            width: snap_metric(self.width, scale_factor),
            height: snap_metric(self.height, scale_factor),
        }
    }
}

fn snap_metric(value: f32, scale_factor: f32) -> f32 {
    let snapped = (value * scale_factor).round().max(1.0) / scale_factor;

    if snapped.is_finite() && snapped > 0.0 {
        snapped
    } else {
        value
    }
}

/// Widget bounds and framing used to derive terminal grid coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalGeometry {
    width: f32,
    height: f32,
    padding: f32,
    border_width: f32,
}

impl TerminalGeometry {
    /// Describe allocated widget bounds in logical pixels.
    pub fn new(
        width: f32,
        height: f32,
        padding: f32,
        border_width: f32,
    ) -> Self {
        Self {
            width: width.max(0.0),
            height: height.max(0.0),
            padding: padding.max(0.0),
            border_width: border_width.max(0.0),
        }
    }

    /// Compute a safe PTY size for the inner content area.
    pub fn terminal_size(&self, metrics: CellMetrics) -> TerminalSize {
        let horizontal_frame = 2.0 * (self.padding + self.border_width);
        let vertical_frame = 2.0 * (self.padding + self.border_width);
        let inner_width = (self.width - horizontal_frame).max(metrics.width);
        let inner_height = (self.height - vertical_frame).max(metrics.height);
        let cols = (inner_width / metrics.width).floor().max(1.0) as u16;
        let rows = (inner_height / metrics.height).floor().max(1.0) as u16;

        TerminalSize {
            cell_width: metrics.width.round().clamp(1.0, u16::MAX as f32)
                as u16,
            cell_height: metrics.height.round().clamp(1.0, u16::MAX as f32)
                as u16,
            cols,
            rows,
        }
    }

    /// Convert a pointer position in widget coordinates to a surface point.
    pub fn point_to_grid(
        &self,
        x: f32,
        y: f32,
        metrics: CellMetrics,
        display_offset: usize,
    ) -> Point {
        let size = self.terminal_size(metrics);
        let origin = self.padding + self.border_width;
        let column = ((x - origin).max(0.0) / metrics.width).floor() as usize;
        let row = ((y - origin).max(0.0) / metrics.height).floor() as usize;
        let column = column.min(usize::from(size.cols) - 1);
        let row = row.min(usize::from(size.rows) - 1);

        Point::new(Line(row as i32 - display_offset as i32), Column(column))
    }
}
