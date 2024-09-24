use alloc::string::String;

pub mod mem;

#[derive(Debug)]
#[allow(unused)]
pub enum ErrorKind {
    Mem(mem::ErrorKind),
}

#[derive(Debug)]
#[allow(unused)]
pub enum MsgType {
    StaticStr(&'static str),
    String(String),
}

#[derive(Debug)]
#[allow(unused)]
pub struct Error {
    kind: ErrorKind,
    msg: Option<MsgType>,
}

#[macro_export]
macro_rules! new_error {
    ($kind:expr, $msg:expr) => {
        Error::new_with_msg($kind, $msg)
    };
    ($kind:expr) => {
        Error::new($kind)
    };
}

#[allow(unused)]
impl Error {
    pub fn new_with_msg(kind: ErrorKind, msg: MsgType) -> Self {
        Self {
            kind,
            msg: Some(msg),
        }
    }
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, msg: None }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
