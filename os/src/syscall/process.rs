//! os/src/syscall/process.rs <br>
//! process related syscall

use alloc::sync::Arc;
use log::*;

use crate::{
    error,
    loader::get_elf_data_by_name,
    mem::page_table::{translate_into_mut_i32, translated_str},
    task,
};

/// **功能：** 退出应用程序并将返回值告知批处理系统。 <br>
/// **参数：**  <br>
///         - `exit_code` 表示应用程序的返回值。<br>
/// **返回值：** 该系统调用不应该返回。<br>
/// **syscall ID：** 93
pub fn sys_exit(exit_code: i32) -> ! {
    info!(
        "Process pid:{} exited with code {}",
        task::current_process().unwrap().get_pid(),
        exit_code
    );
    task::exit_current_and_run_next(exit_code);
    unreachable!("Unreachable in sys_exit");
}

/// **功能：** 使当前应用程序放弃CPU使用权。 <br>
/// **参数：** 无 <br>
/// **返回值：** 0 <br>
/// **syscall ID：** 124
pub fn sys_yield() -> isize {
    task::suspend_current_and_run_next();
    0
}

/// **功能：** 创建一个新的进程。 <br>
/// **参数：** 无 <br>
/// **返回值：**<br>
///         - 成功：对于子进程会返回0，对于当前进程则会返回子进程的PID<br>
///         - 失败：-1（内存不足）<br>
/// **syscall ID：** 220
pub fn sys_fork() -> isize {
    let current_task = task::current_process().unwrap();
    let res = current_task.fork();
    match res {
        Ok(new_task) => {
            let new_pid = new_task.pid.0;
            // 修改返回值，对于子进程要返回0
            let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
            trap_cx.x[10] = 0; //x[10] is a0 reg
                               // add new task to scheduler
            task::add_task(new_task);
            return new_pid as isize;
        }
        Err(e) => match e.kind() {
            error::ErrorKind::Mem(e) => match e {
                error::mem::ErrorKind::OutOfMemory => {
                    return -1;
                }
                _ => {
                    panic!("Unexpected error: {:?}", e);
                }
            },
            _ => {
                panic!("Unexpected error: {:?}", e);
            }
        },
    }
}

/// **功能：** 等待一个子进程退出。 <br>
/// **参数：**  <br>
///         - `pid` 表示子进程的进程号；<br>
///         - `exit_code` 收集子进程返回值的地址。<br>
/// **返回值：**<br>
///         - 成功：0（子进程退出）<br>
///         - 失败：
///             -1（没有找到对应的子进程）<br>
///             -2（没有找到对应的僵尸子进程）<br>
///             -3（exit_code指针异常）<br>
/// **syscall ID：** 260
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let cp = task::current_process().unwrap();

    let cpi = cp.inner_exclusive_access();
    if cpi
        .children
        .iter()
        .find(|pcb| pid == -1 || pid as usize == pcb.get_pid())
        .is_none()
    {
        // 没有找到对应的子进程
        return -1;
    }

    // 查找是否有对应的僵尸子进程
    let target_pcb = cpi.children.iter().find(|pcb| {
        pcb.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == pcb.get_pid())
    });

    if let Some(target_cp) = target_pcb {
        // 找到对应的僵尸子进程
        let target_cpi = target_cp.inner_exclusive_access();
        assert_eq!(Arc::strong_count(target_cp), 1);
        let ec = target_cpi.exit_code;
        let ec_ret_res = translate_into_mut_i32(cpi.memory_set.token(), exit_code_ptr as usize);
        if ec_ret_res.is_err() {
            return -3;
        } else {
            let ec_ret = ec_ret_res.unwrap();
            *ec_ret = ec;
        }
        target_cp.get_pid() as isize
    } else {
        // 没有找到对应的僵尸子进程
        -2
    }
}

/// **功能：** 执行一个新的程序。 <br>
/// **参数：**  <br>
///         - `path` 表示新程序的路径。<br>
/// **返回值：**<br>
///         - 成功：不会返回<br>
///         - 失败：
///                 -1（无法获取path）<br>
///                 -2（elf数据异常）<br>
///                 -3（执行失败，大概率为OOM）<br>
/// **syscall ID：** 221
pub fn sys_exec(path: *const u8) -> isize {
    let token = task::current_process_token();
    let res = translated_str(token, path);
    if res.is_err() {
        error!("Failed to get path: {:?}", res.err());
        -1
    } else {
        let path = res.unwrap();
        let res = get_elf_data_by_name(&path);
        if res.is_err() {
            error!("Failed to get elf data: {:?}", res.err());
            return -2;
        }

        if let Err(e) = task::current_process().unwrap().exec(res.unwrap()) {
            error!("Failed to exec: {:?}", e);
            return -3;
        }
        0
    }
}

/// **功能：** 获取当前进程的PID。 <br>
/// **参数：** 无 <br>
/// **返回值：** 当前进程的PID <br>
/// **syscall ID：** 172
pub fn sys_getpid() -> isize {
    task::current_process().unwrap().pid.0 as isize
}
