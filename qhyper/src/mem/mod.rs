use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut, NonNull};

use arrayvec::ArrayVec;
use buddy_system_allocator::LockedHeap;
use fdt_parser::Fdt;
use memory_addr::{pa_range, PhysAddrRange};
use page_table_generic::{AccessSetting, CacheSetting};
use space::Space;

use crate::{arch, consts::KERNEL_STACK_SIZE};

pub mod addr;
pub mod mmu;
pub mod space;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

static mut VM_VA_OFFSET: usize = 0x111;
static mut FDT_ADDR: usize = 0;
static mut FDT_LEN: usize = 0;

const KERNEL_STACK_BOTTOM: usize = 0xE10000000000;

pub(crate) unsafe fn set_fdt(ptr: *mut u8, len: usize) {
    unsafe {
        FDT_ADDR = ptr as usize;
        FDT_LEN = len;
    }
}

pub fn fdt_data() -> &'static [u8] {
    unsafe {
        if FDT_LEN == 0 {
            return &[];
        }
        &*slice_from_raw_parts(FDT_ADDR as _, FDT_LEN)
    }
}

pub(crate) fn get_fdt() -> Option<Fdt<'static>> {
    unsafe {
        if FDT_LEN == 0 {
            return None;
        }
        Fdt::from_ptr(NonNull::new(FDT_ADDR as _)?).ok()
    }
}

fn slice_to_phys_range(data: &[u8], offset: usize) -> PhysAddrRange {
    let ptr_range = data.as_ptr_range();
    let start = ptr_range.start as usize - offset;
    let end = ptr_range.end as usize - offset;
    pa_range!(start..end)
}

pub fn kernel_imag_spaces<const CAP: usize>() -> ArrayVec<Space, CAP> {
    let is_virt = arch::is_mmu_enabled();
    let k_offset = if is_virt { va_offset() } else { 0 };

    let mut spaces = ArrayVec::<Space, CAP>::new();
    spaces.push(Space {
        name: ".text",
        phys: slice_to_phys_range(text(), k_offset),
        offset: va_offset(),
        access: AccessSetting::Read | AccessSetting::Execute,
        cache: CacheSetting::Normal,
    });
    spaces.push(Space {
        name: ".rodata",
        phys: slice_to_phys_range(rodata(), k_offset),
        offset: va_offset(),
        access: AccessSetting::Read | AccessSetting::Execute,
        cache: CacheSetting::Normal,
    });
    spaces.push(Space {
        name: ".data",
        phys: slice_to_phys_range(data(), k_offset),
        offset: va_offset(),
        access: AccessSetting::Read | AccessSetting::Execute | AccessSetting::Write,
        cache: CacheSetting::Normal,
    });
    spaces.push(Space {
        name: ".bss",
        phys: slice_to_phys_range(bss(), k_offset),
        offset: va_offset(),
        access: AccessSetting::Read | AccessSetting::Execute | AccessSetting::Write,
        cache: CacheSetting::Normal,
    });
    spaces
}

pub(crate) unsafe fn set_va(va_offset: usize) {
    unsafe {
        VM_VA_OFFSET = va_offset;
    }
}

pub fn va_offset() -> usize {
    unsafe { VM_VA_OFFSET }
}

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

pub fn boot_stack() -> &'static [u8] {
    let start = _stack_bottom as *const u8;
    let end = _stack_top as *const u8 as usize;
    let len = end - start as usize;
    unsafe { &*slice_from_raw_parts(start, len) }
}

pub fn boot_stack_space() -> Space {
    let offset = stack().as_ptr() as usize - boot_stack().as_ptr() as usize;
    Space {
        name: "stack0",
        phys: slice_to_phys_range(boot_stack(), 0),
        offset,
        access: AccessSetting::Read | AccessSetting::Execute | AccessSetting::Write,
        cache: CacheSetting::Normal,
    }
}

pub fn stack() -> &'static [u8] {
    let start = KERNEL_STACK_BOTTOM as *const u8;
    let len = KERNEL_STACK_SIZE;
    unsafe { &*slice_from_raw_parts(start, len) }
}
