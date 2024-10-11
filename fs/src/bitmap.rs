use crate::block_cache::get_block_cache;
use crate::block_dev::BlockDevice;
use crate::config;
use alloc::sync::Arc;

type BitmapBlock = [u64; config::SECTOR_BYTES / size_of::<u64>()]; // 一个位图块

/// 位图
pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Bitmap {
        Bitmap {
            start_block_id,
            blocks,
        }
    }

    /// 初始化位图
    /// 
    /// mut仅用于标记此操作会修改位图内容
    pub fn init(&mut self, block_device: &Arc<dyn BlockDevice>) {
        for block_id in 0..self.blocks {
            get_block_cache(block_id + self.start_block_id, block_device)
                .write()
                .modify(0, |data: &mut [u8; config::SECTOR_BYTES]| {
                    data.fill(0);   // 初始化为0
                });
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
    ///
    /// - 成功则返回bit位置（分配出的bit在位图中的offset）
    /// - 失败则返回None
    /// 
    /// mut仅用于标记此操作会修改位图内容
    pub fn alloc(&mut self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        // 遍历位图块
        for block_id in 0..self.blocks {
            let pos = get_block_cache(
                block_id + self.start_block_id as usize,
                block_device,
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
                        Some(block_id * config::SECTOR_BITS + bits64_pos * 64 + inner_pos as usize)
                    } else {
                        // 找不到则返回None
                        None
                    }
                });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    /// 释放一个bit
    /// 
    /// mut仅用于标记此操作会修改位图内容
    pub fn free(&mut self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = Self::decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, block_device)
            .write()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] ^= 1u64 << inner_pos;
            });
    }

    /// 查询一个bit是否已被分配
    pub fn is_used(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) -> bool {
        let (block_pos, bits64_pos, inner_pos) = Self::decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, block_device)
            .read()
            .read(0, |bitmap_block: &BitmapBlock| {
                bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0
            })
    }

    /// 将bit地址转换为块号和块内偏移
    fn decomposition(mut bit: usize) -> (usize, usize, usize) {
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
