use core::fmt::{self, Write};

use crate::debug::dbg;

struct DebugWrite;

impl Write for DebugWrite {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        dbg(s);
        Ok(())
    }
}
pub fn print(args: fmt::Arguments<'_>) {
    let _ = DebugWrite {}.write_fmt(args);
}
