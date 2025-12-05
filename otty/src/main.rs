mod main_window;
mod sidebar;
mod tab_bar;
mod tab_button;
mod theme;

use iced::Size;
use main_window::App;

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .resizable(true)
        .subscription(App::subscription)
        .run_with(App::new)
}
