//! WCAG 2.1 contrast ratio between two colors.
//!
//! Only the palette tests use it today, so the module is compiled for tests
//! only.

use iced::Color;

/// Returns the WCAG 2.1 contrast ratio of two opaque colors, from 1 (same
/// color) to 21 (black on white). The order of the colors does not matter.
pub(crate) fn contrast_ratio(a: Color, b: Color) -> f32 {
    let a = relative_luminance(a);
    let b = relative_luminance(b);

    let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };

    (lighter + 0.05) / (darker + 0.05)
}

/// WCAG 2.1 relative luminance: how bright the color looks, from 0 to 1.
fn relative_luminance(color: Color) -> f32 {
    0.2126 * linear_channel(color.r)
        + 0.7152 * linear_channel(color.g)
        + 0.0722 * linear_channel(color.b)
}

/// Converts one sRGB channel (0 to 1) to linear light.
fn linear_channel(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use iced::Color;

    use super::contrast_ratio;

    const TOLERANCE: f32 = 0.001;

    #[test]
    fn given_black_and_white_when_contrast_computed_then_ratio_is_21() {
        let ratio = contrast_ratio(Color::BLACK, Color::WHITE);

        assert!((ratio - 21.0).abs() < TOLERANCE, "ratio was {ratio}");
    }

    #[test]
    fn given_equal_colors_when_contrast_computed_then_ratio_is_1() {
        let color = Color::from_rgb8(0x4f, 0xa6, 0xed);

        let ratio = contrast_ratio(color, color);

        assert!((ratio - 1.0).abs() < TOLERANCE, "ratio was {ratio}");
    }

    #[test]
    fn given_swapped_colors_when_contrast_computed_then_ratio_is_the_same() {
        let dark = Color::from_rgb8(0x0f, 0x11, 0x15);
        let light = Color::from_rgb8(0xec, 0xef, 0xf4);

        let forward = contrast_ratio(light, dark);
        let backward = contrast_ratio(dark, light);

        assert!((forward - backward).abs() < TOLERANCE);
        assert!((forward - 16.39).abs() < 0.01, "ratio was {forward}");
    }
}
