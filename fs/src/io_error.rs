
#[derive(Debug)]
pub enum ErrorKind {
}

#[derive(Debug)]
#[allow(unused)]
pub struct IoError {
    kind: ErrorKind,
    msg: Option<&'static str>,
}

impl IoError {
    pub fn new(kind: ErrorKind, msg: &'static str) -> Self {
        IoError { kind, msg: Some(msg) }
    }
}