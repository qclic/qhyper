use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};

use buddy_system_allocator::LockedHeap;

pub mod mmu;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

#[link_section = ".data"]
pub static VM_VA_OFFSET: usize = 0x111;

extern "C" {
    fn _stext();
    fn _etext();
    fn _srodata();
    fn _erodata();
    fn _sdata();
    fn _edata();
    fn _sbss();
    fn _ebss();
    fn _stack_bottom();
    fn _stack_top();
}

pub fn clean_bss() {
    let start = _sbss as *const u8 as usize;
    let end = _ebss as *const u8 as usize;
    let bss = unsafe { &mut *slice_from_raw_parts_mut(start as *mut u8, end - start) };
    bss.fill(0);
}

macro_rules! fn_ld_range {
    ($name:ident) => {
        pub fn $name() -> &'static [u8] {
            let start = concat_idents!(_s, $name) as *const u8 as usize;
            let end = concat_idents!(_e, $name) as *const u8 as usize;
            unsafe { &*slice_from_raw_parts(start as *mut u8, end - start) }
        }
    };
}

fn_ld_range!(text);
fn_ld_range!(rodata);
fn_ld_range!(data);
fn_ld_range!(bss);

pub fn stack() -> &'static [u8] {
    let mut start = _stack_bottom as *const u8;
    let end = _stack_top as *const u8 as usize;
    let len = end - start as usize;
    start = unsafe { start.sub(VM_VA_OFFSET) };
    unsafe { &*slice_from_raw_parts(start, len) }
}
