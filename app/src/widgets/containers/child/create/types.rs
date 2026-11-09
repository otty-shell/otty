/// One text field of a create dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormField {
    /// The image of a pull, or the name of a volume or network.
    Name,
    /// The tag of a pull.
    Tag,
    /// The driver of a volume or network.
    Driver,
}
