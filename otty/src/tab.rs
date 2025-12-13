use std::collections::HashMap;

use iced::widget::pane_grid::{self, Highlight, Line, PaneGrid};
use iced::widget::{Stack, container, mouse_area};
use iced::{Border, Element, Length, Point, Size, Task, Theme};
use otty_ui_term::settings::Settings;
use otty_ui_term::{BlockCommand, SurfaceMode, TerminalView};

use crate::context_menu::{self, PaneContextMenu};
use crate::main_window::Event;
use crate::theme::IcedColorPalette;

#[derive(Clone, Debug)]
pub struct TabBlockSelection {
    pub terminal_id: u64,
    pub block_id: String,
}

pub struct Tab {
    pub id: u64,
    pub title: String,
    pub panes: pane_grid::State<TabPane>,
    pub terminals: HashMap<u64, TerminalEntry>,
    pub focus: Option<pane_grid::Pane>,
    pub context_menu: Option<PaneContextMenu>,
    selected_block: Option<TabBlockSelection>,
    default_title: String,
    grid_cursor: Option<Point>,
    grid_size: Size,
    theme: IcedColorPalette,
}

#[derive(Debug, Clone)]
pub struct TabPane {
    pub terminal_id: u64,
}

pub struct TerminalEntry {
    pub pane: pane_grid::Pane,
    pub terminal: otty_ui_term::Terminal,
    pub title: String,
}

#[derive(Default)]
pub struct TabAction {
    pub close_tab: bool,
    pub task: Option<Task<Event>>,
}

impl TabAction {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_task(task: Task<Event>) -> Self {
        Self {
            close_tab: false,
            task: Some(task),
        }
    }

    pub fn close_tab() -> Self {
        Self {
            close_tab: true,
            task: None,
        }
    }

    pub fn merge_task(&mut self, task: Task<Event>) {
        self.task = Some(match self.task.take() {
            Some(existing) => Task::batch(vec![existing, task]),
            None => task,
        });
    }
}

impl Tab {
    pub fn new(
        id: u64,
        default_title: String,
        terminal_id: u64,
        settings: &Settings,
        theme: &IcedColorPalette,
    ) -> (Self, Task<Event>) {
        let terminal =
            otty_ui_term::Terminal::new(terminal_id, settings.clone())
                .expect("failed to create the new terminal instance");
        let widget_id = terminal.widget_id().clone();

        let (panes, initial_pane) =
            pane_grid::State::new(TabPane { terminal_id });

        let mut terminals = HashMap::new();
        terminals.insert(
            terminal_id,
            TerminalEntry {
                pane: initial_pane,
                terminal,
                title: default_title.clone(),
            },
        );

        let tab = Tab {
            id,
            title: default_title.clone(),
            panes,
            terminals,
            focus: Some(initial_pane),
            context_menu: None,
            selected_block: None,
            default_title,
            grid_cursor: None,
            grid_size: Size::ZERO,
            theme: theme.clone(),
        };

        (tab, TerminalView::focus(widget_id))
    }

    pub fn contains_terminal(&self, id: u64) -> bool {
        self.terminals.contains_key(&id)
    }

    pub fn pane_terminal_id(&self, pane: pane_grid::Pane) -> Option<u64> {
        self.panes.get(pane).map(|pane| pane.terminal_id)
    }

    pub fn focused_terminal_id(&self) -> Option<u64> {
        let pane = self.focus?;
        self.pane_terminal_id(pane)
    }

    pub fn focused_terminal_entry(&self) -> Option<&TerminalEntry> {
        let terminal_id = self.focused_terminal_id()?;
        self.terminals.get(&terminal_id)
    }

    pub fn terminal_entry_mut(
        &mut self,
        terminal_id: u64,
    ) -> Option<&mut TerminalEntry> {
        self.terminals.get_mut(&terminal_id)
    }

    pub fn selected_block_terminal(&self) -> Option<u64> {
        self.selected_block.as_ref().map(|sel| sel.terminal_id)
    }

    pub fn selected_block(&self) -> Option<&TabBlockSelection> {
        self.selected_block.as_ref()
    }

    pub fn set_selected_block(&mut self, terminal_id: u64, block_id: String) {
        self.selected_block = Some(TabBlockSelection {
            terminal_id,
            block_id,
        });
    }

    pub fn clear_selected_block(&mut self) {
        self.selected_block = None;
    }

    pub fn clear_selected_block_for_terminal(&mut self, terminal_id: u64) {
        if self
            .selected_block
            .as_ref()
            .map(|sel| sel.terminal_id == terminal_id)
            .unwrap_or(false)
        {
            self.selected_block = None;
        }
    }

    pub fn view_with_selection(
        &self,
        selected_block_terminal: Option<u64>,
    ) -> Element<'_, Event, Theme, iced::Renderer> {
        let tab_id = self.id;
        let tab_ref = self;

        let pane_grid =
            PaneGrid::new(&self.panes, move |pane, pane_state, _| {
                let is_focused = tab_ref.focus == Some(pane);
                let content =
                    tab_ref.view_single_pane(pane, pane_state, is_focused);

                pane_grid::Content::new(content)
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(1.0)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                let mut separator = palette.background.weak.text;
                separator.a = 0.25;

                pane_grid::Style {
                    hovered_region: Highlight {
                        background: separator.into(),
                        border: Border::default(),
                    },
                    picked_split: Line {
                        color: separator,
                        width: 1.0,
                    },
                    hovered_split: Line {
                        color: separator,
                        width: 1.0,
                    },
                }
            })
            .on_resize(12.0, move |event| Event::PaneResized { tab_id, event });

        let pane_grid = container(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                let mut separator = palette.background.weak.text;
                separator.a = 0.25;

                iced::widget::container::Style {
                    background: Some(separator.into()),
                    ..Default::default()
                }
            })
            .into();

        let mut layers = vec![pane_grid];

        if let Some(menu) = &self.context_menu {
            let selection = if selected_block_terminal == Some(menu.terminal_id)
            {
                self.selected_block().cloned()
            } else {
                None
            };
            layers.push(context_menu::overlay(tab_id, menu, selection));
        }

        let stack_widget = Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill);

        mouse_area(stack_widget)
            .on_move(move |position| Event::PaneGridCursorMoved {
                tab_id,
                position,
            })
            .into()
    }

    pub fn split_pane(
        &mut self,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
        terminal_id: u64,
        settings: &Settings,
    ) -> TabAction {
        let split = self.panes.split(axis, pane, TabPane { terminal_id });

        if let Some((new_pane, _)) = split {
            let terminal =
                otty_ui_term::Terminal::new(terminal_id, settings.clone())
                    .expect("failed to create the new terminal instance");
            let widget_id = terminal.widget_id().clone();

            self.terminals.insert(
                terminal_id,
                TerminalEntry {
                    pane: new_pane,
                    terminal,
                    title: self.default_title.clone(),
                },
            );
            self.focus = Some(new_pane);
            self.context_menu = None;
            self.update_title_from_terminal(Some(terminal_id));

            return TabAction::with_task(TerminalView::focus(widget_id));
        }

        TabAction::none()
    }

    pub fn close_pane(&mut self, pane: pane_grid::Pane) -> TabAction {
        if self.panes.len() == 1 {
            return TabAction::close_tab();
        }

        let result = self.panes.close(pane);
        if let Some((pane_state, sibling)) = result {
            let terminal_id = pane_state.terminal_id;
            self.clear_selected_block_for_terminal(terminal_id);
            self.context_menu = None;
            self.terminals.remove(&terminal_id);

            let needs_focus = self.focus == Some(pane) || self.focus.is_none();
            if needs_focus {
                self.focus = Some(sibling);
                if let Some(next_id) = self.pane_terminal_id(sibling) {
                    if let Some(entry) = self.terminals.get(&next_id) {
                        let widget_id = entry.terminal.widget_id().clone();
                        self.update_title_from_terminal(Some(next_id));
                        return TabAction::with_task(TerminalView::focus(
                            widget_id,
                        ));
                    }
                }
            }

            return TabAction::none();
        }

        TabAction::none()
    }

    pub fn focus_pane(&mut self, pane: pane_grid::Pane) -> TabAction {
        self.set_focus_on_pane(pane, true, true)
    }

    pub fn resize(&mut self, event: pane_grid::ResizeEvent) {
        self.panes.resize(event.split, event.ratio);
    }

    pub fn open_context_menu(
        &mut self,
        pane: pane_grid::Pane,
        terminal_id: u64,
    ) -> TabAction {
        let Some((widget_id, snapshot)) =
            self.terminals.get(&terminal_id).map(|entry| {
                (
                    entry.terminal.widget_id().clone(),
                    entry.terminal.snapshot_arc(),
                )
            })
        else {
            return TabAction::none();
        };
        if snapshot.view().mode.contains(SurfaceMode::ALT_SCREEN) {
            return TabAction::none();
        }

        let mut action = self.set_focus_on_pane(pane, false, false);

        let cursor = self.grid_cursor.unwrap_or_else(|| {
            Point::new(self.grid_size.width / 2.0, self.grid_size.height / 2.0)
        });

        let select_task = TerminalView::command(
            widget_id.clone(),
            BlockCommand::SelectHovered,
        );
        action.merge_task(select_task);

        let menu_state = PaneContextMenu::new(
            pane,
            cursor,
            self.grid_size,
            &self.theme.clone(),
            terminal_id,
        );
        action.merge_task(menu_state.focus_task());
        self.context_menu = Some(menu_state);

        action
    }

    pub fn close_context_menu(&mut self) -> TabAction {
        if self.context_menu.take().is_some() {
            if let Some(pane) = self.focus {
                return self.set_focus_on_pane(pane, false, true);
            }
        }

        TabAction::none()
    }

    pub fn handle_terminal_event(
        &mut self,
        event: otty_ui_term::Event,
        shell_name: &str,
    ) -> TabAction {
        use otty_ui_term::Event::*;

        let terminal_id = *event.terminal_id();

        match event {
            Shutdown { .. } => {
                if let Some(entry) = self.terminals.get(&terminal_id) {
                    let pane = entry.pane;
                    return self.close_pane(pane);
                }
            },
            TitleChanged { title, .. } => {
                if let Some(entry) = self.terminal_entry_mut(terminal_id) {
                    entry.title = title.clone();
                }
                if self.focused_terminal_id() == Some(terminal_id) {
                    self.title = title;
                }
            },
            ResetTitle { .. } => {
                let reset_title = shell_name.to_string();
                if let Some(entry) = self.terminal_entry_mut(terminal_id) {
                    entry.title = reset_title.clone();
                }
                if self.focused_terminal_id() == Some(terminal_id) {
                    self.title = reset_title;
                }
            },
            other => {
                if let Some(entry) = self.terminal_entry_mut(terminal_id) {
                    entry.terminal.handle(other);
                }
            },
        }

        TabAction::none()
    }

    pub fn update_grid_cursor(&mut self, position: Point) -> TabAction {
        self.grid_cursor = Some(Self::clamp_point(position, self.grid_size));
        TabAction::none()
    }

    pub fn set_grid_size(&mut self, size: Size) {
        self.grid_size = size;
        if let Some(cursor) = self.grid_cursor {
            self.grid_cursor = Some(Self::clamp_point(cursor, size));
        }
    }

    #[cfg(test)]
    pub(crate) fn grid_size(&self) -> Size {
        self.grid_size
    }

    fn view_single_pane(
        &self,
        pane: pane_grid::Pane,
        pane_state: &TabPane,
        is_focused: bool,
    ) -> Element<'_, Event, Theme, iced::Renderer> {
        let terminal_entry = self
            .terminals
            .get(&pane_state.terminal_id)
            .expect("terminal missing for pane");

        let tab_id = self.id;
        let pane_id = pane;
        let terminal_id = pane_state.terminal_id;
        let focus_event = Event::PaneClicked {
            tab_id,
            pane: pane_id,
        };

        let terminal_view =
            TerminalView::show(&terminal_entry.terminal).map(Event::Terminal);
        let terminal_area = mouse_area(terminal_view)
            .on_press(focus_event.clone())
            .on_right_press(Event::OpenPaneContextMenu {
                tab_id,
                pane: pane_id,
                terminal_id,
            })
            .into();

        let mut stack_widget = Stack::with_children(vec![terminal_area]);
        stack_widget = stack_widget.width(Length::Fill).height(Length::Fill);

        container(stack_widget)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |theme: &Theme| {
                let palette = theme.extended_palette();
                let border_color = if is_focused {
                    palette.primary.strong.color
                } else {
                    palette.background.strong.color
                };

                iced::widget::container::Style {
                    border: iced::Border {
                        width: 1.0,
                        color: border_color,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn set_focus_on_pane(
        &mut self,
        pane: pane_grid::Pane,
        close_menu: bool,
        focus_terminal: bool,
    ) -> TabAction {
        let Some(terminal_id) = self.pane_terminal_id(pane) else {
            return TabAction::none();
        };

        self.focus = Some(pane);
        if close_menu {
            self.context_menu = None;
        }
        self.update_title_from_terminal(Some(terminal_id));

        if focus_terminal {
            if let Some(entry) = self.terminals.get(&terminal_id) {
                return TabAction::with_task(TerminalView::focus(
                    entry.terminal.widget_id().clone(),
                ));
            }
        }

        TabAction::none()
    }

    fn update_title_from_terminal(&mut self, terminal_id: Option<u64>) {
        if let Some(id) = terminal_id {
            if let Some(entry) = self.terminals.get(&id) {
                self.title = entry.title.clone();
                return;
            }
        }

        self.title = self.default_title.clone();
    }

    fn clamp_point(point: Point, bounds: Size) -> Point {
        Point::new(
            point.x.clamp(0.0, bounds.width.max(0.0)),
            point.y.clamp(0.0, bounds.height.max(0.0)),
        )
    }
}
