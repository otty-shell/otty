//! `otty`: the window frame of the OTTY app and everything that links
//! the features together.
//!
//! The frame is the header and the side panels around the main area. The
//! features (modes, tabs, terminal, containers, ...) fill its places; the
//! active mode decides which feature fills each one. Each feature is wired
//! here by the issue that moves it, see
//! `specs/2026-11-10-feature-crates/spec.md`.

mod app;
mod dispatch;
mod frame;
mod guards;
mod keys;
mod language;
mod layout;
mod message;
mod screens;
mod startup;
mod subscription;
mod terminal_settings;
#[cfg(test)]
mod testing;
mod update;
mod view;

use env_logger::Env;
use iced::{Size, window};
use image::ImageFormat;
use otty_app_theme::icons::APP_ICON_DATA;

use crate::app::{App, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};

/// Id of the app on Linux: the name of its `.desktop` and AppStream files
/// in `assets/packages/linux`. The desktop matches the window to the
/// `.desktop` file by this id, to show the OTTY name and icon in the taskbar.
#[cfg(target_os = "linux")]
const LINUX_APP_ID: &str = "run.otty.Otty";

fn main() -> iced::Result {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let app = iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .antialiasing(true)
        .window(window::Settings {
            decorations: false,
            min_size: Some(Size {
                width: MIN_WINDOW_WIDTH,
                height: MIN_WINDOW_HEIGHT,
            }),
            icon: window::icon::from_file_data(
                APP_ICON_DATA,
                Some(ImageFormat::Png),
            )
            .ok(),
            #[cfg(target_os = "linux")]
            platform_specific: window::settings::PlatformSpecific {
                application_id: LINUX_APP_ID.to_owned(),
                ..Default::default()
            },
            ..window::Settings::default()
        })
        .resizable(true)
        .subscription(App::subscription);

    otty_ui_kit::fonts::register_variable_cjk_faces();

    otty_ui_kit::fonts::all_bytes()
        .into_iter()
        .fold(app, |app, bytes| app.font(bytes))
        .run()
}
