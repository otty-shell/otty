//! The files of a new widget, as in «Пример виджета» of
//! `docs/internal/architecture/app/widgets.md`, but without content of their own: the
//! intents and effects are empty enums, so the reducer and the route have
//! nothing to match yet.
//!
//! In the templates `$name` stands for the snake_case name, `$Name` for the
//! CamelCase name and `$words` for the name as words.

use super::FileWrite;
use super::name::WidgetName;

const MOD_RS: &str = "\
pub(crate) mod event;
mod model;
mod reducer;
mod state;
pub(crate) mod view;

use event::{$NameEvent, $NameIntent};
use iced::Task;
use model::$NameViewModel;
use state::$NameState;

/// The $words widget: holds its state and lets the other layers read and
/// change it.
pub(crate) struct $NameWidget {
    state: $NameState,
}

impl $NameWidget {
    /// Create the widget with its starting state.
    pub(crate) fn new() -> Self {
        Self {
            state: $NameState::default(),
        }
    }

    /// Apply an intent to the state and return the effects it asks for.
    pub(crate) fn reduce(
        &mut self,
        intent: $NameIntent,
    ) -> Task<$NameEvent> {
        reducer::reduce(&mut self.state, intent)
    }

    /// Return what the $words panel shows.
    pub(crate) fn vm(&self) -> $NameViewModel {
        $NameViewModel::from_state(&self.state)
    }
}
";

const EVENT_RS: &str = "\
/// What the $words widget is asked to do.
#[derive(Debug, Clone)]
pub(crate) enum $NameIntent {}

/// What the $words reducer asks its route to do.
#[derive(Debug, Clone)]
pub(crate) enum $NameEffect {}

/// An event of the $words widget as the app loop carries it.
#[derive(Debug, Clone)]
pub(crate) enum $NameEvent {
    /// An intent for the reducer.
    Intent($NameIntent),
    /// An effect for the route.
    Effect($NameEffect),
}
";

const STATE_RS: &str = "\
/// The $words data and the rules that keep it valid.
#[derive(Debug, Default)]
pub(super) struct $NameState {}
";

const REDUCER_RS: &str = "\
#![forbid(clippy::disallowed_methods)]

use iced::Task;

use super::event::{$NameEvent, $NameIntent};
use super::state::$NameState;

/// Apply an intent to the state and return the effects it asks for.
pub(super) fn reduce(
    _state: &mut $NameState,
    intent: $NameIntent,
) -> Task<$NameEvent> {
    match intent {}
}
";

const MODEL_RS: &str = "\
use super::state::$NameState;

/// What the $words panel shows.
#[derive(Debug, Clone, Copy)]
pub(crate) struct $NameViewModel {}

impl $NameViewModel {
    /// Build the view model from the widget state.
    pub(super) fn from_state(_state: &$NameState) -> Self {
        Self {}
    }
}
";

const VIEW_MOD_RS: &str = "\
pub(crate) mod $name_panel;
";

const PANEL_RS: &str = "\
use iced::Element;
use iced::widget::column;

use super::super::event::$NameIntent;
use super::super::model::$NameViewModel;
use crate::theme::ThemeProps;

/// Props for the $words panel.
pub(crate) struct $NamePanelProps<'a> {
    pub(crate) vm: $NameViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the $words panel.
pub(crate) fn view(
    _props: $NamePanelProps<'_>,
) -> Element<'_, $NameIntent> {
    column![].into()
}
";

const ROUTE_RS: &str = "\
use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::$name::event::{$NameEffect, $NameEvent};

/// Reduce the intents of the $words widget and run its effects.
pub(crate) fn route(app: &mut App, event: $NameEvent) -> Task<AppEvent> {
    match event {
        $NameEvent::Intent(intent) => {
            app.widgets.$name.reduce(intent).map(AppEvent::$Name)
        },
        $NameEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(effect: $NameEffect) -> Task<AppEvent> {
    match effect {}
}
";

/// The new files: the path relative to `app/src` and the contents.
const FILES: &[(&str, &str)] = &[
    ("widgets/$name/mod.rs", MOD_RS),
    ("widgets/$name/event.rs", EVENT_RS),
    ("widgets/$name/state.rs", STATE_RS),
    ("widgets/$name/reducer.rs", REDUCER_RS),
    ("widgets/$name/model.rs", MODEL_RS),
    ("widgets/$name/view/mod.rs", VIEW_MOD_RS),
    ("widgets/$name/view/$name_panel.rs", PANEL_RS),
    ("router/$name.rs", ROUTE_RS),
];

/// The new files of the widget `name`.
pub(super) fn new_files(name: &WidgetName) -> Vec<FileWrite> {
    FILES
        .iter()
        .map(|(path, contents)| FileWrite {
            path: filled(path, name),
            contents: filled(contents, name),
        })
        .collect()
}

/// `template` with its placeholders replaced by the forms of `name`.
fn filled(template: &str, name: &WidgetName) -> String {
    template
        .replace("$Name", name.camel())
        .replace("$name", name.snake())
        .replace("$words", name.words())
}
