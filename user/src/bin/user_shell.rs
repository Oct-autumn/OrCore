#![no_std]
#![no_main]

use alloc::string::String;
use user_lib::{console, exec, fork, waitpid};

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8; // 换行
const CR: u8 = 0x0du8; // 回车
const DL: u8 = 0x7fu8; // 删除
const BS: u8 = 0x08u8; // 退格
const UP: u8 = 0x1bu8; // 上
const DOWN: u8 = 0x1au8; // 下
const LEFT: u8 = 0x1bu8; // 左
const RIGHT: u8 = 0x1cu8; // 右

#[no_mangle]
pub fn main() -> i32 {
    println!("< Welcome to Oct rOS >");
    println!("build: {}", env!("CARGO_PKG_VERSION"));
    let mut line: String = String::new(); // 命令行
    let mut line_ptr = 0; // 光标位置
    print!(">> ");
    loop {
        let c: u8 = console::getchar();
        match c {
            LF | CR => {
                println!(""); // 换行
                if !line.is_empty() {
                    // 执行命令
                    exec_cmd(line.clone());
                    line.clear();
                    line_ptr = 0;
                }
                print!(">> ");
            }
            BS => {
                // 退格
                if line_ptr > 0 {
                    line_ptr -= 1;
                    line.remove(line_ptr);
                    print!("{} {}", BS as char, BS as char);
                }
            }
            DL => {
                // 删除
                if line_ptr < line.len() {
                    line.remove(line_ptr);
                    print!("{}{}", BS as char, &line.as_str()[line_ptr..]);
                }
            }
            // TODO: 拦截上下移动，实现历史命令
            // TODO: 实现光标左右移动
            _ => {
                // 其他字符
                print!("{}", c as char);
                if line_ptr == line.len() {
                    line.push(c as char);
                } else {
                    line.insert(line_ptr, c as char);
                }
                line_ptr += 1;
            }
        }
    }
}

fn exec_cmd(mut cmd: String) -> isize {
    // 执行程序
    cmd.push('\0');
    let pid = fork();
    if pid == 0 {
        // 子进程：运行应用程序
        let ret = exec(cmd.as_str());
        if ret != 0 {
            println!("An error occurred while executing! {}", ret);
            return ret;
        }
        unreachable!();
    } else {
        // 父进程：等待应用程序结束
        let mut exit_code: i32 = 0;
        let exit_pid = waitpid(pid as usize, &mut exit_code);
        assert_eq!(exit_pid, pid);
        println!("[Shell] pid:{} exited with code {}", pid, exit_code);
    }
    0
}
