//! Test for fork / waitpid / getpid

#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::alloc::vec::Vec;

use user_lib::{fork, getpid, waitpid};

#[no_mangle]
pub fn main() -> i32 {
    let mut subprocess_pid = Vec::new();

    for i in 1..5 {
        let pid = fork();
        if pid == 0 {
            println!("pid {}: I am child", getpid());
            return i;
        } else {
            subprocess_pid.push(pid);
            println!(
                "pid {}: I am parent, fork a child with pid {}",
                getpid(),
                pid
            );
        }
    }

    for pid in subprocess_pid.iter() {
        println!("pid {}: wait for child {}", getpid(), pid);
        let mut exit_code: i32 = 0;
        waitpid((*pid) as usize, &mut exit_code);
        println!(
            "pid {}: child {} exited with code {}",
            getpid(),
            pid,
            exit_code
        );
    }

    println!("All subprocesses exited!");
    0
}
