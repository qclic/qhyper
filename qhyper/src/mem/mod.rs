use core::{
    ptr::{null_mut, slice_from_raw_parts, slice_from_raw_parts_mut, NonNull},
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use arrayvec::ArrayVec;
use buddy_system_allocator::LockedHeap;
use fdt_parser::Fdt;
use log::info;
use memory_addr::{pa_range, MemoryAddr, PhysAddrRange};
use page_table_generic::{AccessSetting, CacheSetting};
use space::{Space, SPACE_SET};

use crate::{
    arch::{self, is_mmu_enabled},
    consts::KERNEL_STACK_SIZE,
    percpu,
};

pub mod addr;
pub mod mmu;
pub mod once;
pub mod space;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

static VM_VA_OFFSET: AtomicUsize = AtomicUsize::new(111);
static FDT_ADDR: AtomicPtr<u8> = AtomicPtr::new(null_mut());
static FDT_LEN: AtomicUsize = AtomicUsize::new(0);

const KERNEL_STACK_BOTTOM: usize = 0xE10000000000;
/// The size of a page.
pub const PAGE_SIZE_4K: usize = 0x1000;
pub const PAGE_SIZE_2M: usize = 0x20_0000;
pub const PAGE_SIZE_1G: usize = 0x4000_0000;

pub fn init() {
    let memory_used_end = SPACE_SET
        .iter()
        .max_by(|&x, &y| x.phys.end.cmp(&y.phys.end))
        .map(|one| one.phys)
        .expect("no space")
        .end
        .as_usize();
    let mut inited = false;

    //TODO 非设备树平台
    let fdt = get_fdt().unwrap();
    for memory in fdt.memory() {
        for region in memory.regions() {
            let mut start = region.address as usize;
            let end = start + region.size;

            if start < memory_used_end && end > memory_used_end {
                start = memory_used_end.align_up_4k();
            }

            let size = end - start;
            info!(
                "Add memory region [{:#x} - {:#x}), size: {:#x}",
                start,
                start + size,
                size
            );
            unsafe {
                if inited {
                    HEAP_ALLOCATOR.lock().add_to_heap(start, size);
                } else {
                    HEAP_ALLOCATOR.lock().init(start, size);
                    inited = true;
                }
            }
        }
    }
    percpu::init();
    mmu::init();
}

pub(crate) unsafe fn save_fdt<'a>(ptr: *mut u8) -> Option<Fdt<'a>> {
    let stack_top = boot_stack().as_ptr_range().end;
    let fdt = fdt_parser::Fdt::from_ptr(NonNull::new(ptr)?).ok()?;
    let len = fdt.total_size();

    unsafe {
        let dst = &mut *slice_from_raw_parts_mut(stack_top as usize as _, len);
        let src = &*slice_from_raw_parts(ptr, len);
        dst.copy_from_slice(src);

        set_fdt(dst.as_mut_ptr(), len.align_up_4k());
    }

    get_fdt()
}
fn set_fdt(ptr: *mut u8, len: usize) {
    FDT_ADDR.store(ptr, Ordering::SeqCst);
    FDT_LEN.store(len, Ordering::SeqCst);
}

pub fn fdt_data() -> &'static [u8] {
    unsafe {
        if FDT_LEN.load(Ordering::SeqCst) == 0 {
            return &[];
        }
        &*slice_from_raw_parts(
            FDT_ADDR.load(Ordering::SeqCst),
            FDT_LEN.load(Ordering::SeqCst),
        )
    }
}

pub(crate) fn get_fdt() -> Option<Fdt<'static>> {
    if FDT_LEN.load(Ordering::SeqCst) == 0 {
        return None;
    }
    Fdt::from_ptr(NonNull::new(FDT_ADDR.load(Ordering::SeqCst))?).ok()
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

    if !fdt_data().is_empty() {
        spaces.push(Space {
            name: "fdt",
            phys: slice_to_phys_range(fdt_data(), 0),
            offset: 0,
            access: AccessSetting::Read,
            cache: CacheSetting::Normal,
        });
    }
    spaces
}

pub(crate) unsafe fn set_va(va_offset: usize) {
    VM_VA_OFFSET.store(va_offset, Ordering::SeqCst);
}

pub fn va_offset() -> usize {
    VM_VA_OFFSET.load(Ordering::Relaxed)
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

pub fn stack0() -> PhysAddrRange {
    let offset = if is_mmu_enabled() { va_offset() } else { 0 };
    slice_to_phys_range(boot_stack(), offset)
}
