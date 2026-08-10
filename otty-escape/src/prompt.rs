/// Semantic boundary emitted by OSC 133 prompt markers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptBoundary {
    /// Prompt rendering begins at the current terminal cursor.
    Start,
    /// Prompt rendering ends at the current terminal cursor.
    End,
}
