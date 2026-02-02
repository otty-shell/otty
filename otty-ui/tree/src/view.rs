use iced::alignment;
use iced::widget::{Column, Row, Space, container, mouse_area};
use iced::{Element, Length, mouse};

use crate::model::{FlattenedNode, TreeNode, TreePath, flatten_tree};

/// Flattened tree row used by [`TreeView`] render callbacks.
pub type TreeRow<'a, T> = FlattenedNode<'a, T>;

/// Rendering context passed to row callbacks.
pub struct TreeRowContext<'a, T: TreeNode> {
    pub entry: TreeRow<'a, T>,
    pub is_selected: bool,
    pub is_hovered: bool,
}

type RowRenderer<'a, T, Message> =
    dyn Fn(&TreeRowContext<'a, T>) -> Element<'a, Message> + 'a;
type RowStyle<'a, T> = dyn Fn(&TreeRowContext<'a, T>) -> container::Style + 'a;
type ToggleContent<'a, T, Message> =
    dyn Fn(&TreeRowContext<'a, T>) -> Element<'a, Message> + 'a;
type RowAction<'a, Message> = dyn Fn(TreePath) -> Message + 'a;
type HoverAction<'a, Message> = dyn Fn(Option<TreePath>) -> Message + 'a;
type RowPredicate<'a, T> = dyn Fn(&TreeRowContext<'a, T>) -> bool + 'a;
type RowExtra<'a, T, Message> =
    dyn Fn(&TreeRowContext<'a, T>) -> Option<Element<'a, Message>> + 'a;
type RowsExtra<'a, Message> = dyn Fn() -> Option<Element<'a, Message>> + 'a;

/// Lightweight tree view helper that wires selection to row rendering.
pub struct TreeView<'a, T: TreeNode, Message: Clone + 'a> {
    nodes: &'a [T],
    selected: Option<&'a TreePath>,
    hovered: Option<&'a TreePath>,
    on_press: Option<Box<RowAction<'a, Message>>>,
    on_release: Option<Box<RowAction<'a, Message>>>,
    on_right_press: Option<Box<RowAction<'a, Message>>>,
    on_enter: Option<Box<RowAction<'a, Message>>>,
    on_exit: Option<Box<RowAction<'a, Message>>>,
    on_hover: Option<Box<HoverAction<'a, Message>>>,
    on_toggle_folder: Option<Box<RowAction<'a, Message>>>,
    render_row: Box<RowRenderer<'a, T, Message>>,
    row_style: Option<Box<RowStyle<'a, T>>>,
    toggle_content: Option<Box<ToggleContent<'a, T, Message>>>,
    row_visible: Option<Box<RowPredicate<'a, T>>>,
    row_interactive: Option<Box<RowPredicate<'a, T>>>,
    before_rows: Option<Box<RowsExtra<'a, Message>>>,
    after_rows: Option<Box<RowsExtra<'a, Message>>>,
    before_row: Option<Box<RowExtra<'a, T, Message>>>,
    after_row: Option<Box<RowExtra<'a, T, Message>>>,
    spacing: f32,
    indent_width: f32,
    toggle_width: f32,
}

impl<'a, T, Message> TreeView<'a, T, Message>
where
    T: TreeNode + 'a,
    Message: Clone + 'a,
{
    /// Create a tree view that renders each row using `render_row`.
    pub fn new(
        nodes: &'a [T],
        render_row: impl Fn(&TreeRowContext<'a, T>) -> Element<'a, Message> + 'a,
    ) -> Self {
        Self {
            nodes,
            selected: None,
            hovered: None,
            on_press: None,
            on_release: None,
            on_right_press: None,
            on_enter: None,
            on_exit: None,
            on_hover: None,
            on_toggle_folder: None,
            render_row: Box::new(render_row),
            row_style: None,
            toggle_content: None,
            row_visible: None,
            row_interactive: None,
            before_rows: None,
            after_rows: None,
            before_row: None,
            after_row: None,
            spacing: 0.0,
            indent_width: 0.0,
            toggle_width: 0.0,
        }
    }

    /// Provide the currently selected path to inform row rendering.
    pub fn selected(mut self, path: Option<&'a TreePath>) -> Self {
        self.selected = path;
        self
    }

    /// Provide the currently hovered path to inform row rendering.
    pub fn hovered(mut self, path: Option<&'a TreePath>) -> Self {
        self.hovered = path;
        self
    }

    /// Emit a message when a row is clicked.
    pub fn on_select(
        mut self,
        on_select: impl Fn(TreePath) -> Message + 'a,
    ) -> Self {
        self.on_press = Some(Box::new(on_select));
        self
    }

    /// Emit a message when a row receives a left press.
    pub fn on_press(
        mut self,
        on_press: impl Fn(TreePath) -> Message + 'a,
    ) -> Self {
        self.on_press = Some(Box::new(on_press));
        self
    }

    /// Emit a message when a row receives a left release.
    pub fn on_release(
        mut self,
        on_release: impl Fn(TreePath) -> Message + 'a,
    ) -> Self {
        self.on_release = Some(Box::new(on_release));
        self
    }

    /// Emit a message when a row receives a right press.
    pub fn on_right_press(
        mut self,
        on_right_press: impl Fn(TreePath) -> Message + 'a,
    ) -> Self {
        self.on_right_press = Some(Box::new(on_right_press));
        self
    }

    /// Emit a message when the pointer enters a row.
    pub fn on_enter(
        mut self,
        on_enter: impl Fn(TreePath) -> Message + 'a,
    ) -> Self {
        self.on_enter = Some(Box::new(on_enter));
        self
    }

    /// Emit a message when the pointer leaves a row.
    pub fn on_exit(
        mut self,
        on_exit: impl Fn(TreePath) -> Message + 'a,
    ) -> Self {
        self.on_exit = Some(Box::new(on_exit));
        self
    }

    /// Emit a message when the pointer enters or leaves a row.
    pub fn on_hover(
        mut self,
        on_hover: impl Fn(Option<TreePath>) -> Message + 'a,
    ) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }

    /// Emit a message when a folder toggle is clicked.
    pub fn on_toggle_folder(
        mut self,
        on_toggle: impl Fn(TreePath) -> Message + 'a,
    ) -> Self {
        self.on_toggle_folder = Some(Box::new(on_toggle));
        self
    }

    /// Provide a row style callback for background/text styling.
    pub fn row_style(
        mut self,
        row_style: impl Fn(&TreeRowContext<'a, T>) -> container::Style + 'a,
    ) -> Self {
        self.row_style = Some(Box::new(row_style));
        self
    }

    /// Provide content to render inside the toggle area.
    pub fn toggle_content(
        mut self,
        toggle_content: impl Fn(&TreeRowContext<'a, T>) -> Element<'a, Message> + 'a,
    ) -> Self {
        self.toggle_content = Some(Box::new(toggle_content));
        self
    }

    /// Control whether a row is rendered.
    pub fn row_visible(
        mut self,
        row_visible: impl Fn(&TreeRowContext<'a, T>) -> bool + 'a,
    ) -> Self {
        self.row_visible = Some(Box::new(row_visible));
        self
    }

    /// Control whether a row receives mouse interaction handlers.
    pub fn row_interactive(
        mut self,
        row_interactive: impl Fn(&TreeRowContext<'a, T>) -> bool + 'a,
    ) -> Self {
        self.row_interactive = Some(Box::new(row_interactive));
        self
    }

    /// Insert content before all rows.
    pub fn before_rows(
        mut self,
        before_rows: impl Fn() -> Option<Element<'a, Message>> + 'a,
    ) -> Self {
        self.before_rows = Some(Box::new(before_rows));
        self
    }

    /// Insert content after all rows.
    pub fn after_rows(
        mut self,
        after_rows: impl Fn() -> Option<Element<'a, Message>> + 'a,
    ) -> Self {
        self.after_rows = Some(Box::new(after_rows));
        self
    }

    /// Insert content before a given row.
    pub fn before_row(
        mut self,
        before_row: impl Fn(&TreeRowContext<'a, T>) -> Option<Element<'a, Message>>
        + 'a,
    ) -> Self {
        self.before_row = Some(Box::new(before_row));
        self
    }

    /// Insert content after a given row.
    pub fn after_row(
        mut self,
        after_row: impl Fn(&TreeRowContext<'a, T>) -> Option<Element<'a, Message>>
        + 'a,
    ) -> Self {
        self.after_row = Some(Box::new(after_row));
        self
    }

    /// Set indentation width per tree depth level.
    pub fn indent_width(mut self, width: f32) -> Self {
        self.indent_width = width.max(0.0);
        self
    }

    /// Set the width reserved for the toggle area.
    pub fn toggle_width(mut self, width: f32) -> Self {
        self.toggle_width = width.max(0.0);
        self
    }

    /// Vertical spacing between rows.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Build the `Element` for the tree view.
    pub fn view(self) -> Element<'a, Message> {
        let mut column = Column::new().spacing(self.spacing);

        if let Some(ref before_rows) = self.before_rows {
            if let Some(extra) = before_rows() {
                column = column.push(extra);
            }
        }

        for entry in flatten_tree(self.nodes) {
            let is_selected = self
                .selected
                .map(|path| path == &entry.path)
                .unwrap_or(false);
            let is_hovered = self
                .hovered
                .map(|path| path == &entry.path)
                .unwrap_or(false);
            let path = entry.path.clone();
            let context = TreeRowContext {
                entry,
                is_selected,
                is_hovered,
            };

            if let Some(ref before_row) = self.before_row {
                if let Some(extra) = before_row(&context) {
                    column = column.push(extra);
                }
            }

            let is_visible = self
                .row_visible
                .as_ref()
                .map(|predicate| predicate(&context))
                .unwrap_or(true);

            if is_visible {
                let content = (self.render_row)(&context);
                let is_interactive = self
                    .row_interactive
                    .as_ref()
                    .map(|predicate| predicate(&context))
                    .unwrap_or(true);
                let content = if is_interactive {
                    wrap_mouse_area(
                        content,
                        self.on_press.as_deref(),
                        self.on_release.as_deref(),
                        self.on_right_press.as_deref(),
                        self.on_enter.as_deref(),
                        self.on_exit.as_deref(),
                        self.on_hover.as_deref(),
                        &path,
                    )
                } else {
                    content
                };

                let mut row = Row::new().spacing(0.0);

                if self.indent_width > 0.0 {
                    let indent =
                        context.entry.depth as f32 * self.indent_width.max(0.0);
                    if indent > 0.0 {
                        row =
                            row.push(Space::new().width(Length::Fixed(indent)));
                    }
                }

                if self.toggle_width > 0.0 || self.toggle_content.is_some() {
                    let toggle_slot = build_toggle_slot(
                        &context,
                        &self,
                        &path,
                        self.on_enter.as_deref(),
                        self.on_exit.as_deref(),
                        self.on_hover.as_deref(),
                    );
                    row = row.push(toggle_slot);
                }

                row = row.push(content);

                let mut row_element: Element<'a, Message> = row.into();

                if let Some(ref row_style) = self.row_style {
                    let style = row_style(&context);
                    row_element =
                        container(row_element).style(move |_| style).into();
                }

                column = column.push(row_element);
            }

            if let Some(ref after_row) = self.after_row {
                if let Some(extra) = after_row(&context) {
                    column = column.push(extra);
                }
            }
        }

        if let Some(ref after_rows) = self.after_rows {
            if let Some(extra) = after_rows() {
                column = column.push(extra);
            }
        }

        column.into()
    }
}

fn wrap_mouse_area<'a, Message: Clone + 'a>(
    element: Element<'a, Message>,
    on_press: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_release: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_right_press: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_enter: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_exit: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_hover: Option<&(dyn Fn(Option<TreePath>) -> Message + 'a)>,
    path: &TreePath,
) -> Element<'a, Message> {
    if on_press.is_none()
        && on_release.is_none()
        && on_right_press.is_none()
        && on_enter.is_none()
        && on_exit.is_none()
        && on_hover.is_none()
    {
        return element;
    }

    let mut area = mouse_area(element);

    if let Some(on_press) = on_press {
        area = area.on_press(on_press(path.clone()));
    }

    if let Some(on_release) = on_release {
        area = area.on_release(on_release(path.clone()));
    }

    if let Some(on_right_press) = on_right_press {
        area = area.on_right_press(on_right_press(path.clone()));
    }

    if let Some(on_enter) = on_enter {
        area = area.on_enter(on_enter(path.clone()));
    }

    if let Some(on_exit) = on_exit {
        area = area.on_exit(on_exit(path.clone()));
    }

    if on_enter.is_none() && on_exit.is_none() {
        if let Some(on_hover) = on_hover {
            area = area
                .on_enter(on_hover(Some(path.clone())))
                .on_exit(on_hover(None));
        }
    }

    area.interaction(mouse::Interaction::Pointer).into()
}

fn build_toggle_slot<'a, T, Message>(
    context: &TreeRowContext<'a, T>,
    view: &TreeView<'a, T, Message>,
    path: &TreePath,
    on_enter: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_exit: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_hover: Option<&(dyn Fn(Option<TreePath>) -> Message + 'a)>,
) -> Element<'a, Message>
where
    T: TreeNode + 'a,
    Message: Clone + 'a,
{
    let width = view.toggle_width.max(0.0);
    let content = view
        .toggle_content
        .as_ref()
        .map(|toggle| toggle(context))
        .unwrap_or_else(|| Space::new().into());

    let content = container(content)
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into();

    if context.entry.node.is_folder() {
        if let Some(on_toggle) = view.on_toggle_folder.as_ref() {
            return wrap_mouse_area(
                content,
                Some(on_toggle),
                None,
                None,
                on_enter,
                on_exit,
                on_hover,
                path,
            );
        }
    }

    if on_hover.is_some() || on_enter.is_some() || on_exit.is_some() {
        wrap_mouse_area(
            content, None, None, None, on_enter, on_exit, on_hover, path,
        )
    } else {
        content
    }
}
