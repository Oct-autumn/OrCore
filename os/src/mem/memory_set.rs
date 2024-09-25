//! os/src/mem/memory_set.rs
//!
//! 内存空间与逻辑段

use core::{arch::asm, cmp::min};

use alloc::{collections::btree_map::BTreeMap, format, vec::Vec};
use bitflags::bitflags;
use log::error;
use riscv::register::satp;

use crate::{
    config::{self},
    error::{self, process, Error, ErrorKind, MsgType, Result},
    new_error, println,
};

use super::{
    address::{PhysAddr, PhysPageNum, VPNRange, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
    page_table::{PTEFlags, PageTable, PageTableEntry},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SegType {
    /// 直接映射
    Identical,
    /// 帧映射
    Framed,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct SegPermission: u8 {
        /// 可读
        const R = 1 << 1;
        /// 可写
        const W = 1 << 2;
        /// 可执行
        const X = 1 << 3;
        /// 用户态可读
        const U = 1 << 4;
    }
}

/// 地址空间中的一个区域（相当于一个“段”）
pub struct LogicalSegment {
    /// 虚拟页号区段
    vpn_range: VPNRange,
    /// 包含的数据帧（使用BTreeMap保存`索引-值`对，方便查找）
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    /// 映射类型
    seg_type: SegType,
    /// 映射权限
    seg_perm: SegPermission,
}

impl LogicalSegment {
    /// 新建逻辑段
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        seg_type: SegType,
        seg_perm: SegPermission,
    ) -> Self {
        Self {
            vpn_range: VPNRange::new(start_va.floor(), end_va.ceil()),
            data_frames: BTreeMap::new(),
            seg_type,
            seg_perm,
        }
    }

    /// 将单个虚拟页映射到物理页上
    fn map_single(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) -> Result<()> {
        let ppn: PhysPageNum;
        match self.seg_type {
            SegType::Identical => {
                // 直接映射
                // 直接映射不涉及帧的分配和释放，无需额外操作，直接使用虚拟页号作为物理页号
                ppn = PhysPageNum::from(vpn.0);
            }
            SegType::Framed => {
                // 帧映射
                // 先申请分配一个物理页，然后映射，最后将其保存到`data_frames`中维持生命周期
                if let Some(frame) = frame_alloc() {
                    ppn = frame.ppn;
                    self.data_frames.insert(vpn, frame);
                } else {
                    return Err(new_error!(
                        ErrorKind::Mem(error::mem::ErrorKind::OutOfMemory),
                        MsgType::String(format!("alloc frame failed for vpn={:x}", vpn.0))
                    ));
                }
            }
        }
        let frame_flags = PTEFlags::from_bits(self.seg_perm.bits()).unwrap(); // 将段权限转换为页表项标志位
        page_table.map(vpn, ppn, frame_flags)
    }

    /// 解除单个虚拟页的映射
    #[allow(unused)]
    fn unmap_single(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) -> Result<()> {
        match self.seg_type {
            SegType::Framed => {
                // 帧映射
                // 需要从`data_frames`中删除对应的帧
                //（巧妙的设计：当帧被从data_frames中移除时，FrameTracker对应的Drop方法便会被触发，从而解除物理帧的分配）
                self.data_frames.remove(&vpn);
            }
            _ => {
                // 直接映射不涉及帧的分配和释放，无需额外操作
            }
        }
        page_table.unmap(vpn)
    }

    /// 将逻辑段内的虚拟页映射到物理页上
    fn map_all(&mut self, page_table: &mut PageTable) -> Result<()> {
        for vpn in self.vpn_range {
            // 逐帧映射
            if let Err(e) = self.map_single(page_table, vpn) {
                // 如果映射失败，解除之前的映射
                for vpn_unmap in self.vpn_range {
                    if vpn_unmap >= vpn {
                        break;
                    }
                    assert!(self.unmap_single(page_table, vpn).is_ok()); // 可保证只解除已经映射的页，不会出错
                }
                return Err(e);
            }
        }
        Ok(())
    }

    /// 解除逻辑段的所有虚拟页的映射
    #[allow(unused)]
    fn unmap_all(&mut self, page_table: &mut PageTable) -> Result<()> {
        for vpn in self.vpn_range {
            // 逐帧解映射
            if let Err(e) = self.unmap_single(page_table, vpn) {
                return Err(e);
            }
        }
        Ok(())
    }

    /// 从数据源复制数据到逻辑段
    pub fn copy_from_slice(&mut self, page_table: &PageTable, data: &[u8]) -> Result<()> {
        // 只有帧映射的逻辑段才能复制数据
        if self.seg_type != SegType::Framed {
            return Err(new_error!(
                ErrorKind::Mem(error::mem::ErrorKind::InvalidSegmentType),
                MsgType::StaticStr("only support copy data to framed segment")
            ));
        }
        // 判断数据长度是否合法
        let data_len = data.len();
        let seg_len = self.vpn_range.get_end().0 - self.vpn_range.get_start().0;
        if data_len > seg_len * config::PAGE_SIZE {
            return Err(new_error!(
                ErrorKind::Mem(error::mem::ErrorKind::DataTooLong),
                MsgType::String(format!(
                    "data is too long: data_len={}, seg_len={}",
                    data_len, seg_len
                ))
            ));
        }

        // 逐帧复制数据
        let mut data_ptr: usize = 0;
        for vpn in self.vpn_range {
            let data_slice = &data[data_ptr..min(data_len, data_ptr + config::PAGE_SIZE)];
            let dst =
                &mut page_table.translate(vpn)?.ppn().get_as_bytes_array()[..data_slice.len()];
            dst.copy_from_slice(data_slice);
            data_ptr += data_slice.len();
            if data_ptr >= data_len {
                break;
            }
        }

        Ok(())
    }

    /// 从已有逻辑段复刻新的逻辑段
    pub fn from_another(seg: &LogicalSegment) -> Self {
        Self {
            vpn_range: seg.vpn_range.clone(),
            data_frames: BTreeMap::new(),
            seg_type: seg.seg_type,
            seg_perm: seg.seg_perm,
        }
    }
}

/// 地址空间（类似“内存工作集”的概念）
pub struct MemorySet {
    /// 该地址空间对应的页表
    page_table: PageTable,
    /// 该地址空间包含的所有的逻辑段
    segments: Vec<LogicalSegment>,
}

impl MemorySet {
    pub fn new() -> Self {
        Self {
            page_table: PageTable::new().ok().unwrap(),
            segments: Vec::new(),
        }
    }

    /// 激活该地址空间
    ///
    /// 将该地址空间的页表设置为satp寄存器的值，并刷新TLB
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma"); // 刷新TLB
        }
    }

    /// 将一个逻辑段加入地址空间（可进行数据拷贝）
    fn push(&mut self, mut seg: LogicalSegment, data: Option<&[u8]>) -> Result<()> {
        // 将逻辑段映射到页表上
        if let Err(e) = seg.map_all(&mut self.page_table) {
            return Err(e);
        }
        if let Some(data) = data {
            if let Err(e) = seg.copy_from_slice(&mut self.page_table, data) {
                return Err(e);
            }
        }
        self.segments.push(seg);
        Ok(())
    }

    /// 将一个逻辑段从地址空间中移出
    fn remove(&mut self, start_va: VirtAddr) -> Result<()> {
        for i in 0..self.segments.len() {
            let seg = &mut self.segments[i];
            if seg.vpn_range.get_start() == start_va.floor() {
                if let Err(e) = seg.unmap_all(&mut self.page_table) {
                    return Err(e);
                }
                self.segments.remove(i);
                break;
            }
        }
        Ok(())
    }

    /// 新建一个逻辑段，加入地址空间
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: SegPermission,
    ) -> Result<()> {
        self.push(
            LogicalSegment::new(start_va, end_va, SegType::Framed, permission),
            None,
        )
    }

    /// 删除一个逻辑段，从地址空间中解除映射
    pub fn remove_area(&mut self, start_va: VirtAddr) -> Result<()> {
        self.remove(start_va)
    }

    /// 清空地址空间
    pub fn recycle(&mut self) {
        // 释放所有逻辑段中的帧
        self.segments.clear();
    }

    /// 创建跳板页
    fn map_trampoline(&mut self) {
        extern "C" {
            fn strampoline();
        }
        if let Err(e) = self.page_table.map(
            VirtAddr::from(config::TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        ) {
            panic!("map_trampoline failed: {:?}", e);
        }
    }

    /// 为内核分配地址空间
    pub fn new_kernel() -> Self {
        // 内核段地址
        extern "C" {
            fn stext();
            fn etext();
            fn srodata();
            fn erodata();
            fn sdata();
            fn edata();
            fn sbss_with_stack();
            fn ebss();
            fn ekernel();
        }

        let mut mem_set = Self::new();

        mem_set.map_trampoline(); // 映射跳板代码段

        // 直接映射内核段

        // 代码段 [stext, etext) R-X-
        println!(".text [{:#x}, {:#x}) R-X-", stext as usize, etext as usize);
        if let Err(e) = mem_set.push(
            LogicalSegment::new(
                (stext as usize).into(),
                (etext as usize).into(),
                SegType::Identical,
                SegPermission::R | SegPermission::X,
            ),
            None,
        ) {
            panic!("map .text failed: {:?}", e);
        }

        // 只读数据段 [srodata, erodata) R---
        println!(
            ".rodata [{:#x}, {:#x}) R---",
            srodata as usize, erodata as usize
        );
        if let Err(e) = mem_set.push(
            LogicalSegment::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                SegType::Identical,
                SegPermission::R,
            ),
            None,
        ) {
            panic!("map .rodata failed: {:?}", e);
        }

        // 数据段 [sdata, edata) RW--
        println!(".data [{:#x}, {:#x}) RW--", sdata as usize, edata as usize);
        if let Err(e) = mem_set.push(
            LogicalSegment::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                SegType::Identical,
                SegPermission::R | SegPermission::W,
            ),
            None,
        ) {
            panic!("map .data failed: {:?}", e);
        }

        // 未初始化数据段 [sbss_with_stack, ebss) RW--
        println!(
            ".bss [{:#x}, {:#x}) RW--",
            sbss_with_stack as usize, ebss as usize
        );
        if let Err(e) = mem_set.push(
            LogicalSegment::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                SegType::Identical,
                SegPermission::R | SegPermission::W,
            ),
            None,
        ) {
            panic!("map .bss failed: {:?}", e);
        }

        // 物理内存直接映射 [ekernel, MEMORY_END) RW--
        println!(
            "PhyMem [{:#x}, {:#x}) RW--",
            ekernel as usize,
            config::MEMORY_END
        );
        if let Err(e) = mem_set.push(
            LogicalSegment::new(
                (ekernel as usize).into(),
                config::MEMORY_END.into(),
                SegType::Identical,
                SegPermission::R | SegPermission::W,
            ),
            None,
        ) {
            panic!("map physical memory failed: {:?}", e);
        }

        println!("kernel memory set initialized");

        mem_set
    }

    /// 从elf文件为用户程序分配地址空间
    ///
    /// 返回值为（MemorySet, 用户程序的栈指针, 用户程序的入口地址）
    pub fn new_app_from_elf(elf_data: &[u8]) -> Result<(Self, usize, usize)> {
        let mut mem_set = Self::new();

        mem_set.map_trampoline(); // 映射跳板代码段

        // 解析elf文件
        let elf = xmas_elf::ElfFile::new(elf_data).map_err(|_| {
            new_error!(
                ErrorKind::Process(process::ErrorKind::InvalidElf),
                MsgType::StaticStr("invalid elf file")
            )
        })?;
        let header = elf.header;
        // 检查elf文件是否合法
        let magic = header.pt1.magic;
        if magic != [0x7f, 0x45, 0x4c, 0x46] {
            return Err(new_error!(
                ErrorKind::Process(process::ErrorKind::InvalidElf),
                MsgType::StaticStr("bad elf!")
            ));
        }

        let ph_count = header.pt2.ph_count(); // program header count
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            // 遍历查找需要加载的程序头
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                // 起始和结束虚拟地址
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                // 标记段权限
                let mut seg_perm = SegPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    seg_perm |= SegPermission::R;
                }
                if ph_flags.is_write() {
                    seg_perm |= SegPermission::W;
                }
                if ph_flags.is_execute() {
                    seg_perm |= SegPermission::X;
                }

                // 申请段空间
                let seg = LogicalSegment::new(start_va, end_va, SegType::Framed, seg_perm);
                max_end_vpn = seg.vpn_range.get_end();
                if let Err(e) = mem_set.push(
                    seg,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                ) {
                    error!("map segment failed: {:?}", e);
                    return Err(e);
                }
            }
        }

        // 映射用户栈
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        user_stack_bottom += config::PAGE_SIZE; // 插入一个页作为Guard Page
        let user_stack_top = user_stack_bottom + config::USER_STACK_SIZE; // 设置栈顶
        if let Err(e) = mem_set.push(
            LogicalSegment::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                SegType::Framed,
                SegPermission::R | SegPermission::W | SegPermission::U,
            ),
            None,
        ) {
            error!("map user stack failed: {:?}", e);
            return Err(e);
        }

        // 映射中断上下文段（仅内核态可访问）
        if let Err(e) = mem_set.push(
            LogicalSegment::new(
                config::TRAP_CONTEXT.into(),
                config::TRAMPOLINE.into(),
                SegType::Framed,
                SegPermission::R | SegPermission::W,
            ),
            None,
        ) {
            error!("map for user trap context failed: {:?}", e);
            return Err(e);
        }

        Ok((
            mem_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        ))
    }

    /// 从已有地址空间复刻新的地址空间
    pub fn from_existed(mem_set: &MemorySet) -> Result<Self> {
        let mut new_mem_set = Self::new();
        new_mem_set.map_trampoline();
        for seg in mem_set.segments.iter() {
            let new_seg = LogicalSegment::from_another(seg);
            new_mem_set.push(new_seg, None)?;
            for vpn in seg.vpn_range {
                let src_ppn = mem_set.page_table.translate(vpn)?.ppn();
                let dst_ppn = new_mem_set.page_table.translate(vpn)?.ppn();
                dst_ppn
                    .get_as_bytes_array()
                    .copy_from_slice(src_ppn.get_as_bytes_array());
            }
        }
        Ok(new_mem_set)
    }

    /// 将虚拟页号转换为对应的物理页号
    pub fn translate(&self, vpn: VirtPageNum) -> Result<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    /// 获取页表的token
    pub fn token(&self) -> usize {
        self.page_table.token()
    }
}

/// 测试内核地址空间
pub fn remap_test() {
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
    }

    println!("running remap_test...");
    let kernel_space = super::KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_text.floor())
            .unwrap()
            .is_writable(),
        false
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_rodata.floor())
            .unwrap()
            .is_writable(),
        false,
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_data.floor())
            .unwrap()
            .is_executable(),
        false,
    );
    println!("remap_test passed!");
}
