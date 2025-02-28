use alloc::string::String;
use core::fmt::{Debug, Formatter, Result};

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code, clippy::upper_case_acronyms)]
/// POSIX errno
pub enum HvErrorNum {
    EPERM = 1,    // Operation not permitted.
    ENOENT = 2,   // No such file or directory.
    EIO = 5,      // I/O error.
    E2BIG = 7,    // Argument list too long.
    ENOMEM = 12,  // Not enough space.
    EFAULT = 14,  // Bad address.
    EBUSY = 16,   // Device or resource busy.
    EEXIST = 17,  // File exists.
    ENODEV = 19,  // No such device.
    EINVAL = 22,  // Invalid argument.
    ERANGE = 34,  // Result too large.
    ENOSYS = 38,  // Function not implemented.
}

pub struct HvError {
    pub num: HvErrorNum,
    pub loc_line: u32,
    pub loc_col: u32,
    pub loc_file: &'static str,
    pub msg: Option<String>,
}