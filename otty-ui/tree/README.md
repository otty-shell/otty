# otty-ui-tree

`otty-ui-tree` is a small tree widget for `iced` plus tree flatten/sort helpers.

The crate has two layers:

- model layer: `TreeNode`, `TreePath`, `flatten_tree`
- view layer: `TreeView`, `TreeRowContext`

## Installation

If you use workspace dependencies:

```toml
[workspace.dependencies]
otty-ui-tree = { path = "otty-ui/tree" }
```

And in your crate:

```toml
[dependencies]
otty-ui-tree = { workspace = true }
```

Or add it directly:

```toml
[dependencies]
otty-ui-tree = { path = "../otty-ui/tree" }
```

## Basic Usage

```rust
use iced::widget::{Space, container, text};
use iced::{Element, Length};
use otty_ui_tree::{TreeNode, TreePath, TreeRowContext, TreeView};

#[derive(Clone)]
enum Node {
    Folder { title: String, expanded: bool, children: Vec<Node> },
    File { title: String },
}

impl TreeNode for Node {
    fn title(&self) -> &str {
        match self {
            Node::Folder { title, .. } => title,
            Node::File { title } => title,
        }
    }

    fn children(&self) -> Option<&[Self]> {
        match self {
            Node::Folder { children, .. } => Some(children),
            Node::File { .. } => None,
        }
    }

    fn expanded(&self) -> bool {
        matches!(self, Node::Folder { expanded: true, .. })
    }

    fn is_folder(&self) -> bool {
        matches!(self, Node::Folder { .. })
    }
}

#[derive(Clone, Debug)]
enum Message {
    Press(TreePath),
    Hover(Option<TreePath>),
}

struct State {
    tree: Vec<Node>,
    selected: Option<TreePath>,
    hovered: Option<TreePath>,
}

fn view(state: &State) -> Element<'_, Message> {
    TreeView::new(&state.tree, |context| {
        container(text(context.entry.node.title()))
            .width(Length::Fill)
            .padding([4, 8])
            .into()
    })
    .selected_row(state.selected.as_ref())
    .hovered_row(state.hovered.as_ref())
    .on_press(Message::Press)
    .on_hover(Message::Hover)
    .row_leading_content(|context| {
        if context.entry.node.is_folder() {
            text("> ").into()
        } else {
            Space::new().width(Length::Fixed(12.0)).into()
        }
    })
    .indent_size(14.0)
    .spacing(0.0)
    .view()
}
```

Full runnable example: `examples/tree_view.rs`.

## API Concepts

- `selected_row` and `hovered_row` are render input state.
- `on_press` and `on_hover` are output events for your update loop.
- `row_leading_content` is an extra slot inside each row, before main content.
- `before_row` and `after_row` insert separate elements around a row.

## Filters

- `row_visible_filter`: hides the main row.
  `before_row` and `after_row` can still render for that entry.
- `row_interactive_filter`: keeps the row visible but disables row-level mouse handlers.

## Flatten and Sorting

`flatten_tree` produces a depth-first list of visible rows and sorts each level by:

- folders first, then files
- natural title ordering (numeric-aware and case-insensitive first)

## Path Model

`TreePath` is built from `title()` values from root to row.
This is convenient for selection/hover but assumes sibling titles are unique.

## When to Use before_row and after_row

Use these hooks for separate rows around an entry, for example:

- inline edit row under an entry (`after_row`)
- separator or label before an entry (`before_row`)

If you need content inside the row (icon, chevron, badge), use `row_leading_content`.
