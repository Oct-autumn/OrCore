//! fs/src/ex_fat/cluster_chain/cluster_bitmap.rs
//!
//! 簇分配位图

use crate::block_device::block_cache::BlockCacheManager;
use crate::config;
use crate::ex_fat::boot_sector::BootSector;
use crate::ex_fat::model::cluster_id::ClusterId;
use alloc::sync::Arc;
use spin::Mutex;

type BitmapBlock = [u64; config::SECTOR_BYTES / size_of::<u64>()]; // 一个位图块

/// 簇分配位图
pub struct ClusterAllocBitmap {
    /// 已使用的簇数
    pub used_cluster_count: usize,
    /// 簇总数
    pub cluster_count: usize,
    /// 位图起始块号
    pub start_block_id: usize,
    /// 位图块数
    pub bitmap_blocks: usize,
    /// 缓存管理器
    block_cache_manager: Arc<Mutex<BlockCacheManager>>,
}

impl ClusterAllocBitmap {
    pub fn new(boot_sector: &BootSector, block_cache_manager: Arc<Mutex<BlockCacheManager>>) -> Self {
        let cluster_count = boot_sector.cluster_count as usize;
        let bitmap_blocks = (((cluster_count + 7) / 8) + (1 << boot_sector.bytes_per_sector_shift) - 1) >> boot_sector.bytes_per_sector_shift;

        /*
            以上代码等同于：
            let cluster_count = boot_sector.cluster_count.into();
            let bytes_per_sector = 1u32 << boot_sector.bytes_per_sector_shift;
            let bitmap_bytes = (cluster_count + 7) / 8;
            let bitmap_blocks: usize = bitmap_bytes / bytes_per_sector;
        */

        let start_block_id = boot_sector.cluster_heap_offset as usize;

        let used_cluster_count = {
            // <---- BCM独占区 开始 ---->
            let mut bcm_guard = block_cache_manager.lock();
            let mut count = 0;
            for block_offset in 0..bitmap_blocks {
                count += bcm_guard.get_block_cache(block_offset + start_block_id)
                    .read()
                    .read(0, |bitmap_block: &BitmapBlock| {
                        bitmap_block.iter().map(|bits64| bits64.count_ones() as usize).sum::<usize>()
                    });
            }
            count
            // <---- BCM独占区 结束 ---->
        };

        Self {
            used_cluster_count,
            cluster_count,
            start_block_id,
            bitmap_blocks,
            block_cache_manager,
        }
    }

    /// 分配一个簇
    /// # param
    /// - hint_cluster_id: 优先分配指定簇，若已被占用则从之后开始查找空闲bit
    /// # retval
    /// - Some(ClusterId): 分配的簇
    pub fn alloc(&mut self, hint_cluster_id: &ClusterId) -> Option<ClusterId> {
        let mut hint_cluster_id = hint_cluster_id.clone();
        if hint_cluster_id.0 < 2 || hint_cluster_id.0 as usize >= self.cluster_count + 2 {
            // 无效的hint_cluster_id，重置为2
            hint_cluster_id = ClusterId(2);
        }
        // 尝试直接分配hint_cluster_id
        let (block_pos, bits64_pos, inner_pos) = Self::translate_cluster_id_pos(&hint_cluster_id);
        if self.block_cache_manager.lock().get_block_cache(block_pos + self.start_block_id)
            .write()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                if bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0 {
                    // 簇已被占用
                    false
                } else {
                    // 簇未被占用
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                    true
                }
            }) {
            self.used_cluster_count += 1;
            return Some(hint_cluster_id);   // 返回分配的簇
        }

        { // 尝试直接分配指定簇失败，从之后开始查找空闲bit
            { // 搜索block_pos扇区后半部分的bit
                if let Some(pos) = self.block_cache_manager.lock().get_block_cache(block_pos + self.start_block_id)
                    .write()
                    .modify(0, |bitmap_block: &mut BitmapBlock| {
                        { // 单独搜索bitmap_block[bits64_pos]后半部分的bit
                            // 生成的掩码在或操作时，将inner_pos之前的bit置为1
                            let mask = !(u64::MAX << inner_pos);
                            let masked_u64 = bitmap_block[bits64_pos] | mask;
                            if masked_u64 != u64::MAX {
                                // 如果找得到，则分配
                                // 从低位开始找到第一个为0的bit
                                let inner_pos = masked_u64.trailing_ones() as usize;
                                bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                                return Some(block_pos * config::SECTOR_BITS + bits64_pos * 64 + inner_pos);
                            }
                        }

                        // 如果bitmap_block[bits64_pos]没有剩余bit，则搜索bitmap_block剩下的bit
                        if let Some((bits64_pos, inner_pos)) = bitmap_block[bits64_pos + 1..]
                            .iter()
                            .enumerate()
                            .find(|(_, bits64)| (**bits64 & u64::MAX) != 0) // 找到一个非满的块
                            .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))   // 后缀1的个数将指示从低位开始第一个为0的bit的位置
                        {
                            // 如果找得到，则分配
                            bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                            return Some(block_pos * config::SECTOR_BITS + bits64_pos * 64 + inner_pos);
                        }
                        None
                    }) {
                    self.used_cluster_count += 1;
                    return Some(ClusterId(pos as u32 + 2));    // 返回分配的簇
                }
            }


            // 从下一个扇区开始搜索
            for block_offset in 1..self.bitmap_blocks {
                if let Some(pos) = self.block_cache_manager.lock().get_block_cache((block_pos + block_offset) % self.bitmap_blocks + self.start_block_id)
                    .write()
                    .modify(0, |bitmap_block: &mut BitmapBlock| {
                        if let Some((bits64_pos, inner_pos)) = bitmap_block
                            .iter()
                            .enumerate()
                            .find(|(_, bits64)| (**bits64 != u64::MAX)) // 找到一个非满的块
                            .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))   // 后缀1的个数将指示从低位开始第一个为0的bit的位置
                        {
                            // 如果找得到，则分配
                            bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                            Some(block_offset * config::SECTOR_BITS + bits64_pos * 64 + inner_pos)
                        } else {
                            // 找不到则返回None
                            None
                        }
                    }) {
                    self.used_cluster_count += 1;
                    return Some(ClusterId(pos as u32 + 2));    // 返回分配的簇
                }
            }

            { // 搜索block_pos扇区前半部分的bit
                if let Some(pos) = self.block_cache_manager.lock().get_block_cache(block_pos + self.start_block_id)
                    .write()
                    .modify(0, |bitmap_block: &mut BitmapBlock| {
                        // 如果bitmap_block[bits64_pos]没有剩余bit，则搜索bitmap_block剩下的bit
                        if let Some((bits64_pos, inner_pos)) = bitmap_block[..bits64_pos]
                            .iter()
                            .enumerate()
                            .find(|(_, bits64)| (**bits64 & u64::MAX) != 0) // 找到一个非满的块
                            .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))   // 后缀1的个数将指示从低位开始第一个为0的bit的位置
                        {
                            // 如果找得到，则分配
                            bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                            return Some(block_pos * config::SECTOR_BITS + bits64_pos * 64 + inner_pos);
                        }

                        { // 单独搜索bitmap_block[bits64_pos]前半部分的bit
                            // 生成的掩码在或操作时，将inner_pos之后的bit置为1
                            let mask = u64::MAX << inner_pos;
                            let masked_u64 = bitmap_block[bits64_pos] | mask;
                            if masked_u64 != u64::MAX {
                                // 如果找得到，则分配
                                let inner_pos = masked_u64.trailing_ones() as usize;    // 后缀1的个数将指示从低位开始第一个为0的bit的位置
                                bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                                return Some(block_pos * config::SECTOR_BITS + bits64_pos * 64 + inner_pos);
                            }
                        }

                        None
                    }) {
                    self.used_cluster_count += 1;
                    return Some(ClusterId(pos as u32 + 2));    // 返回分配的簇
                }
            }
        }

        panic!("No free bit in bitmap");
    }

    /// 释放一个簇
    pub fn free(&mut self, cluster_id: &ClusterId) {
        let (block_pos, bits64_pos, inner_pos) = Self::translate_cluster_id_pos(cluster_id);
        self.block_cache_manager.lock().get_block_cache(block_pos + self.start_block_id)
            .write()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);    // 断言：要释放的bit必须已被分配
                bitmap_block[bits64_pos] ^= 1u64 << inner_pos;
            });
        self.used_cluster_count -= 1;
    }

    /// 判断簇是否被使用
    pub fn is_allocated(&self, cluster_id: &ClusterId) -> bool {
        let (block_pos, bits64_pos, inner_pos) = Self::translate_cluster_id_pos(cluster_id);
        self.block_cache_manager.lock().get_block_cache(block_pos + self.start_block_id)
            .read()
            .read(0, |bitmap_block: &BitmapBlock| {
                bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0
            })
    }

    /// 将bit地址转换为相对盘块号、64位位图块号和64位位图块内偏移
    fn translate_cluster_id_pos(cluster_id: &ClusterId) -> (usize, usize, usize) {
        // 由于FAT表的簇号从2开始，因此映射到bitmap时要减2
        let mut bit = cluster_id.0 as usize - 2;
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
