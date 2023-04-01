use log::*;

mod file_sys;
mod process;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => {
            file_sys::sys_write(args[0], args[1] as *const u8, args[2])
        }
        SYSCALL_EXIT => {
            process::sys_exit(args[0] as i32);
        }
        _ => {
            error!("Unsupported syscall_id {}", syscall_id);
            -1
        }
    }
}