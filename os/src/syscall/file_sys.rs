//! os/src/syscall/file_sys.rs <br>
//! file and file-system related syscall

use log::*;

use crate::{
    config,
    mem::{
        memory_set::SegPermission,
        page_table::{self},
    },
    print, task,
};

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffer = page_table::translated_byte_buffer(task::current_app_token(), buf, len);
            for b in buffer {
                print!("{}", core::str::from_utf8(b).unwrap());
            }
            len as isize
        }
        _ => {
            error!("Unsupported fd {}", fd);
            -1
        }
    }
}

pub fn sys_mmap(s_va: usize, len: usize, prot: usize) -> isize {
    assert!(s_va & config::PAGE_OFFSET_MASK == 0); // 起始地址应当是页对齐的
    assert_eq!(prot & !0b111, 0); // 除最低三位，prot其它位必须为0
    assert_ne!(prot & 0b111, 0); // prot最低三位不能全为0

    // len向上取整
    let len = (len + config::PAGE_SIZE - 1) & !config::PAGE_OFFSET_MASK;
    // 逻辑段权限
    let mut perm = SegPermission::U;
    if prot & 0b001 != 0 {
        // 可读
        perm |= SegPermission::R;
    }
    if prot & 0b010 != 0 {
        // 可写
        perm |= SegPermission::W;
    }
    if prot & 0b100 != 0 {
        // 可执行
        perm |= SegPermission::X;
    }

    task::alloc_for_current(s_va, len, perm)
}

pub fn sys_munmap(s_va: usize, len: usize) -> isize {
    assert!(s_va & config::PAGE_OFFSET_MASK == 0); // 起始地址应当是页对齐的

    // len向上取整
    let len = (len + config::PAGE_SIZE - 1) & !config::PAGE_OFFSET_MASK;

    task::dealloc_for_current(s_va, len)
}
