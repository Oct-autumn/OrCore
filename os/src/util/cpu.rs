//! os/src/util/cpu.rs

use core::{arch::asm, usize};

use crate::sbi_call::{self};

/// 将核心ID保存到tp寄存器
pub fn set_cpu_id(hart_id: usize) {
    // 将处理器ID放入 tp(x4) 寄存器
    // tp 寄存器是一个专用寄存器，它不会被应用程序使用，且在之后的trap处理中也不会被修改
    unsafe {
        asm!("mv tp, {}", in(reg) hart_id);
    }
}

/// 获取当前处理器核心的ID
pub fn hart_id() -> usize {
    let mut ret;
    unsafe {
        asm!("mv {}, tp", out(reg) ret);
    }
    ret
}

/// 向指定id的处理器核心发送中断请求，唤起核心
#[allow(unused)]
pub fn send_ipi(hart_id: usize) {
    let hart_mask = 1 << hart_id;
    sbi_call::send_ipi(hart_mask);

    // sbi_call::send_ipi(1 << hart_id);
}

/// 向其它处理器核心发送中断请求，唤起所有核心
pub fn broadcast_ipi() {
    sbi_call::send_ipi(usize::MAX);
}
