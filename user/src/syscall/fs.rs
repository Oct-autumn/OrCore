use bitflags::bitflags;
use super::{syscall, SYSCALL_CLOSE, SYSCALL_MMAP, SYSCALL_MUNMAP, SYSCALL_READ, SYSCALL_WRITE};

/// **功能：** 将内存中缓冲区中的数据写入文件。
///
/// **参数：**
///   - `fd` 表示待写入文件的文件描述符
///   - `buf` 表示内存中缓冲区的起始地址
///   - `len` 表示内存中缓冲区的长度
///
/// **返回值：**
///   - 成功：返回写入的长度；
///   - 失败：
///     - -1（不支持的文件描述符）
///     - -2（无效的缓冲区）
///
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

/// **功能：** 打开文件
///
/// **参数：**
///         - `path` 表示文件路径
///         - `flags` 表示打开文件的标志
///
/// **返回值：**
///   - 成功：返回文件描述符
/// **syscall ID：** 56
pub fn sys_open(path: &str, flags: u32) -> isize {
    // TODO: 实现打开文件
    0
}

bitflags! {
    pub struct OpenFlags: u32 {
        /// 只读
        const RD  = 0b0000000000;
        /// 只写
        const WR  = 0b0000000001;
        /// 读写
        const RDWR = 0b0000000010;
        /// 不存在时创建，存在时清空
        const CR = 0b0100000000;
        /// 存在时清空
        const TR = 0b1000000000;
    }
}

/// **功能：** 打开文件
///
/// **参数：**
///         - `path` 表示文件路径
///         - `flags` 表示打开文件的标志
///
/// **返回值：**
///   - 成功：返回文件描述符
pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits())
}

/// **功能：** 关闭文件
///
/// **参数：**
///     - `fd` 表示文件描述符
///
/// **返回值：**
///     - 成功：返回0
///
/// **syscall ID：** 57
pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

/// **功能：** 关闭文件
///
/// **参数：**
///     - `fd` 表示文件描述符
///
/// **返回值：**
///     - 成功：返回0
pub fn close(fd: usize) -> isize { sys_close(fd) }

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
