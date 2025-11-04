#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hyperlink {
    /// Identifier for the given hyperlink.
    pub id: Option<String>,
    /// Resource identifier of the hyperlink.
    pub uri: String,
}
