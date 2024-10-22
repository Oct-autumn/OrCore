use core::any::Any;

pub mod block_cache;

/// 用于实现块设备对接的trait
pub trait BlockDevice : Send + Sync + Any {
    fn read_block(&self, block_num: usize, buf: &mut [u8]);
    fn write_block(&self, block_num: usize, buf: &[u8]);
    fn num_blocks(&self) -> usize;
}