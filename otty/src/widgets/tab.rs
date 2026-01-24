use std::collections::HashMap;

use iced::widget::pane_grid::{self, Highlight, Line, PaneGrid};
use iced::widget::{Stack, container, mouse_area};
use iced::{Border, Element, Length, Point, Theme};
use otty_ui_term::TerminalView;

use crate::app::theme::ThemeProps;
use crate::widgets::pane_context_menu::{
    PaneContextMenu, PaneContextMenuEvent, PaneContextMenuProps,
    PaneContextMenuState,
};

/// UI events emitted by the terminal tab view.
#[derive(Debug, Clone)]
pub(crate) enum TabEvent {
    Terminal(otty_ui_term::Event),
    ActivateTab {
        tab_id: u64,
    },
    CloseTab {
        tab_id: u64,
    },
    PaneClicked {
        tab_id: u64,
        pane: pane_grid::Pane,
    },
    PaneResized {
        tab_id: u64,
        event: pane_grid::ResizeEvent,
    },
    PaneGridCursorMoved {
        tab_id: u64,
        position: Point,
    },
    OpenContextMenu {
        tab_id: u64,
        pane: pane_grid::Pane,
        terminal_id: u64,
    },
    CloseContextMenu {
        tab_id: u64,
    },
    ContextMenuInput,
    SplitPane {
        tab_id: u64,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
    },
    ClosePane {
        tab_id: u64,
        pane: pane_grid::Pane,
    },
    CopySelection {
        tab_id: u64,
        terminal_id: u64,
    },
    PasteIntoPrompt {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockContent {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockPrompt {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockCommand {
        tab_id: u64,
        terminal_id: u64,
    },
}

/// Per-pane data stored inside a tab's pane grid.
#[derive(Debug, Clone)]
pub(crate) struct TabPane {
    pub(crate) terminal_id: u64,
}

/// Terminal entry used by the tab view.
pub(crate) struct TerminalEntry {
    pub(crate) pane: pane_grid::Pane,
    pub(crate) terminal: otty_ui_term::Terminal,
    pub(crate) title: String,
}

/// Props for rendering a terminal tab.
#[derive(Clone, Copy)]
pub(crate) struct TabProps<'a> {
    pub(crate) tab_id: u64,
    pub(crate) panes: &'a pane_grid::State<TabPane>,
    pub(crate) terminals: &'a HashMap<u64, TerminalEntry>,
    pub(crate) focus: Option<pane_grid::Pane>,
    pub(crate) context_menu: Option<&'a PaneContextMenuState>,
    pub(crate) selected_block_terminal: Option<u64>,
    pub(crate) theme: ThemeProps<'a>,
}

/// The main terminal tab view with panes and overlays.
pub(crate) struct TabView<'a> {
    props: TabProps<'a>,
}

impl<'a> TabView<'a> {
    pub fn new(props: TabProps<'a>) -> Self {
        Self { props }
    }

    pub fn view(self) -> Element<'a, TabEvent> {
        let tab_id = self.props.tab_id;
        let focus = self.props.focus;
        let terminals = self.props.terminals;

        let pane_grid =
            PaneGrid::new(self.props.panes, move |pane, pane_state, _| {
                let is_focused = focus == Some(pane);
                let content = view_single_pane(
                    tab_id, pane, pane_state, terminals, is_focused,
                );

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
            .on_resize(12.0, move |event| TabEvent::PaneResized {
                tab_id,
                event,
            });

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

        if let Some(menu) = self.props.context_menu {
            let has_block_selection =
                self.props.selected_block_terminal == Some(menu.terminal_id);
            layers.push(
                PaneContextMenu::new(PaneContextMenuProps {
                    menu,
                    has_block_selection,
                    theme: self.props.theme,
                })
                .view()
                .map(move |event| map_context_event(tab_id, menu, event)),
            );
        }

        let stack_widget = Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill);

        mouse_area(stack_widget)
            .on_move(move |position| TabEvent::PaneGridCursorMoved {
                tab_id,
                position,
            })
            .into()
    }
}

fn view_single_pane<'a>(
    tab_id: u64,
    pane: pane_grid::Pane,
    pane_state: &'a TabPane,
    terminals: &'a HashMap<u64, TerminalEntry>,
    is_focused: bool,
) -> Element<'a, TabEvent> {
    let terminal_entry = terminals
        .get(&pane_state.terminal_id)
        .expect("terminal missing for pane");

    let terminal_id = pane_state.terminal_id;
    let focus_event = TabEvent::PaneClicked { tab_id, pane };

    let terminal_view =
        TerminalView::show(&terminal_entry.terminal).map(TabEvent::Terminal);
    let terminal_area = mouse_area(terminal_view)
        .on_press(focus_event)
        .on_right_press(TabEvent::OpenContextMenu {
            tab_id,
            pane,
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

fn map_context_event(
    tab_id: u64,
    menu: &PaneContextMenuState,
    event: PaneContextMenuEvent,
) -> TabEvent {
    match event {
        PaneContextMenuEvent::SplitPane(axis) => TabEvent::SplitPane {
            tab_id,
            pane: menu.pane,
            axis,
        },
        PaneContextMenuEvent::ClosePane => TabEvent::ClosePane {
            tab_id,
            pane: menu.pane,
        },
        PaneContextMenuEvent::CopySelection => TabEvent::CopySelection {
            tab_id,
            terminal_id: menu.terminal_id,
        },
        PaneContextMenuEvent::PasteIntoPrompt => TabEvent::PasteIntoPrompt {
            tab_id,
            terminal_id: menu.terminal_id,
        },
        PaneContextMenuEvent::CopyBlockContent => {
            TabEvent::CopySelectedBlockContent {
                tab_id,
                terminal_id: menu.terminal_id,
            }
        },
        PaneContextMenuEvent::CopyBlockPrompt => {
            TabEvent::CopySelectedBlockPrompt {
                tab_id,
                terminal_id: menu.terminal_id,
            }
        },
        PaneContextMenuEvent::CopyBlockCommand => {
            TabEvent::CopySelectedBlockCommand {
                tab_id,
                terminal_id: menu.terminal_id,
            }
        },
        PaneContextMenuEvent::Dismiss => TabEvent::CloseContextMenu { tab_id },
        PaneContextMenuEvent::FocusInput => TabEvent::ContextMenuInput,
    }
}
