use super::{
    syscall, SYSCALL_EXEC, SYSCALL_EXIT, SYSCALL_FORK, SYSCALL_GETPID, SYSCALL_WAITPID,
    SYSCALL_YIELD,
};

/// **功能：** 退出应用程序并将返回值告知批处理系统。 <br>
/// **参数：**  <br>
///         - `exit_code` 表示应用程序的返回值。<br>
/// **返回值：** 无意义<br>
/// **syscall ID：** 93
fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

/// **功能：** 退出应用程序并将返回值告知批处理系统。 <br>
/// **参数：**  <br>
///         - `exit_code` 表示应用程序的返回值。<br>
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

/// **功能：** 使当前应用程序放弃CPU使用权。 <br>
/// **参数：** 无 <br>
/// **返回值：** 0 <br>
/// **syscall ID：** 124
fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

/// **功能：** 使当前应用程序放弃CPU使用权。 <br>
pub fn yield_next() -> isize {
    sys_yield()
}

/// **功能：** 创建一个新的进程。 <br>
/// **参数：** 无 <br>
/// **返回值：**<br>
///         - 成功：对于子进程会返回0，对于当前进程则会返回子进程的PID<br>
///         - 失败：返回-1（内存不足）<br>
/// **syscall ID：** 220
fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

/// **功能：** 创建一个新的进程。 <br>
/// **参数：** 无 <br>
/// **返回值：**<br>
///         - 成功：对于子进程会返回0，对于当前进程则会返回子进程的PID<br>
///         - 失败：返回-1（内存不足）<br>
pub fn fork() -> isize {
    sys_fork()
}

/// **功能：** 执行一个新的程序。 <br>
/// **参数：**  <br>
///         - `path` 表示新程序的路径。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
/// **syscall ID：** 221
fn sys_exec(path: &str) -> isize {
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, 0, 0])
}

/// **功能：** 执行一个新的程序。 <br>
/// **参数：**  <br>
///         - `path` 表示新程序的路径。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
pub fn exec(path: &str) -> isize {
    sys_exec(path)
}

/// **功能：** 等待一个子进程退出 <br>
/// **参数：**  <br>
///         - `pid` 表示子进程的进程号；<br>
///         - `exit_code` 收集子进程返回值的地址。<br>
/// **返回值：** -2表示没有子进程退出，-1表示失败，其他值表示子进程的进程号。<br>
/// **syscall ID：** 260
fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}

/// **功能：** 等待任意子进程退出 <br>
/// **参数：**  <br>
///         - `exit_code` 收集子进程返回值的地址。<br>
/// **返回值：** -1表示失败，其他值表示子进程的进程号。<br>
pub fn wait(exit_code: *mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            // -2(没有子进程退出) => 继续等待
            -2 => {
                yield_next();
            }
            // -1(错误) 或者 子进程的pid
            exit_pid => return exit_pid,
        }
    }
}

/// **功能：** 等待一个子进程退出 <br>
/// **参数：**  <br>
///         - `pid` 表示子进程的进程号；<br>
///         - `exit_code` 收集子进程返回值的地址。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
pub fn waitpid(pid: usize, exit_code: *mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            // -2(没有子进程退出) => 继续等待
            -2 => {
                yield_next();
            }
            // -1(错误) 或者 子进程的pid
            exit_pid => return exit_pid,
        }
    }
}

/// **功能：** 获取当前进程的PID <br>
/// **参数：** 无 <br>
/// **返回值：** 当前进程的PID <br>
/// **syscall ID：** 172
fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

/// **功能：** 获取当前进程的PID <br>
/// **参数：** 无 <br>
/// **返回值：** 当前进程的PID <br>
pub fn getpid() -> isize {
    sys_getpid()
}
