use std::mem::size_of;

use super::{Block, BlockState};
use crate::grid::Row;
use crate::{Cell, Dimensions};

/// Approximate memory retained by mutable and finished block content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockMemoryMetrics {
    active_block_count: usize,
    finished_block_count: usize,
    active_lines: usize,
    finished_lines: usize,
    active_bytes: usize,
    finished_bytes: usize,
}

impl BlockMemoryMetrics {
    /// Return the number of mutable blocks.
    pub fn active_block_count(&self) -> usize {
        self.active_block_count
    }

    /// Return the number of finished history blocks.
    pub fn finished_block_count(&self) -> usize {
        self.finished_block_count
    }

    /// Return logical lines retained by mutable blocks.
    pub fn active_lines(&self) -> usize {
        self.active_lines
    }

    /// Return logical lines retained by finished history.
    pub fn finished_lines(&self) -> usize {
        self.finished_lines
    }

    /// Return approximate bytes retained by mutable blocks.
    pub fn active_bytes(&self) -> usize {
        self.active_bytes
    }

    /// Return approximate bytes retained by finished history.
    pub fn finished_bytes(&self) -> usize {
        self.finished_bytes
    }

    /// Return approximate bytes retained by all blocks.
    pub fn total_bytes(&self) -> usize {
        self.active_bytes + self.finished_bytes
    }

    pub(super) fn from_blocks(blocks: &[Block]) -> Self {
        let mut metrics = Self::default();

        for block in blocks {
            let lines = block.surface.grid().total_lines();
            let bytes = estimated_block_bytes(block);
            if matches!(block.state, BlockState::Finished(_)) {
                metrics.finished_block_count += 1;
                metrics.finished_lines += lines;
                metrics.finished_bytes += bytes;
            } else {
                metrics.active_block_count += 1;
                metrics.active_lines += lines;
                metrics.active_bytes += bytes;
            }
        }

        metrics
    }
}

fn estimated_block_bytes(block: &Block) -> usize {
    let grid = block.surface.grid();
    let primary_lines = grid.total_lines();
    let inactive_lines = grid.screen_lines();
    let allocated_lines = primary_lines + inactive_lines;
    let cell_bytes = allocated_lines
        .saturating_mul(grid.columns())
        .saturating_mul(size_of::<Cell>());
    let row_bytes = allocated_lines.saturating_mul(size_of::<Row<Cell>>());
    let cached_text_bytes =
        block.cached_text.as_ref().map_or(0, |text| text.len());

    size_of::<Block>()
        .saturating_add(cell_bytes)
        .saturating_add(row_bytes)
        .saturating_add(estimated_metadata_bytes(block))
        .saturating_add(cached_text_bytes)
        .saturating_add(block.content.estimated_text_bytes())
}

fn estimated_metadata_bytes(block: &Block) -> usize {
    block.meta.id.capacity()
        + option_string_capacity(&block.meta.cmd)
        + option_string_capacity(&block.meta.cwd)
        + option_string_capacity(&block.meta.shell)
}

fn option_string_capacity(value: &Option<String>) -> usize {
    value.as_ref().map_or(0, String::capacity)
}
