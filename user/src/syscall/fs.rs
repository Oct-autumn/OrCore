use super::{syscall, SYSCALL_MMAP, SYSCALL_MUNMAP, SYSCALL_READ, SYSCALL_WRITE};

/// **功能：** 将内存中缓冲区中的数据写入文件。 <br>
/// **参数：**  <br>
///         - `fd` 表示待写入文件的文件描述符；<br>
///         - `buf` 表示内存中缓冲区的起始地址；<br>
///         - `len` 表示内存中缓冲区的长度。<br>
/// **返回值：** 返回成功写入的长度。<br>
/// **syscall ID：** 64
fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

/// **功能：** 将内存中缓冲区中的数据写入文件。 <br>
/// **参数：**  <br>
///         - `fd` 表示待写入文件的文件描述符；<br>
///         - `buf` 表示缓冲区；<br>
/// **返回值：** 返回成功写入的长度。<br>
pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

/// **功能：** 从文件中读取数据到内存缓冲区。 <br>
/// **参数：**  <br>
///         - `fd` 表示待读取文件的文件描述符；<br>
///         - `buf` 表示内存中缓冲区的起始地址；<br>
/// **返回值：** 返回成功读取的长度。<br>
/// **syscall ID：** 63
fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    syscall(SYSCALL_READ, [fd, buf.as_mut_ptr() as usize, buf.len()])
}

/// **功能：** 从文件中读取数据到内存缓冲区。 <br>
/// **参数：**  <br>
///         - `fd` 表示待读取文件的文件描述符；<br>
///         - `buf` 表示内存中缓冲区的起始地址；<br>
/// **返回值：** 返回成功读取的长度。<br>
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

/// **功能：** 为当前应用程序申请内存空间。 <br>
/// **参数：**  <br>
///         - `s_va` 表示申请内存的起始地址；<br>
///         - `len` 表示申请内存的长度；<br>
///         - `prot` 表示内存的权限。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
/// **syscall ID：** 222
fn sys_mmap(s_va: usize, len: usize, prot: usize) -> isize {
    syscall(SYSCALL_MMAP, [s_va, len, prot])
}

/// **功能：** 为当前应用程序申请内存空间。 <br>
/// **参数：**  <br>
///         - `s_va` 表示申请内存的起始地址；<br>
///         - `len` 表示申请内存的长度；<br>
///         - `prot` 表示内存的权限。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
pub fn mmap(s_va: usize, len: usize, prot: usize) -> isize {
    sys_mmap(s_va, len, prot)
}

/// **功能：** 释放当前应用程序的内存空间。 <br>
/// **参数：**  <br>
///         - `s_va` 表示释放内存的起始地址；<br>
///         - `len` 表示释放内存的长度。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
/// **syscall ID：** 215
fn sys_munmap(s_va: usize, len: usize) -> isize {
    syscall(SYSCALL_MUNMAP, [s_va, len, 0])
}

/// **功能：** 释放当前应用程序的内存空间。 <br>
/// **参数：**  <br>
///         - `s_va` 表示释放内存的起始地址；<br>
///         - `len` 表示释放内存的长度。<br>
/// **返回值：** 0表示成功，-1表示失败。<br>
pub fn munmap(s_va: usize, len: usize) -> isize {
    sys_munmap(s_va, len)
}
