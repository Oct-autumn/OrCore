//! os/src/mem/page_table.rs
//!
//! 页表相关的数据结构和方法实现

use bitflags::*;

use super::address::PhysPageNum;

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;   // Valid 代表PTE是否有效
        const R = 1 << 1;   // Read 代表是否允许读
        const W = 1 << 2;   // Write 代表是否允许写
        const X = 1 << 3;   // Execute 代表是否允许执行
        const U = 1 << 4;   // User 代表是否允许用户态访问
        const G = 1 << 5;   // Global 代表是否是全局页
        const A = 1 << 6;   // Accessed 代表是否被访问过
        const D = 1 << 7;   // Dirty 代表是否被写过
    }
}

/// PageTableEntry是一个页表项，对应RISC-V sv39的硬件页表项
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    /// 创建一个新的页表项
    ///
    /// # 参数
    ///     - `ppn`：物理页号
    ///     - `flags`：页表项的标志位
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits() as usize,
        }
    }

    /// 创建一个空的页表项
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }

    /// 获取页表项的物理页号
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    /// 获取页表项的标志位
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    /// 判断页表项是否有效
    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::V)
    }

    /// 判断页表项是否可读
    pub fn is_readable(&self) -> bool {
        self.flags().contains(PTEFlags::R)
    }

    /// 判断页表项是否可写
    pub fn is_writable(&self) -> bool {
        self.flags().contains(PTEFlags::W)
    }

    /// 判断页表项是否可执行
    pub fn is_executable(&self) -> bool {
        self.flags().contains(PTEFlags::X)
    }
}
