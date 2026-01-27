use std::collections::BTreeMap;

use iced::Size;

use crate::features::tab::{TabContent, TabItem};
use crate::ui::widgets::tab_bar;

#[derive(Default)]
pub(crate) struct State {
    pub(crate) active_tab_id: Option<u64>,
    pub(crate) tab_items: BTreeMap<u64, TabItem>,
    pub(crate) next_tab_id: u64,
    pub(crate) next_terminal_id: u64,
    pub(crate) window_size: Size,
    pub(crate) screen_size: Size,
}

impl State {
    pub(crate) fn new(window_size: Size, screen_size: Size) -> Self {
        Self {
            window_size,
            screen_size,
            ..Default::default()
        }
    }

    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.active_tab_id
            .and_then(|id| self.tab_items.get(&id))
            .map(|tab| tab.title.as_str())
    }

    pub(crate) fn tab_summaries(&self) -> Vec<(u64, String)> {
        self.tab_items
            .iter()
            .map(|(id, item)| (*id, item.title.clone()))
            .collect()
    }

    pub(crate) fn active_tab(&self) -> Option<&TabItem> {
        let id = self.active_tab_id?;
        self.tab_items.get(&id)
    }

    pub(crate) fn set_screen_size(&mut self, size: Size) {
        self.screen_size = size;
        self.sync_tab_grid_sizes();
    }

    pub(crate) fn sync_tab_grid_sizes(&mut self) {
        let size = self.pane_grid_size();
        for tab in self.tab_items.values_mut() {
            let TabContent::Terminal(terminal) = &mut tab.content;
            terminal.set_grid_size(size);
        }
    }

    pub(crate) fn pane_grid_size(&self) -> Size {
        let tab_bar_height = tab_bar::TAB_BAR_HEIGHT;
        let height = (self.screen_size.height - tab_bar_height).max(0.0);
        Size::new(self.screen_size.width, height)
    }
}
