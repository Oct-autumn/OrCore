mod block_cache;

use crate::block_device::BlockDevice;
use alloc::collections::{BTreeMap, LinkedList};
use alloc::sync::Arc;
pub use block_cache::BlockCache;
use spin::RwLock;
use crate::config::SECTOR_BYTES;

/// 缓存块管理器
///
/// 我们将实现一个LRU缓存淘汰算法
/// 当缓存块满时，将淘汰最近最少使用的缓存块
pub struct BlockCacheManager {
    /// 缓存块块号映射(块号 -> 缓存块)
    caches: BTreeMap<usize, Arc<RwLock<BlockCache>>>,
    /// 缓存块List(最近最少使用的缓存块在list头部)(内部为一个元组：(块号, 缓存块))
    list: LinkedList<(usize, Arc<RwLock<BlockCache>>)>,
    /// 存储设备接口指针
    block_device: Arc<dyn BlockDevice>,
    /// 最大缓存块数（默认为16）
    ///
    /// 当缓存块数超过最大缓存块数时，将执行缓存块淘汰
    max_cache_blocks: usize,
}

impl BlockCacheManager {
    pub fn new(device: Arc<dyn BlockDevice>) -> Self {
        Self {
            caches: BTreeMap::new(),
            list: LinkedList::new(),
            block_device: device,
            max_cache_blocks: 16,
        }
    }

    pub fn set_max_cache_blocks(&mut self, max_cache_blocks: usize) {
        assert!(max_cache_blocks > 0);
        self.max_cache_blocks = max_cache_blocks;
    }

    /// 淘汰缓存块
    fn disuse(&mut self) -> bool {
        // 移除最近最少使用的缓存块（移除list头节点）
        if let Some(block_cache) = self.list.pop_front() {
            let min_use_block_id = block_cache.0;
            // 从缓存块Map中删除最少使用的缓存块
            self.caches.remove(&min_use_block_id);
            // TODO: 被删除的块可能被持有引用而不会立即释放，这很危险，可能导致数据不一致
            true
        } else {
            false
        }
    }

    /// 将缓存块移动到链表尾
    fn move_to_tail(&mut self, block_id: usize) {
        let mut index = 0;
        // 找到缓存块在list中的位置
        for (i, c) in self.list.iter().enumerate() {
            if c.0 == block_id {
                index = i;
                break;
            }
        }
        let mut list = self.list.split_off(index);
        list.pop_front();
        self.list.append(&mut list);
        self.list
            .push_back((block_id, self.caches.get(&block_id).unwrap().clone()));
    }

    /// 获取缓存块
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
    ) -> Arc<RwLock<BlockCache>> {
        if let Some(cache) = self.caches.get(&block_id) {
            // 将缓存块移动到list尾部
            let cache = cache.clone();
            self.move_to_tail(block_id);
            cache.clone()
        } else {
            let cache = Arc::new(RwLock::new(BlockCache::new(block_id, self.block_device.clone())));
            if self.caches.len() >= 16 {
                // 缓存块满时，淘汰最近最少使用的缓存块
                if !self.disuse() {
                    panic!("Failed to disuse block cache!");
                }
            }
            self.caches.insert(block_id, cache.clone());
            self.list.push_back((block_id, cache.clone()));
            cache
        }
    }

    /// 将所有缓存块写回块设备
    pub fn sync_all(&mut self) {
        self.caches.iter_mut().for_each(|(_, cache)| {
            let mut cache = cache.write();
            cache.sync();
        });
    }
    
    /// 绕过缓存，直接清零某个块
    pub fn direct_set_zero(&mut self, block_id: usize) {
        self.block_device.write_block(block_id, &[0; SECTOR_BYTES]);
    }
}
