mod fs;
mod process;
mod time;

use core::arch::asm;

#[allow(unused)]
pub use fs::{mmap, munmap, read, write};
#[allow(unused)]
pub use process::{exec, exit, fork, getpid, wait, waitpid, yield_next};
#[allow(unused)]
pub use time::{get_time_msec, get_time_usec};

const SYSCALL_WRITE: usize = 64;
const SYSCALL_READ: usize = 63;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;


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
