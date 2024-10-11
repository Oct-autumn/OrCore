mod block_cache;

use crate::block_dev::BlockDevice;
use alloc::collections::{BTreeMap, LinkedList};
use alloc::sync::Arc;
pub use block_cache::BlockCache;
use lazy_static::lazy_static;
use spin::{Mutex, RwLock};

lazy_static! {
    /// 缓存块管理器
    static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(BlockCacheManager::new());
}

/// 缓存块管理器
///
/// 我们将实现一个LRU缓存淘汰算法
/// 当缓存块满时，将淘汰最近最少使用的缓存块
pub struct BlockCacheManager {
    /// 缓存块块号映射(块号 -> 缓存块)
    caches: BTreeMap<usize, Arc<RwLock<BlockCache>>>,
    /// 缓存块List(最近最少使用的缓存块在list头部)(内部为一个元组：(块号, 缓存块))
    list: LinkedList<(usize, Arc<RwLock<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            caches: BTreeMap::new(),
            list: LinkedList::new(),
        }
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
        block_device: &Arc<dyn BlockDevice>,
    ) -> Arc<RwLock<BlockCache>> {
        if let Some(cache) = self.caches.get(&block_id) {
            // 将缓存块移动到list尾部
            let cache = cache.clone();
            self.move_to_tail(block_id);
            cache.clone()
        } else {
            let cache = Arc::new(RwLock::new(BlockCache::new(block_id, block_device.clone())));
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

    // TODO: debug
    pub fn print_cache_block(&self, block_id: usize) {
        if let Some(cache) = self.caches.get(&block_id) {
            let cache = cache.read();
            cache.read(0, |data: &[u8; 512]| {
                for i in 0..512 {
                    print!("{:02x} ", data[i]);
                    if i % 32 == 31 {
                        println!();
                    }
                }
            });
        } else {
            println!("block_id: {} not found", block_id);
        }
    }
}

/// 获取缓存块
pub fn get_block_cache(
    block_id: usize,
    block_device: &Arc<dyn BlockDevice>,
) -> Arc<RwLock<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}

/// 将所有缓存块写回块设备
pub fn sync_all() {
    BLOCK_CACHE_MANAGER.lock().sync_all();
}

pub fn print_all_cache() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (block_id, cache) in manager.caches.iter() {
        println!("block_id: {}", block_id);
        let cache = cache.read();
        cache.read(0, |data: &[u8; 512]| {
            for i in 0..512 {
                print!("{:02x} ", data[i]);
                if i % 32 == 31 {
                    println!();
                }
            }
        });
    }
}
