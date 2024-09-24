//! os/src/mem/page_table.rs
//!
//! 页表相关的数据结构和方法实现
//!
// TODO: 处理页分配异常

use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

use crate::{mem::address::VirtAddr, println};

use super::{
    address::{PhysPageNum, StepByOne, VirtPageNum},
    frame_allocator::FrameTracker,
};

bitflags! {
    pub struct PTEFlags: u8 {
        /// Valid 代表PTE是否有效
        const V = 1 << 0;
        /// Read 代表是否允许读
        const R = 1 << 1;
        /// Write 代表是否允许写
        const W = 1 << 2;
        /// Execute 代表是否允许执行
        const X = 1 << 3;
        /// User 代表是否允许用户态访问
        const U = 1 << 4;
        /// Global 代表是否是全局页
        const G = 1 << 5;
        /// Accessed 代表是否被访问过
        const A = 1 << 6;
        /// Dirty 代表是否被写过
        const D = 1 << 7;
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

pub struct PageTable {
    root_ppn: PhysPageNum,     // 页表根的物理页号
    frames: Vec<FrameTracker>, // 页表使用的物理页
}

impl PageTable {
    pub fn new() -> Self {
        // 初始化页表：分配一个物理页作为页表根
        let frame = super::frame_allocator::frame_alloc().expect("alloc frame failed");
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 建立页表映射
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        // 获取页表项的可变引用
        let pte = self.find_pte_create(vpn).unwrap();
        // 要建立映射的页表项必须是未被映射的
        assert!(!pte.is_valid(), "vpn {:?} has been mapped", vpn);
        // 建立映射
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// 解除页表映射
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        // 获取页表项的可变引用
        let pte = self.find_pte_create(vpn).expect("invalid vpn");
        // 要解除映射的页表项必须是已被映射的
        assert!(pte.is_valid(), "vpn {:?} has not been mapped", vpn);
        // 解除映射
        *pte = PageTableEntry::empty();
    }

    /// 根据虚拟页号返回页表项的可变引用    <br>
    /// 若页表项不存在则创建
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let index = vpn.indexes();
        let mut result: Option<&mut PageTableEntry> = None;

        let mut ppn = self.root_ppn; // 从根页表开始查找
        for i in 0..3 {
            let pte = &mut ppn.get_as_pte_array()[index[i]];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                // 页表项不存在，分配一个新的物理页作为页表
                let frame = super::frame_allocator::frame_alloc().expect("alloc frame failed");
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }

    /// 根据虚拟页号返回页表项的引用    <br>
    /// 若页表项不存在则返回None
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
        let index = vpn.indexes();
        let mut result: Option<&PageTableEntry> = None;

        let mut ppn = self.root_ppn; // 从根页表开始查找
        for i in 0..3 {
            let pte = &ppn.get_as_pte_array()[index[i]];
            if i == 2 {
                if pte.is_valid() {
                    result = Some(pte);
                }
                break;
            }
            if !pte.is_valid() {
                // 页表项不存在，直接返回None
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }

    // 手动MMU

    /// 从satp数据中获取页表根的物理页号
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// 从虚拟页号查找页表项
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| pte.clone())
    }

    /// 以satp数据格式取出页表（根页表）
    ///
    /// 用于设置页表寄存器satp
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_as_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_as_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// 实验：手动MMU
pub fn mmu_test() {
    println!("running mmu_test...");
    // 建立页表
    let mut page_table = PageTable::new();
    println!("PageTable created, root_ppn: {:?}", page_table.root_ppn);

    // 申请一个虚拟地址与物理地址的映射
    let vpn = VirtPageNum::from(0x2333);
    let frame = crate::mem::frame_allocator::frame_alloc().unwrap();
    page_table.map(vpn, frame.ppn, PTEFlags::R | PTEFlags::W);
    println!("VPN:{:?} -> {:?}", vpn, frame.ppn);

    // 查找映射
    let pte = page_table.translate(vpn).unwrap();
    assert!(pte.is_valid());
    assert!(pte.is_readable());
    assert!(pte.is_writable());
    assert!(pte.ppn() == frame.ppn);
    println!("Translate {:?} -> {:?}", vpn, pte.ppn());

    // 释放映射
    page_table.unmap(vpn);
    println!("Unmap {:?}", vpn);

    // 再次查找映射
    let pte = page_table.translate(vpn);
    assert!(pte.is_none());

    println!("mmu_test passed!");
}
