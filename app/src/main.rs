// Allow dead_code during incremental build: views, models, and accessors are
// defined ahead of full view composition (wired in Phase 4/5).
#![allow(dead_code)]

mod app;
mod components;
mod domain;
mod events;
mod fonts;
mod geometry;
mod guards;
mod i18n;
mod icons;
mod layout;
mod services;
mod shell_integration;
mod shortcuts;
mod state;
#[cfg(test)]
mod testing;
mod theme;
mod widgets;

use env_logger::Env;
use iced::{Size, window};
use image::ImageFormat;

use crate::app::{App, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};
use crate::icons::APP_ICON_DATA;

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

    otty_ui_kit::fonts::all_bytes()
        .into_iter()
        .fold(app, |app, bytes| app.font(bytes))
        .run()
}
