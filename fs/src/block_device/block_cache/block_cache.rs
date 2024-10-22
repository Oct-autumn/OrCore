use crate::block_device::BlockDevice;
use crate::config::SECTOR_BYTES;
use alloc::sync::Arc;

pub struct BlockCache {
    /// 缓存数据
    cache: [u8; SECTOR_BYTES],
    /// 缓存块号
    block_id: usize,
    /// 存储设备接口指针
    block_device: Arc<dyn BlockDevice>,
    /// 是否被修改
    modified: bool,
}

impl BlockCache {
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        // 创建块缓存时读取块数据到缓存
        let mut cache = [0; SECTOR_BYTES];
        block_device.read_block(block_id, &mut cache);

        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    /// 偏移量处在内存中的地址
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    /// 获取缓存块在偏移量处以类型T的引用
    fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= SECTOR_BYTES);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }

    /// 获取缓存块在偏移量处以类型T的可变引用
    fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= SECTOR_BYTES);
        self.modified = true; // 因为可能会发生修改，所以将缓存块标记为已修改
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    /// 将缓存块写回块设备
    pub fn sync(&mut self) {
        if self.modified {
            self.block_device.write_block(self.block_id, &self.cache);
            self.modified = false;
        }
    }

    /// 读取缓存块在偏移量处的数据
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    /// 修改缓存块在偏移量处的数据
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        let ret = f(self.get_mut(offset));
        ret
    }

    /// 修改缓存块在偏移量处的数据，并立即写回块设备
    pub fn modify_and_sync<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        let ret = self.modify(offset, f);
        self.sync();
        ret
    }

    pub fn block_id(&self) -> usize {
        self.block_id
    }
}

impl Drop for BlockCache {
    /// 同样用到RAII思想，当BlockCache对象被销毁时，将缓存块写回块设备
    fn drop(&mut self) {
        self.sync();
    }
}
