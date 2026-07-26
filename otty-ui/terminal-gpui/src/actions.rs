use gpui::actions;

actions!(
    otty_terminal,
    [
        Copy,
        Paste,
        SelectAll,
        ClearSelection,
        ScrollPageUp,
        ScrollPageDown,
        ScrollToTop,
        ScrollToBottom,
    ]
);
