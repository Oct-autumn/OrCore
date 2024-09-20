//! user/src/sys_call.rs
//! The rust_sbi service impl

/* sys_call()          Func    call sbi service
 * console_putchar()   Func    put a char into console
 * shutdown()          Func    shutdown the machine gracefully
 */

use core::arch::asm;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
        "ecall",
        inlateout("x10") args[0] => ret,
        in("x11") args[1],
        in("x12") args[2],
        in("x17") id
        );
    }
    ret
}

/// **功能：** 将内存中缓冲区中的数据写入文件。 <br>
/// **参数：**  <br>
///         - `fd` 表示待写入文件的文件描述符；<br>
///         - `buf` 表示内存中缓冲区的起始地址；<br>
///         - `len` 表示内存中缓冲区的长度。<br>
/// **返回值：** 返回成功写入的长度。<br>
/// **syscall ID：** 64
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

/// **功能：** 退出应用程序并将返回值告知批处理系统。 <br>
/// **参数：**  <br>
///         - `exit_code` 表示应用程序的返回值。<br>
/// **返回值：** 该系统调用不应该返回。<br>
/// **syscall ID：** 93
pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

/// **功能：** 使当前应用程序放弃CPU使用权。 <br>
/// **参数：** 无 <br>
/// **返回值：** 0 <br>
/// **syscall ID：** 124
pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

#[repr(C)]
pub struct TimeVal {
    /// 系统启动后经过的秒数
    pub sec: usize,
    /// 系统启动后经过的微秒数
    pub usec: usize,
}

/// **功能：** 获取当前时间。 <br>
/// **参数：**  <br>
///         - `time` 表示存放时间的结构体指针；<br>
///         - `tz` 表示时区。（在本OS中不会使用）<br>
/// **返回值：** 0 <br>
/// **syscall ID：** 169
pub fn sys_get_time(time: *mut TimeVal, tz: usize) -> isize {
    syscall(SYSCALL_GET_TIME, [time as *const _ as usize, tz, 0])
}
