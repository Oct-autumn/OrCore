use log::*;


mod file_sys;
mod process;
mod time;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => file_sys::sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_YIELD => process::sys_yield(),
        SYSCALL_GET_TIME => time::sys_get_time(args[0], args[1]),
        SYSCALL_EXIT => process::sys_exit(args[0] as i32),
        SYSCALL_MMAP => file_sys::sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => file_sys::sys_munmap(args[0], args[1]),
        _ => {
            error!("Unsupported syscall_id {}", syscall_id);
            -1
        }
    }
}
