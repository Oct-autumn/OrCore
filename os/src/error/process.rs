/// Process related errors
#[derive(Debug)]
pub enum ErrorKind {
    /// 无效的ELF文件
    InvalidElf,
    /// 无效程序路径
    InvalidPath,
}
