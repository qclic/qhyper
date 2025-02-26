use core::ptr::slice_from_raw_parts_mut;

use buddy_system_allocator::LockedHeap;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

#[link_section = ".data"]
pub static VM_VA_OFFSET: usize = 0x111;

extern "C" {
    fn _sbss();
    fn _ebss();
}

pub(crate) unsafe fn clean_bss() {
    let start = _sbss as *const u8 as usize;
    let end = _ebss as *const u8 as usize;
    let bss = unsafe { &mut *slice_from_raw_parts_mut(start as *mut u8, end - start) };
    bss.fill(0);
}
