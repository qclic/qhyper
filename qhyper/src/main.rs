#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(concat_idents)]

use core::hint::spin_loop;

use log::info;

extern crate alloc;

#[cfg_attr(target_arch = "aarch64", path = "arch/aarch64/mod.rs")]
pub mod arch;
pub mod debug;
mod lang_items;
#[macro_use]
pub mod logger;
pub mod consts;
pub mod io;
pub mod mem;
pub mod time;

pub fn vm_main() -> ! {
    logger::init();
    info!("VM start");

    loop {
        spin_loop();
    }
}
