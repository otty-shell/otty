use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use super::{Block, BlockId};

pub(super) struct BlockList {
    blocks: Vec<Block>,
    block_id_to_index: HashMap<BlockId, usize>,
}

impl BlockList {
    pub(super) fn len(&self) -> usize {
        self.blocks.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub(super) fn get(&self, index: usize) -> Option<&Block> {
        self.blocks.get(index)
    }

    pub(super) fn get_mut(&mut self, index: usize) -> Option<&mut Block> {
        self.blocks.get_mut(index)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }

    pub(super) fn index_of(&self, block_id: &BlockId) -> Option<usize> {
        self.block_id_to_index.get(block_id).copied()
    }

    pub(super) fn contains(&self, block_id: &BlockId) -> bool {
        self.block_id_to_index.contains_key(block_id)
    }

    pub(super) fn as_slice(&self) -> &[Block] {
        &self.blocks
    }

    pub(super) fn new(initial_block: Block) -> Self {
        let block_id_to_index = HashMap::from([(initial_block.id.clone(), 0)]);

        Self {
            blocks: vec![initial_block],
            block_id_to_index,
        }
    }

    pub(super) fn append(&mut self, block: Block) -> bool {
        if self.contains(&block.id) {
            return false;
        }

        let index = self.blocks.len();
        self.block_id_to_index.insert(block.id.clone(), index);
        self.blocks.push(block);

        true
    }

    pub(super) fn remove(&mut self, index: usize) -> Option<Block> {
        if index >= self.blocks.len() {
            return None;
        }

        let removed = self.blocks.remove(index);
        self.rebuild_index();

        Some(removed)
    }

    fn rebuild_index(&mut self) {
        self.block_id_to_index.clear();
        self.block_id_to_index.extend(
            self.blocks
                .iter()
                .enumerate()
                .map(|(index, block)| (block.id.clone(), index)),
        );
    }

    #[cfg(test)]
    pub(super) fn index_len(&self) -> usize {
        self.block_id_to_index.len()
    }

    #[cfg(test)]
    fn reorder(&mut self, from: usize, to: usize) -> bool {
        if from >= self.blocks.len() || to >= self.blocks.len() {
            return false;
        }
        if from == to {
            return true;
        }

        let block = self.blocks.remove(from);
        self.blocks.insert(to, block);
        self.rebuild_index();

        true
    }
}

impl Index<usize> for BlockList {
    type Output = Block;

    fn index(&self, index: usize) -> &Self::Output {
        &self.blocks[index]
    }
}

impl IndexMut<usize> for BlockList {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.blocks[index]
    }
}

impl<'a> IntoIterator for &'a BlockList {
    type Item = &'a Block;
    type IntoIter = std::slice::Iter<'a, Block>;

    fn into_iter(self) -> Self::IntoIter {
        self.blocks.iter()
    }
}

impl<'a> IntoIterator for &'a mut BlockList {
    type Item = &'a mut Block;
    type IntoIter = std::slice::IterMut<'a, Block>;

    fn into_iter(self) -> Self::IntoIter {
        self.blocks.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::BlockList;
    use crate::block::{Block, BlockId, BlockMeta};
    use crate::{Dimensions, SurfaceConfig};

    struct TestDimensions;

    impl Dimensions for TestDimensions {
        fn total_lines(&self) -> usize {
            2
        }

        fn screen_lines(&self) -> usize {
            2
        }

        fn columns(&self) -> usize {
            4
        }
    }

    fn block(id: impl Into<String>) -> Block {
        let id = id.into();
        Block::new(
            &SurfaceConfig::default(),
            &TestDimensions,
            BlockMeta {
                id,
                ..BlockMeta::default()
            },
        )
    }

    fn assert_index_matches_linear_search(list: &BlockList) {
        assert_eq!(list.index_len(), list.len());
        for (expected_index, block) in list.iter().enumerate() {
            assert_eq!(list.index_of(&block.id), Some(expected_index));
        }
    }

    #[test]
    fn index_matches_linear_search_after_append_remove_and_reorder() {
        let mut list = BlockList::new(block("bootstrap"));
        let mut seed = 0x5eed_u64;
        let mut next_id = 0_u64;

        for _ in 0..512 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            match seed % 3 {
                0 => {
                    next_id += 1;
                    assert!(list.append(block(format!("block-{next_id}"))));
                },
                1 if list.len() > 1 => {
                    let index = (seed as usize) % list.len();
                    list.remove(index);
                },
                2 if list.len() > 1 => {
                    let from = (seed as usize) % list.len();
                    let to = ((seed >> 16) as usize) % list.len();
                    list.reorder(from, to);
                },
                _ => {},
            }

            assert_index_matches_linear_search(&list);
        }

        assert!(list.index_of(&BlockId::new("missing")).is_none());
    }
}
