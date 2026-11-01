//! Render a screen in OTTY Dark and OTTY Light and write the PNG files.

use std::fs;
use std::path::PathBuf;

use iced::Point;
use iced_test::Simulator;
use otty_ui_kit::fonts;

use super::fixture::Fixture;
use super::window::{Screen, WINDOW};
use crate::theme::{AppTheme, BuiltinTheme, ThemeProps};

/// Render `build` in both themes and write the PNG files.
pub(super) fn shoot(
    screen: &str,
    build: impl for<'a> Fn(&'a Fixture, ThemeProps<'a>) -> Screen<'a>,
) {
    shoot_at(screen, None, build);
}

/// Render `build` in both themes with the pointer at `cursor`, if any, and
/// write the PNG files.
pub(super) fn shoot_at(
    screen: &str,
    cursor: Option<Point>,
    build: impl for<'a> Fn(&'a Fixture, ThemeProps<'a>) -> Screen<'a>,
) {
    let fixture = Fixture::new();

    for (suffix, builtin) in [
        ("dark", BuiltinTheme::OttyDark),
        ("light", BuiltinTheme::OttyLight),
    ] {
        let app_theme = AppTheme::new(builtin);
        let iced_theme = iced::Theme::from(&app_theme);
        let element = build(&fixture, ThemeProps::new(&app_theme));

        let settings = iced::Settings {
            fonts: fonts::all_bytes()
                .into_iter()
                .map(std::borrow::Cow::Borrowed)
                .collect(),
            default_font: fonts::UI,
            ..iced::Settings::default()
        };
        let mut ui = Simulator::with_size(settings, WINDOW, element);
        if let Some(cursor) = cursor {
            ui.point_at(cursor);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::CursorMoved { position: cursor },
            )]);
        }
        let snapshot = ui.snapshot(&iced_theme).expect("snapshot should draw");

        save(&snapshot, &format!("{screen}-{suffix}"));
    }
}

/// Write `snapshot` as `<name>.png` in the screenshot folder.
fn save(snapshot: &iced_test::simulator::Snapshot, name: &str) {
    let folder = std::env::var_os("OTTY_SCREENSHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("otty-screens"));
    let stage = folder.join(format!(".stage-{name}"));
    let _ = fs::remove_dir_all(&stage);
    fs::create_dir_all(&stage).expect("screenshot folder should be writable");

    // `matches_image` names the file after the renderer, so it goes to a
    // stage folder first and is renamed from there.
    snapshot
        .matches_image(stage.join(name))
        .expect("PNG should be written");
    let written = fs::read_dir(&stage)
        .expect("stage folder should be readable")
        .next()
        .expect("one PNG should be written")
        .expect("entry should be readable")
        .path();
    fs::rename(written, folder.join(format!("{name}.png")))
        .expect("PNG should be moved");
    let _ = fs::remove_dir_all(&stage);
}
