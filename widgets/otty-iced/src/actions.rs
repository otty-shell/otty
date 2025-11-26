#[derive(Debug, Clone, PartialEq, Default)]
pub enum Action {
    Shutdown,
    ChangeTitle(String),
    ResetTitle,
    #[default]
    Ignore,
}
