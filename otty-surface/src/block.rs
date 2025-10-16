#[derive(Default, Debug)]
pub struct BlockSurface {}

impl BlockSurface {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
