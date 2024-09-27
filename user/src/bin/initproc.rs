#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_next};

// OSKernel运行时会将本程序作为第一个启动
// 进入main函数后，根父进程会fork一个子进程，子进程会执行shell
#[no_mangle]
fn main() -> i32 {
    let pid = fork();
    if pid == 0 {
        // 启动shell
        exec("user_shell\0"); // &str末尾不会主动插入/0，所以这里手动插入
    } else if pid > 0 {
        // 父进程等待子进程退出
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_next();
                continue;
            } else {
                println!("[Initproc] Process {} exited with code {}", pid, exit_code);
            }
        }
    } else {
        println!("[Initproc] Error occured when forking subprocess! Exit.")
    }
    0
}
