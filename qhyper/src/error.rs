use alloc::string::String;
use core::fmt::{Debug, Formatter, Result};

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code, clippy::upper_case_acronyms)]
/// POSIX errno
pub enum HvErrorNum {
    EPERM = 1,
    ENOENT = 2,
    EIO = 5,
    E2BIG = 7,
    ENOMEM = 12,
    EFAULT = 14,
    EBUSY = 16,
    EEXIST = 17,
    ENODEV = 19,
    EINVAL = 22,
    ERANGE = 34,
    ENOSYS = 38,
}

pub struct HvError {
    pub num: HvErrorNum,
    pub loc_line: u32,
    pub loc_col: u32,
    pub loc_file: &'static str,
    pub msg: Option<String>,
}

impl HvErrorNum {
    pub fn as_str(&self) -> &'static str {
        use HvErrorNum::*;
        match *self {
            EPERM => "Operation not permitted",
            ENOENT => "No such file or directory",
            EIO => "I/O error",
            E2BIG => "Argument list too long",
            ENOMEM => "Out of memory",
            EFAULT => "Bad address",
            EBUSY => "Device or resource busy",
            EEXIST => "File exists",
            ENODEV => "No such device",
            EINVAL => "Invalid argument",
            ERANGE => "Math result not representable",
            ENOSYS => "Function not implemented",
        }
    }
}

impl HvError {
    pub fn new(
        num: HvErrorNum,
        loc_file: &'static str,
        loc_line: u32,
        loc_col: u32,
        msg: Option<String>,
    ) -> Self {
        Self {
            num,
            loc_file,
            loc_line,
            loc_col,
            msg,
        }
    }

    pub fn code(&self) -> isize {
        -(self.num as usize as isize)
    }
}

impl Debug for HvError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "[{}:{}:{}] {}",
            self.loc_file,
            self.loc_line,
            self.loc_col,
            self.num.as_str()
        )?;
        if let Some(ref msg) = self.msg {
            write!(f, ": {}", msg)?;
        }
        Ok(())
    }
}
/// Generate a HvError according to error node and msg.
#[macro_export]
macro_rules! hv_err {
    ($num: ident) => {{
        use crate::error::{HvError, HvErrorNum::*};
        HvError::new($num, file!(), line!(), column!(), None)
    }};
    ($num: ident, $msg: expr) => {{
        use crate::error::{HvError, HvErrorNum::*};
        HvError::new($num, file!(), line!(), column!(), Some($msg.into()))
    }};
}
/// Generate a Err including a HvError struct
#[macro_export]
macro_rules! hv_result_err {
    ($num: ident) => {
        Err(hv_err!($num))
    };
    ($num: ident, $msg: expr) => {
        Err(hv_err!($num, $msg))
    };
}
