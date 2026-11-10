//! `otty-app`: the window frame of the OTTY app and everything that links
//! the features together.
//!
//! The frame is the header, the side panels around the main area, the status
//! bar and the mode switch. The features (tabs, terminal, containers, ...)
//! fill its places; each one is wired here by the issue that moves it, see
//! `specs/2026-11-10-feature-crates/spec.md`.

mod app;
mod chrome;
mod dispatch;
mod effects;
mod guards;
mod keys;
mod layout;
mod message;
mod modes;
mod screens;
mod sidebar;
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
