/// Operating system command with raw arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatingSystemCommand {
    pub arguments: Vec<Vec<u8>>,
}
