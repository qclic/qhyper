#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(concat_idents)]

extern crate alloc;

#[cfg_attr(target_arch = "aarch64", path = "arch/aarch64/mod.rs")]
pub mod arch;
mod lang_items;
pub mod mem;
