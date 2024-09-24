//! os/src/syscall/file_sys.rs <br>
//! file and file-system related syscall

use log::*;

use crate::{mem::page_table, print, task};

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffer = page_table::translated_byte_buffer(task::current_user_token(), buf, len);
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
