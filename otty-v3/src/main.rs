// Allow dead_code during incremental build: views, models, and accessors are
// defined ahead of full view composition (wired in Phase 4/5).
#![allow(dead_code)]

mod app;
mod components;
mod events;
mod fonts;
mod geometry;
mod guards;
mod icons;
mod layout;
mod state;
mod style;
mod theme;
mod widgets;

use env_logger::Env;
use iced::{Size, window};
use image::ImageFormat;

use crate::app::{App, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};
use crate::fonts::TERM_FONT_JET_BRAINS_BYTES;
use crate::icons::APP_ICON_DATA;

fn main() -> iced::Result {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    iced::application(App::new, App::update, App::view)
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
        .font(TERM_FONT_JET_BRAINS_BYTES)
        .subscription(App::subscription)
        .run()
}
