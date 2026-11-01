//! `TargetDot`: small round mark that tells where a session runs.

use iced::widget::{Space, container};
use iced::{Border, Color, Element};

use crate::palette::UiPalette;
use crate::size;

/// Where a session or tab runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// This computer.
    Local,
    /// A remote host reached over SSH.
    Ssh,
    /// A protected target such as production.
    Prod,
    /// A non-protected cluster such as staging.
    Staging,
}

impl TargetKind {
    /// Returns the `target-*` color of this kind in `palette`.
    pub const fn color(self, palette: &UiPalette) -> Color {
        match self {
            Self::Local => palette.target_local(),
            Self::Ssh => palette.target_ssh(),
            Self::Prod => palette.target_prod(),
            Self::Staging => palette.target_staging(),
        }
    }
}

/// Renders a round dot in the `target-*` color of `kind`.
///
/// Never show the dot alone: color is not enough to tell targets apart, so
/// always put the target name next to it.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::target_dot::{TargetKind, target_dot};
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let dot: Element<'_, ()> = target_dot(TargetKind::Ssh, &palette);
/// ```
pub fn target_dot<'a, Message: 'a>(
    kind: TargetKind,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let color = kind.color(palette);

    container(Space::new())
        .width(size::DOT)
        .height(size::DOT)
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: Border {
                radius: (size::DOT / 2.0).into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contrast::contrast_ratio;

    /// WCAG 2.1 AA minimum for non-text marks.
    const NON_TEXT_MIN: f32 = 3.0;
    const KINDS: [TargetKind; 4] = [
        TargetKind::Local,
        TargetKind::Ssh,
        TargetKind::Prod,
        TargetKind::Staging,
    ];

    fn assert_dots_readable(name: &str, palette: &UiPalette) {
        let surfaces = [
            ("surface-chrome", palette.surface_chrome()),
            ("surface-panel", palette.surface_panel()),
            ("surface-raised", palette.surface_raised()),
        ];

        for kind in KINDS {
            for (surface_name, surface) in surfaces {
                let ratio = contrast_ratio(kind.color(palette), surface);

                assert!(
                    ratio >= NON_TEXT_MIN,
                    "{name}: {kind:?} on {surface_name} is {ratio:.2}:1"
                );
            }
        }
    }

    #[test]
    fn given_otty_dark_when_dots_drawn_then_each_reaches_3_to_1_on_surfaces() {
        assert_dots_readable("OTTY Dark", &UiPalette::otty_dark());
    }

    #[test]
    fn given_otty_light_when_dots_drawn_then_each_reaches_3_to_1_on_surfaces() {
        assert_dots_readable("OTTY Light", &UiPalette::otty_light());
    }

    #[test]
    fn given_each_kind_when_color_read_then_it_is_its_target_token() {
        let palette = UiPalette::otty_dark();

        assert_eq!(TargetKind::Local.color(&palette), palette.target_local());
        assert_eq!(TargetKind::Ssh.color(&palette), palette.target_ssh());
        assert_eq!(TargetKind::Prod.color(&palette), palette.target_prod());
        assert_eq!(
            TargetKind::Staging.color(&palette),
            palette.target_staging()
        );
    }
}
