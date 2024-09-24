/// Memory related errors
#[derive(Debug)]
pub enum ErrorKind {
    OutOfMemory,
    MappedPage,
    UnmappedPage,
    InvalidSegmentType,
    DataTooLong
}
