use crate::block_device::block_cache::BlockCacheManager;
use crate::config;
use alloc::sync::Arc;
use spin::Mutex;
use crate::ex_fat::model::cluster_id::ClusterId;

type BitmapBlock = [u64; config::SECTOR_BYTES / size_of::<u64>()]; // 一个位图块

/// 位图
pub struct Bitmap {
    /// 位图起始块号
    start_block_id: usize,
    /// 位图块数
    blocks: usize,
    /// 缓存管理器
    block_cache_manager: Arc<Mutex<BlockCacheManager>>,
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize, block_cache_manager: Arc<Mutex<BlockCacheManager>>) -> Bitmap {
        Bitmap {
            start_block_id,
            blocks,
            block_cache_manager,
        }
    }

    /// 获取位图起始块号
    pub fn get_start_block_id(&self) -> usize {
        self.start_block_id
    }

    /// 获取块数
    pub fn get_blocks(&self) -> usize {
        self.blocks
    }

    /// 分配一个bit
    /// # param
    /// - hint_cluster_id: 指定从某处开始查找可用簇，若为无效值则从头开始查找
    ///
    /// - 成功则返回bit位置（分配出的bit在位图中的offset）
    /// - 失败则返回None
    pub fn alloc(&self, hint_cluster_id: &ClusterId) -> Option<usize> {
        // <---- BCM独占区 开始 ---->
        let mut bcm_guard = self.block_cache_manager.lock();
        // 遍历位图块
        for block_id in 0..self.blocks {
            if let Some(pos) = bcm_guard.get_block_cache(
                block_id + self.start_block_id
            )
                .write()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    if let Some((bits64_pos, inner_pos)) = bitmap_block
                        .iter()
                        .enumerate()
                        .find(|(_, bits64)| (!(**bits64 & u64::MAX)) != 0) // 找到一个非满的块
                        .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                    {
                        // 如果找得到，则分配
                        bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                        Some(block_id * config::SECTOR_BITS + bits64_pos * 64 + inner_pos)
                    } else {
                        // 找不到则返回None
                        None
                    }
                }) {
                return Some(pos);
                // <---- BCM独占区 结束 ---->
            }
        }
        panic!("No free bit in bitmap");
    }

    /// 释放一个bit
    pub fn free(&self, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = Self::translate_bit_pos(bit);
        self.block_cache_manager.lock().get_block_cache(block_pos + self.start_block_id)
            .write()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);    // 断言：要释放的bit必须已被分配
                bitmap_block[bits64_pos] ^= 1u64 << inner_pos;
            });
    }

    /// 查询一个bit是否已被分配
    pub fn is_allocated(&self, bit: usize) -> bool {
        let (block_pos, bits64_pos, inner_pos) = Self::translate_bit_pos(bit);
        self.block_cache_manager.lock().get_block_cache(block_pos + self.start_block_id)
            .read()
            .read(0, |bitmap_block: &BitmapBlock| {
                bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0
            })
    }

    /// 获取已分配的bit数
    pub fn get_allocated_count(&self) -> usize {
        // <---- BCM独占区 开始 ---->
        let mut bcm_guard = self.block_cache_manager.lock();
        let mut count = 0;
        for block_offset in 0..self.blocks {
            count += bcm_guard.get_block_cache(block_offset + self.start_block_id)
                .read()
                .read(0, |bitmap_block: &BitmapBlock| {
                    bitmap_block.iter().map(|bits64| bits64.count_ones() as usize).sum::<usize>()
                });
        }
        count
        // <---- BCM独占区 结束 ---->
    }

    /// 将bit地址转换为块号和块内偏移
    fn translate_bit_pos(mut bit: usize) -> (usize, usize, usize) {
        let block_pos = bit & !(config::SECTOR_BITS - 1);
        bit = bit & (config::SECTOR_BITS - 1);
        (block_pos, bit >> 6, bit & 0x3F)

        /*
            以上代码等同于：
            // 相对盘块号
            let block_pos = bit / BLOCK_BITS;
            // 64位位图块号
            let bits64_pos = (bit % BLOCK_BITS) / 64;
            // 64位位图块内偏移
            let inner_pos = (bit % BLOCK_BITS) % 64;
            (block_pos, bits64_pos, inner_pos)
        */
    }
}
