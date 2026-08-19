use crate::widgets::quick_launch::types::{NodePath, QuickLaunch};

/// Quick launch wizard initialization payload bound to a newly opened tab.
#[derive(Debug, Clone)]
pub(crate) enum WizardTabInit {
    Create {
        parent_path: NodePath,
    },
    Edit {
        path: NodePath,
        command: Box<QuickLaunch>,
    },
}
