//! os/src/mem/frame_allocator.rs
//!
//! 页帧分配器定义与方法实现

use alloc::{collections::vec_deque::VecDeque, vec::Vec};

use crate::{config::MEMORY_END, mem::address::PhysAddr, println, sync::UPSafeCell};

use super::address::PhysPageNum;

use lazy_static::lazy_static;

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

pub struct StackFrameAllocator {
    current: usize, //空闲内存的起始物理页号
    end: usize,     //空闲内存的结束物理页号
    recycled: VecDeque<usize>,
}

impl StackFrameAllocator {
    /// 初始化空闲内存的起始和结束物理页号
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }

    /// 获取受管理的物理页号范围
    pub fn get_range(&self) -> (usize, usize) {
        (self.current, self.end)
    }

    /// 获取已经分配的物理页号量
    #[allow(unused)]
    pub fn get_allocated(&self) -> usize {
        self.current - self.recycled.len()
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        StackFrameAllocator {
            current: 0,
            end: 0,
            recycled: VecDeque::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        // 优先使用回收的物理页号
        // 最早回收的物理页号在队列的前面
        if let Some(ppn) = self.recycled.pop_front() {
            Some(ppn.into())
        } else {
            if self.current == self.end {
                // 若没有空闲的物理页号，则返回None，表示分配失败
                None
            } else {
                // 若没有回收的物理页号，则分配新的物理页号
                self.current += 1;
                Some((self.current - 1).into())
            }
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // 页号有效性检查（超越当前物理页号或已经被回收）
        if ppn >= self.current || self.recycled.iter().find(|&v| *v == ppn).is_some() {
            panic!(
                "Frame ppn={:#x} has not been allocated! But requested dealloc ",
                ppn
            );
        }
        // 推入回收栈
        self.recycled.push_back(ppn);
    }
}

/// 物理页帧追踪器
///
/// 用于绑定物理页的生命周期
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // 清空页
        ppn.fill_zero();

        Self { ppn }
    }
}

impl Drop for FrameTracker {
    /// 基于RAII的思想，将物理页的生命周期绑定于FrameTracker上 <br>
    /// 当FrameTracker被drop时，物理页将会自动回收
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

// 全局的FrameAllocator实例
lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// 初始化页帧分配器
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(), // 从内核结束地址开始分配（标识符见os/src/linker-qemu(k210).ld）
        PhysAddr::from(MEMORY_END).floor(),      // 到内存结束地址结束分配
    );
}

/// 分配一个物理页  <br>
/// 若分配成功，则返回一个FrameTracker  <br>
/// 若分配失败，则返回None
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(|ppn| FrameTracker::new(ppn))
}

/// 回收一个物理页
fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

/// 页帧分配器测试  <br>
/// 测试分配和回收物理页
#[allow(unused)]
pub fn frame_alloc_test() {
    println!("running frame_test...");

    let (s, e) = FRAME_ALLOCATOR.exclusive_access().get_range();
    println!("all available frame range: [{:x} - {:x}]", s, e);

    let mut alloced_frames = Vec::new();
    for _ in 0..10 {
        let frame = frame_alloc().unwrap();
        alloced_frames.push(frame);
    }
    alloced_frames.clear();
    for _ in 0..10 {
        let frame = frame_alloc().unwrap();
        alloced_frames.push(frame);
    }
    drop(alloced_frames);

    println!("frame_test passed!");
}
