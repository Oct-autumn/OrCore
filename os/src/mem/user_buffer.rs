use alloc::vec::Vec;

/// 用户缓冲区
pub struct UserBuffer {
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    /// 创建一个用户缓冲区
    ///
    /// 将调用translated_byte_buffer的结果封装成一个用户缓冲区
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }

    /// 获取缓冲区的长度
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }
    
    pub fn get_ref_at(&self, index: usize) -> Option<&u8> {
        let mut offset = 0;
        for b in self.buffers.iter() {
            if index < offset + b.len() {
                return Some(&b[index - offset]);
            }
            offset += b.len();
        }
        None
    }
    
    pub fn get_mut_at(&mut self, index: usize) -> Option<&mut u8> {
        let mut offset = 0;
        for b in self.buffers.iter_mut() {
            if index < offset + b.len() {
                return Some(&mut b[index - offset]);
            }
            offset += b.len();
        }
        None
    }
}

/// 用户缓冲区迭代器（用于支持逐字节访问）
pub struct UserBufferIter {
    buffers: Vec<&'static mut [u8]>,
    index: usize,
    offset: usize,
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIter;

    fn into_iter(self) -> Self::IntoIter {
        UserBufferIter {
            buffers: self.buffers,
            index: 0,
            offset: 0,
        }
    }
}

impl Iterator for UserBufferIter {
    type Item = *mut u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.buffers.len() {
            // 如果已经遍历完所有缓冲区，则返回None
            return None;
        }
        
        // 获取数据指针
        let buffer = &mut self.buffers[self.index];
        let ptr = unsafe { buffer.as_mut_ptr().add(self.offset) };

        // 更新索引
        self.offset += 1;
        if self.offset >= buffer.len() {
            self.index += 1;
            self.offset = 0;
        }
        
        Some(ptr)
    }
}