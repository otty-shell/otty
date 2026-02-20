use iced::alignment;
use iced::widget::{Column, Row, Space, container, mouse_area};
use iced::{Element, Length, mouse};

use crate::model::{FlattenedNode, TreeNode, TreePath, flatten_tree};

/// Flattened tree row used by [`TreeView`] render callbacks.
pub type TreeRow<'a, T> = FlattenedNode<'a, T>;

/// Per-row context passed to all render/style/filter callbacks.
pub struct TreeRowContext<'a, T: TreeNode> {
    /// Flattened node info for the current row.
    pub entry: TreeRow<'a, T>,
    /// Whether this row matches [`TreeView::selected_row`].
    pub is_selected: bool,
    /// Whether this row matches [`TreeView::hovered_row`].
    pub is_hovered: bool,
}

type RowRenderer<'a, T, Message> =
    dyn Fn(&TreeRowContext<'a, T>) -> Element<'a, Message> + 'a;
type RowStyle<'a, T> = dyn Fn(&TreeRowContext<'a, T>) -> container::Style + 'a;
type RowLeadingContent<'a, T, Message> =
    dyn Fn(&TreeRowContext<'a, T>) -> Element<'a, Message> + 'a;
type RowAction<'a, Message> = dyn Fn(TreePath) -> Message + 'a;
type HoverAction<'a, Message> = dyn Fn(Option<TreePath>) -> Message + 'a;
type RowPredicate<'a, T> = dyn Fn(&TreeRowContext<'a, T>) -> bool + 'a;
type RowExtra<'a, T, Message> =
    dyn Fn(&TreeRowContext<'a, T>) -> Option<Element<'a, Message>> + 'a;

/// Lightweight tree widget for `iced`.
///
/// `TreeView` is intentionally stateless: selection/hover are provided by the
/// caller on each render via [`TreeView::selected_row`] and
/// [`TreeView::hovered_row`].
pub struct TreeView<'a, T: TreeNode, Message: Clone + 'a> {
    nodes: &'a [T],
    selected: Option<&'a TreePath>,
    hovered: Option<&'a TreePath>,
    on_press: Option<Box<RowAction<'a, Message>>>,
    on_release: Option<Box<RowAction<'a, Message>>>,
    on_right_press: Option<Box<RowAction<'a, Message>>>,
    on_hover: Option<Box<HoverAction<'a, Message>>>,
    render_row: Box<RowRenderer<'a, T, Message>>,
    row_style: Option<Box<RowStyle<'a, T>>>,
    row_leading_content: Option<Box<RowLeadingContent<'a, T, Message>>>,
    row_visible_filter: Option<Box<RowPredicate<'a, T>>>,
    row_interactive_filter: Option<Box<RowPredicate<'a, T>>>,
    before_row: Option<Box<RowExtra<'a, T, Message>>>,
    after_row: Option<Box<RowExtra<'a, T, Message>>>,
    spacing: f32,
    indent_size: f32,
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
            on_hover: None,
            render_row: Box::new(render_row),
            row_style: None,
            row_leading_content: None,
            row_visible_filter: None,
            row_interactive_filter: None,
            before_row: None,
            after_row: None,
            spacing: 0.0,
            indent_size: 0.0,
        }
    }

    /// Sets the selected row used for row styling and context flags.
    ///
    /// This does not emit messages by itself. Use [`TreeView::on_press`] (or
    /// other callbacks) to update selection in your application state.
    pub fn selected_row(mut self, path: Option<&'a TreePath>) -> Self {
        self.selected = path;
        self
    }

    /// Sets the hovered row used for row styling and context flags.
    ///
    /// This is render input state. Hover events are emitted separately via
    /// [`TreeView::on_hover`].
    pub fn hovered_row(mut self, path: Option<&'a TreePath>) -> Self {
        self.hovered = path;
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

    /// Emit a message when the pointer enters or leaves a row.
    ///
    /// The callback receives:
    /// - `Some(path)` when entering a row;
    /// - `None` when leaving the tree area.
    pub fn on_hover(
        mut self,
        on_hover: impl Fn(Option<TreePath>) -> Message + 'a,
    ) -> Self {
        self.on_hover = Some(Box::new(on_hover));
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

    /// Provide content rendered before each row body.
    ///
    /// Typical use is folder/file indicators, chevrons, badges, or fixed
    /// placeholders to keep labels aligned.
    pub fn row_leading_content(
        mut self,
        row_leading_content: impl Fn(&TreeRowContext<'a, T>) -> Element<'a, Message>
        + 'a,
    ) -> Self {
        self.row_leading_content = Some(Box::new(row_leading_content));
        self
    }

    /// Predicate that controls whether the main row is rendered.
    ///
    /// `false` hides the row body and row mouse handlers, but
    /// [`TreeView::before_row`] and [`TreeView::after_row`] hooks are still
    /// evaluated for this entry.
    pub fn row_visible_filter(
        mut self,
        row_visible_filter: impl Fn(&TreeRowContext<'a, T>) -> bool + 'a,
    ) -> Self {
        self.row_visible_filter = Some(Box::new(row_visible_filter));
        self
    }

    /// Predicate that controls whether row-level mouse handlers are attached.
    ///
    /// When `false`, the row still renders but row callbacks (`on_press`,
    /// `on_release`, `on_right_press`, `on_hover`) are not wired for that row.
    pub fn row_interactive_filter(
        mut self,
        row_interactive_filter: impl Fn(&TreeRowContext<'a, T>) -> bool + 'a,
    ) -> Self {
        self.row_interactive_filter = Some(Box::new(row_interactive_filter));
        self
    }

    /// Insert an extra element before each entry for which callback returns
    /// `Some`.
    ///
    /// This hook runs for every entry, even when the entry is hidden by
    /// [`TreeView::row_visible_filter`].
    pub fn before_row(
        mut self,
        before_row: impl Fn(&TreeRowContext<'a, T>) -> Option<Element<'a, Message>>
        + 'a,
    ) -> Self {
        self.before_row = Some(Box::new(before_row));
        self
    }

    /// Insert an extra element after each entry for which callback returns
    /// `Some`.
    ///
    /// This hook runs for every entry, even when the entry is hidden by
    /// [`TreeView::row_visible_filter`].
    pub fn after_row(
        mut self,
        after_row: impl Fn(&TreeRowContext<'a, T>) -> Option<Element<'a, Message>>
        + 'a,
    ) -> Self {
        self.after_row = Some(Box::new(after_row));
        self
    }

    /// Set indentation size per tree depth level.
    pub fn indent_size(mut self, size: f32) -> Self {
        self.indent_size = size.max(0.0);
        self
    }

    /// Vertical spacing between rows.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Build the tree widget as an `iced::Element`.
    ///
    /// If [`TreeView::on_hover`] is configured, the resulting tree also emits
    /// a final `on_hover(None)` when the pointer leaves the whole tree area.
    pub fn view(self) -> Element<'a, Message> {
        let mut column =
            Column::new().spacing(self.spacing).width(Length::Fill);

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
                .row_visible_filter
                .as_ref()
                .map(|predicate| predicate(&context))
                .unwrap_or(true);

            if is_visible {
                let content: Element<'a, Message> =
                    container((self.render_row)(&context))
                        .width(Length::Fill)
                        .into();
                let is_interactive = self
                    .row_interactive_filter
                    .as_ref()
                    .map(|predicate| predicate(&context))
                    .unwrap_or(true);

                let mut row = Row::new().spacing(0.0).width(Length::Fill);

                if self.indent_size > 0.0 {
                    let indent =
                        context.entry.depth as f32 * self.indent_size.max(0.0);
                    if indent > 0.0 {
                        row =
                            row.push(Space::new().width(Length::Fixed(indent)));
                    }
                }

                if self.row_leading_content.is_some() {
                    let leading_hover = if is_interactive {
                        None
                    } else {
                        self.on_hover.as_deref()
                    };
                    let leading_slot = build_row_leading_slot(
                        &context,
                        &self,
                        &path,
                        leading_hover,
                    );
                    row = row.push(leading_slot);
                }

                row = row.push(content);

                let mut row_element: Element<'a, Message> = row.into();
                if is_interactive {
                    row_element = wrap_mouse_area(
                        row_element,
                        self.on_press.as_deref(),
                        self.on_release.as_deref(),
                        self.on_right_press.as_deref(),
                        self.on_hover.as_deref(),
                        false,
                        &path,
                    );
                }

                if let Some(ref row_style) = self.row_style {
                    let style = row_style(&context);
                    row_element = container(row_element)
                        .width(Length::Fill)
                        .style(move |_| style)
                        .into();
                }

                column = column.push(row_element);
            }

            if let Some(ref after_row) = self.after_row {
                if let Some(extra) = after_row(&context) {
                    column = column.push(extra);
                }
            }
        }

        let tree: Element<'a, Message> = column.into();
        if let Some(on_hover) = self.on_hover.as_deref() {
            mouse_area(tree).on_exit(on_hover(None)).into()
        } else {
            tree
        }
    }
}

fn wrap_mouse_area<'a, Message: Clone + 'a>(
    element: Element<'a, Message>,
    on_press: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_release: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_right_press: Option<&(dyn Fn(TreePath) -> Message + 'a)>,
    on_hover: Option<&(dyn Fn(Option<TreePath>) -> Message + 'a)>,
    emit_hover_exit: bool,
    path: &TreePath,
) -> Element<'a, Message> {
    if on_press.is_none()
        && on_release.is_none()
        && on_right_press.is_none()
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

    if let Some(on_hover) = on_hover {
        area = area.on_enter(on_hover(Some(path.clone())));
        if emit_hover_exit {
            area = area.on_exit(on_hover(None));
        }
    }

    area.interaction(mouse::Interaction::Pointer).into()
}

fn build_row_leading_slot<'a, T, Message>(
    context: &TreeRowContext<'a, T>,
    view: &TreeView<'a, T, Message>,
    path: &TreePath,
    on_hover: Option<&(dyn Fn(Option<TreePath>) -> Message + 'a)>,
) -> Element<'a, Message>
where
    T: TreeNode + 'a,
    Message: Clone + 'a,
{
    let content = view
        .row_leading_content
        .as_ref()
        .map(|toggle| toggle(context))
        .unwrap_or_else(|| Space::new().into());

    let content = container(content)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into();

    if on_hover.is_some() {
        wrap_mouse_area(content, None, None, None, on_hover, false, path)
    } else {
        content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Clone, Debug)]
    enum TestNode {
        Folder {
            title: String,
            expanded: bool,
            children: Vec<TestNode>,
        },
        File {
            title: String,
        },
    }

    impl TestNode {
        fn folder(title: &str, expanded: bool, children: Vec<Self>) -> Self {
            Self::Folder {
                title: title.to_owned(),
                expanded,
                children,
            }
        }

        fn file(title: &str) -> Self {
            Self::File {
                title: title.to_owned(),
            }
        }
    }

    impl TreeNode for TestNode {
        fn title(&self) -> &str {
            match self {
                TestNode::Folder { title, .. } => title,
                TestNode::File { title } => title,
            }
        }

        fn children(&self) -> Option<&[Self]> {
            match self {
                TestNode::Folder { children, .. } => Some(children),
                TestNode::File { .. } => None,
            }
        }

        fn expanded(&self) -> bool {
            match self {
                TestNode::Folder { expanded, .. } => *expanded,
                TestNode::File { .. } => false,
            }
        }

        fn is_folder(&self) -> bool {
            matches!(self, TestNode::Folder { .. })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestMessage {
        Press(TreePath),
        Release(TreePath),
        RightPress(TreePath),
        Hover(Option<TreePath>),
    }

    fn path(parts: &[&str]) -> TreePath {
        parts.iter().map(|part| (*part).to_owned()).collect()
    }

    fn one_file_tree() -> Vec<TestNode> {
        vec![TestNode::file("leaf")]
    }

    fn one_folder_tree() -> Vec<TestNode> {
        vec![TestNode::folder("folder", false, vec![])]
    }

    fn mixed_tree() -> Vec<TestNode> {
        vec![
            TestNode::file("top"),
            TestNode::folder("root", true, vec![TestNode::file("child")]),
        ]
    }

    #[test]
    fn tree_view_handles_empty_tree() {
        let nodes: Vec<TestNode> = Vec::new();
        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .view();
    }

    #[test]
    fn selected_and_hovered_are_exposed_in_context() {
        let nodes = mixed_tree();
        let selected = path(&["root", "child"]);
        let hovered = path(&["top"]);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_for_row = Rc::clone(&seen);

        let _ =
            TreeView::<TestNode, TestMessage>::new(&nodes, move |context| {
                seen_for_row.borrow_mut().push((
                    context.entry.path.clone(),
                    context.is_selected,
                    context.is_hovered,
                ));
                Space::new().into()
            })
            .selected_row(Some(&selected))
            .hovered_row(Some(&hovered))
            .indent_size(10.0)
            .view();

        let seen = seen.borrow();
        assert_eq!(seen.len(), 3);
        assert!(seen.iter().any(|entry| entry.0 == path(&["root", "child"])
            && entry.1
            && !entry.2));
        assert!(
            seen.iter()
                .any(|entry| entry.0 == path(&["top"]) && !entry.1 && entry.2)
        );
        assert!(
            seen.iter().any(|entry| entry.0 == path(&["root"])
                && !entry.1
                && !entry.2)
        );
    }

    #[test]
    fn builder_clamps_negative_indent_and_keeps_spacing() {
        let nodes = one_file_tree();
        let selected = path(&["leaf"]);
        let hovered = path(&["leaf"]);
        let view = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .selected_row(Some(&selected))
        .hovered_row(Some(&hovered))
        .indent_size(-3.0)
        .spacing(1.5);

        assert_eq!(view.indent_size, 0.0);
        assert_eq!(view.spacing, 1.5);
    }

    #[test]
    fn on_press_dispatches_press_message() {
        let nodes = one_file_tree();
        let pressed = Rc::new(RefCell::new(Vec::new()));
        let pressed_for_cb = Rc::clone(&pressed);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .on_press(move |tree_path| {
            pressed_for_cb.borrow_mut().push(tree_path.clone());
            TestMessage::Press(tree_path)
        })
        .view();

        assert_eq!(*pressed.borrow(), vec![path(&["leaf"])]);
    }

    #[test]
    fn row_visibility_and_decorators_are_applied_per_entry() {
        let nodes = mixed_tree();
        let rendered = Rc::new(Cell::new(0_usize));
        let styled = Rc::new(Cell::new(0_usize));
        let before_calls = Rc::new(Cell::new(0_usize));
        let after_calls = Rc::new(Cell::new(0_usize));
        let before_inserts = Rc::new(Cell::new(0_usize));
        let after_inserts = Rc::new(Cell::new(0_usize));

        let rendered_for_row = Rc::clone(&rendered);
        let styled_for_row = Rc::clone(&styled);
        let before_calls_for_row = Rc::clone(&before_calls);
        let after_calls_for_row = Rc::clone(&after_calls);
        let before_inserts_for_row = Rc::clone(&before_inserts);
        let after_inserts_for_row = Rc::clone(&after_inserts);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, move |_| {
            rendered_for_row.set(rendered_for_row.get() + 1);
            Space::new().into()
        })
        .row_style(move |_| {
            styled_for_row.set(styled_for_row.get() + 1);
            container::Style::default()
        })
        .row_visible_filter(|context| {
            context.entry.path != path(&["root", "child"])
        })
        .before_row(move |context| {
            before_calls_for_row.set(before_calls_for_row.get() + 1);
            if context.entry.path == path(&["root", "child"]) {
                before_inserts_for_row.set(before_inserts_for_row.get() + 1);
                Some(Space::new().into())
            } else {
                None
            }
        })
        .after_row(move |context| {
            after_calls_for_row.set(after_calls_for_row.get() + 1);
            if context.entry.path == path(&["top"]) {
                after_inserts_for_row.set(after_inserts_for_row.get() + 1);
                Some(Space::new().into())
            } else {
                None
            }
        })
        .view();

        assert_eq!(rendered.get(), 2);
        assert_eq!(styled.get(), 2);
        assert_eq!(before_calls.get(), 3);
        assert_eq!(after_calls.get(), 3);
        assert_eq!(before_inserts.get(), 1);
        assert_eq!(after_inserts.get(), 1);
    }

    #[test]
    fn row_interactive_filter_false_skips_mouse_handlers() {
        let nodes = one_file_tree();
        let presses = Rc::new(Cell::new(0_usize));
        let presses_for_cb = Rc::clone(&presses);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .on_press(move |tree_path| {
            presses_for_cb.set(presses_for_cb.get() + 1);
            TestMessage::Press(tree_path)
        })
        .row_interactive_filter(|_| false)
        .view();

        assert_eq!(presses.get(), 0);
    }

    #[test]
    fn tree_view_mouse_callbacks_are_wired() {
        let nodes = one_file_tree();
        let press_count = Rc::new(Cell::new(0_usize));
        let release_count = Rc::new(Cell::new(0_usize));
        let right_count = Rc::new(Cell::new(0_usize));
        let hover_count = Rc::new(Cell::new(0_usize));

        let press_cb = Rc::clone(&press_count);
        let release_cb = Rc::clone(&release_count);
        let right_cb = Rc::clone(&right_count);
        let hover_cb = Rc::clone(&hover_count);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .on_press(move |tree_path| {
            press_cb.set(press_cb.get() + 1);
            TestMessage::Press(tree_path)
        })
        .on_release(move |tree_path| {
            release_cb.set(release_cb.get() + 1);
            TestMessage::Release(tree_path)
        })
        .on_right_press(move |tree_path| {
            right_cb.set(right_cb.get() + 1);
            TestMessage::RightPress(tree_path)
        })
        .on_hover(move |tree_path| {
            hover_cb.set(hover_cb.get() + 1);
            TestMessage::Hover(tree_path)
        })
        .view();

        assert_eq!(press_count.get(), 1);
        assert_eq!(release_count.get(), 1);
        assert_eq!(right_count.get(), 1);
        assert_eq!(hover_count.get(), 2);
    }

    #[test]
    fn wrap_mouse_area_returns_when_no_handlers_are_set() {
        let node_path = path(&["leaf"]);
        let _ = wrap_mouse_area::<TestMessage>(
            Space::new().into(),
            None,
            None,
            None,
            None,
            false,
            &node_path,
        );
    }

    #[test]
    fn wrap_mouse_area_emits_enter_only_when_exit_is_disabled() {
        let node_path = path(&["leaf"]);
        let hover_events = Rc::new(RefCell::new(Vec::new()));
        let hover_for_cb = Rc::clone(&hover_events);
        let on_hover = move |tree_path: Option<TreePath>| {
            hover_for_cb.borrow_mut().push(tree_path.clone());
            TestMessage::Hover(tree_path)
        };

        let _ = wrap_mouse_area::<TestMessage>(
            Space::new().into(),
            None,
            None,
            None,
            Some(&on_hover),
            false,
            &node_path,
        );

        assert_eq!(*hover_events.borrow(), vec![Some(path(&["leaf"]))]);
    }

    #[test]
    fn wrap_mouse_area_hover_emits_enter_and_exit_events() {
        let node_path = path(&["leaf"]);
        let hover_events = Rc::new(RefCell::new(Vec::new()));
        let hover_cb = Rc::clone(&hover_events);
        let on_hover = move |tree_path: Option<TreePath>| {
            hover_cb.borrow_mut().push(tree_path.clone());
            TestMessage::Hover(tree_path)
        };

        let _ = wrap_mouse_area::<TestMessage>(
            Space::new().into(),
            None,
            None,
            None,
            Some(&on_hover),
            true,
            &node_path,
        );

        assert_eq!(*hover_events.borrow(), vec![Some(path(&["leaf"])), None]);
    }

    #[test]
    fn toggle_slot_not_built_without_content() {
        let nodes = one_folder_tree();
        let hover_events = Rc::new(RefCell::new(Vec::new()));
        let hover_cb = Rc::clone(&hover_events);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .row_interactive_filter(|_| false)
        .on_hover(move |tree_path| {
            hover_cb.borrow_mut().push(tree_path.clone());
            TestMessage::Hover(tree_path)
        })
        .view();

        assert_eq!(*hover_events.borrow(), vec![None]);
    }

    #[test]
    fn toggle_slot_emits_hover_for_folder_when_content_is_set() {
        let nodes = one_folder_tree();
        let hover_events = Rc::new(RefCell::new(Vec::new()));
        let hover_cb = Rc::clone(&hover_events);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .row_leading_content(|_| Space::new().into())
        .row_interactive_filter(|_| false)
        .on_hover(move |tree_path| {
            hover_cb.borrow_mut().push(tree_path.clone());
            TestMessage::Hover(tree_path)
        })
        .view();

        assert_eq!(*hover_events.borrow(), vec![Some(path(&["folder"])), None]);
    }

    #[test]
    fn row_leading_content_is_used_when_configured() {
        let nodes = one_file_tree();
        let row_leading_content_calls = Rc::new(Cell::new(0_usize));
        let row_leading_content_cb = Rc::clone(&row_leading_content_calls);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .row_leading_content(move |_| {
            row_leading_content_cb.set(row_leading_content_cb.get() + 1);
            Space::new().into()
        })
        .view();

        assert_eq!(row_leading_content_calls.get(), 1);
    }

    #[test]
    fn toggle_slot_emits_hover_for_files_when_content_is_set() {
        let nodes = one_file_tree();
        let hover_events = Rc::new(RefCell::new(Vec::new()));
        let hover_cb = Rc::clone(&hover_events);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .row_leading_content(|_| Space::new().into())
        .row_interactive_filter(|_| false)
        .on_hover(move |tree_path| {
            hover_cb.borrow_mut().push(tree_path.clone());
            TestMessage::Hover(tree_path)
        })
        .view();

        assert_eq!(*hover_events.borrow(), vec![Some(path(&["leaf"])), None]);
    }

    #[test]
    fn toggle_slot_can_use_hover_without_toggle_handler() {
        let nodes = one_file_tree();
        let hover_events = Rc::new(RefCell::new(Vec::new()));
        let hover_cb = Rc::clone(&hover_events);

        let _ = TreeView::<TestNode, TestMessage>::new(&nodes, |_| {
            Space::new().into()
        })
        .row_leading_content(|_| Space::new().into())
        .row_interactive_filter(|_| false)
        .on_hover(move |tree_path| {
            hover_cb.borrow_mut().push(tree_path.clone());
            TestMessage::Hover(tree_path)
        })
        .view();

        assert_eq!(*hover_events.borrow(), vec![Some(path(&["leaf"])), None]);
    }
}
