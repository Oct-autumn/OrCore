//! os/src/syscall/file_sys.rs <br>
//! file and file-system related syscall

use log::*;

use crate::{
    config,
    mem::{
        memory_set::SegPermission,
        page_table::{self},
    },
    print,
    sbi_call::console_getchar,
    task,
};

const FD_STDOUT: usize = 1;
const FD_STDIN: usize = 0;

/// **功能：** 将内存中缓冲区中的数据写入文件。 <br>
///
/// **参数：**  <br>
///         - `fd` 表示待写入文件的文件描述符；<br>
///         - `buf` 表示内存中缓冲区的起始地址；<br>
///         - `len` 表示内存中缓冲区的长度。<br>
///
/// **返回值：**<br>
///         - 成功：返回写入的长度；<br>
/// **syscall ID：** 64
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let res = page_table::translated_byte_buffer(task::current_process_token(), buf, len);
            if res.is_err() {
                error!("Failed to translate buffer: {:?}", res.err());
                return -2;
            }

            let buffer = res.unwrap();
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

/// **功能：** 从文件中读取数据到内存缓冲区。 <br>
/// **参数：**  <br>
///         - `fd` 表示待读取文件的文件描述符；<br>
///         - `buf` 表示内存中缓冲区的起始地址；<br>
/// **返回值：** 返回成功读取的长度。<br>
/// **syscall ID：** 63
pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    task::suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            let res = page_table::translated_byte_buffer(task::current_process_token(), buf, len);
            if res.is_err() {
                error!("Failed to translate buffer: {:?}", res.err());
                return -2;
            }
            let mut buffers = res.unwrap();
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}

/// **功能：** 为当前应用程序申请内存空间。 <br>
/// **参数：**  <br>
///         - `s_va` 表示申请内存的起始地址；<br>
///         - `len` 表示申请内存的长度；<br>
///         - `prot` 表示内存的权限。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
/// **syscall ID：** 222
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

    0
    // TODO: task::alloc_for_current(s_va, len, perm)
}

/// **功能：** 释放当前应用程序的内存空间。 <br>
/// **参数：**  <br>
///         - `s_va` 表示释放内存的起始地址；<br>
///         - `len` 表示释放内存的长度。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
/// **syscall ID：** 215
pub fn sys_munmap(s_va: usize, len: usize) -> isize {
    assert!(s_va & config::PAGE_OFFSET_MASK == 0); // 起始地址应当是页对齐的

    // len向上取整
    let len = (len + config::PAGE_SIZE - 1) & !config::PAGE_OFFSET_MASK;

    0
    // TODO: task::dealloc_for_current(s_va, len)
}
