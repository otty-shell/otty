use iced::widget::text::Wrapping;
use iced::widget::{column, container, scrollable, svg, text};
use iced::{Element, Length, Padding, alignment};
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};
use otty_ui_tree::{TreeRowContext, TreeView};

use super::super::event::ExplorerIntent;
use super::super::model::ExplorerTreeViewModel;
use super::super::types::FileNode;
use crate::i18n::{self, Key};
use crate::icons::FILE;
use crate::style::thin_scroll_style;
use crate::theme::{IcedColorPalette, ThemeProps};

const TREE_PADDING_BOTTOM: f32 = 20.0;

/// Props for the explorer sidebar tree view.
#[derive(Debug, Clone)]
pub(crate) struct SidebarTreeProps<'a> {
    pub(crate) vm: ExplorerTreeViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the full explorer sidebar panel (header + directory bar + tree).
pub(crate) fn view(
    props: SidebarTreeProps<'_>,
) -> Element<'_, ExplorerIntent, iced::Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette();

    let ui = props.theme.theme.ui();
    let current_dir = current_directory_bar(props.vm.root_label, ui);
    let tree_list = explorer_tree(&props.vm, palette, ui);

    let content = column![current_dir, tree_list]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render the current directory label bar.
fn current_directory_bar<'a>(
    root_label: Option<&'a str>,
    palette: &UiPalette,
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
    let label = root_label.unwrap_or(i18n::t(Key::NoActiveFolder));
    let background = palette.surface_panel();
    let style = typography::HEADING;

    let label_text = text(label)
        .size(style.size())
        .font(fonts::UI_SEMIBOLD)
        .color(palette.text_strong())
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    // As tall as the tab strip so both bottom edges line up.
    container(label_text)
        .width(Length::Fill)
        .height(Length::Fixed(size::TAB))
        .padding(Padding::from([0.0, spacing::SPACE_4]))
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center)
        .style(move |_| iced::widget::container::Style {
            background: Some(background.into()),
            ..Default::default()
        })
        .into()
}

/// Render the scrollable explorer tree.
fn explorer_tree<'a>(
    vm: &ExplorerTreeViewModel<'a>,
    palette: &'a IcedColorPalette,
    ui: &UiPalette,
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
    let row_palette = ui.clone();
    let tree_view = TreeView::new(vm.tree, move |context| {
        render_entry(context, &row_palette)
    })
    .selected_row(vm.selected_path)
    .hovered_row(vm.hovered_path)
    .on_press(|path| ExplorerIntent::NodePressed { path })
    .on_hover(|path| ExplorerIntent::NodeHovered { path })
    .spacing(0.0);

    let scrollable = scrollable::Scrollable::new(tree_view.view())
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .style(thin_scroll_style(palette.clone()));

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::default().bottom(TREE_PADDING_BOTTOM))
        .into()
}

fn render_entry<'a>(
    context: &TreeRowContext<'a, FileNode>,
    palette: &UiPalette,
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
    let node = context.entry.node;

    let row = TreeRow::new(node.name(), palette)
        .depth(context.entry.depth as u16)
        .selected(context.is_selected)
        .hovered(context.is_hovered);
    let row = if node.is_folder() {
        row.caret(node.is_expanded())
    } else {
        row.icon(svg::Handle::from_memory(FILE), None)
    };

    row.into()
}
