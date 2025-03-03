use core::{
    arch::asm,
    ptr::slice_from_raw_parts,
    sync::atomic::{fence, Ordering},
};

use aarch64_cpu::{
    asm::barrier::{self, *},
    registers::*,
};
use buddy_system_allocator::Heap;
use memory_addr::{pa_range, VirtAddr};
use page_table_arm::*;
use page_table_generic::*;

use crate::{
    arch::boot::rust_main,
    debug::*,
    mem::{
        self, boot_stack, boot_stack_space, debug_space,
        space::{Space, SPACE_SET},
        va_offset,
    },
};

pub type TableRef<'a> = PageTableRef<'a, PageTableImpl>;

pub fn get_table() -> TableRef<'static> {
    PageTableRef::<PageTableImpl>::from_addr(
        (TTBR0_EL2.read(TTBR0_EL2::BADDR) << 1) as usize,
        PageTableImpl::level(),
    )
}

pub fn set_table(table: TableRef<'_>) {
    TTBR0_EL2.set(table.paddr() as _);
}

pub fn flush_table(addr: Option<VirtAddr>) {
    unsafe {
        if let Some(_addr) = addr {
            todo!()
        } else {
            asm!("tlbi alle2is; dsb sy; isb");
        }
    }
}

struct TableAlloc(Heap<32>);
impl Access for TableAlloc {
    fn va_offset(&self) -> usize {
        0
    }

    unsafe fn alloc(&mut self, layout: core::alloc::Layout) -> Option<core::ptr::NonNull<u8>> {
        self.0.alloc(layout).ok()
    }

    unsafe fn dealloc(&mut self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        self.0.dealloc(ptr, layout);
    }
}

pub fn init() -> ! {
    fence(Ordering::SeqCst);
    dbgln("init page table");

    mair_el2_apply();

    let mut access = TableAlloc(Heap::<32>::new());

    let stack_space = boot_stack_space();
    let stack_top = stack_space.virt().end.as_usize();

    // 临时用栈底储存页表项
    let tmp_pt = stack_space.phys.start.as_usize();

    dbg_mem("stack", boot_stack());
    dbg("tmp pt: ");
    dbg_hexln(tmp_pt as _);

    unsafe {
        let imag_spaces = mem::kernel_imag_spaces::<24>();
        for one in imag_spaces {
            SPACE_SET.push(one);
        }
        access.0.init(tmp_pt, 1024 * 1024);

        let mut table = PageTableRef::<PageTableImpl>::create_empty(&mut access).unwrap();

        if va_offset() > 0 {
            for space in SPACE_SET.iter() {
                map_space(&mut table, space, &mut access);
            }
        }

        map_space(&mut table, &stack_space, &mut access);
        SPACE_SET.push(stack_space);

        let debug_space = debug_space();
        map_space(&mut table, &debug_space, &mut access);
        SPACE_SET.push(debug_space);

        if let Some(fdt) = mem::get_fdt() {
            for memory in fdt.memory() {
                for region in memory.regions() {
                    map_direct(
                        &mut table,
                        &*slice_from_raw_parts(region.address, region.size),
                        AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
                        CacheSetting::Normal,
                        &mut access,
                        "memory",
                    );
                }
            }
        }

        // Enable page size = 4K, vaddr size = 48 bits, paddr size = 40 bits.
        TCR_EL2.write(
            TCR_EL2::PS::Bits_48
                + TCR_EL2::TG0::KiB_4
                + TCR_EL2::SH0::Inner
                + TCR_EL2::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
                + TCR_EL2::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
                + TCR_EL2::T0SZ.val(16),
        );

        TTBR0_EL2.set(table.paddr() as _);

        barrier::isb(barrier::SY);

        asm!("tlbi alle2is; dsb sy; isb");
        dbg("sp: ");
        dbg_hexln(stack_top as _);

        // Enable the MMU and turn on I-cache and D-cache
        SCTLR_EL2.modify(SCTLR_EL2::M::Enable + SCTLR_EL2::C::Cacheable + SCTLR_EL2::I::Cacheable);
        isb(SY);

        asm!(
            "MOV      sp,  {stack}",
            "LDR      x8,  ={entry}",
            "BLR      x8",
            "B       .",
            stack = in(reg) stack_top,
            entry = sym rust_main,
            options(nomem, nostack,noreturn)
        )
    }
}

fn map_space(table: &mut PageTableRef<PageTableImpl>, space: &Space, access: &mut TableAlloc) {
    let paddr = space.phys.start.as_usize();
    let vaddr = space.virt().start.as_ptr();
    let len = space.phys.size();

    dbg("map ");
    dbg_tb(space.name, 12);
    dbg(": [");
    dbg_hex(vaddr as usize as _);
    dbg(", ");
    dbg_hex(space.virt().end.as_usize() as _);
    dbg(") -> [");
    dbg_hex(paddr as _);
    dbg(", ");
    dbg_hex(space.phys.end.as_usize() as _);
    dbgln(")");

    unsafe {
        if let Err(_e) = table.map_region(
            MapConfig::new(vaddr, paddr, space.access, space.cache),
            len,
            true,
            access,
        ) {
            dbgln("map failed!");
        }
    }
}

fn map_direct(
    table: &mut PageTableRef<PageTableImpl>,
    range: &[u8],
    privilege_access: AccessSetting,
    cache_setting: CacheSetting,
    access: &mut TableAlloc,
    name: &'static str,
) {
    map_space(
        table,
        &Space {
            name,
            phys: pa_range!(range.as_ptr_range().start as usize..range.as_ptr_range().end as usize),
            offset: 0,
            access: privilege_access,
            cache: cache_setting,
        },
        access,
    );
}

fn mair_el2_apply() {
    let attr0 = MAIR_EL2::Attr0_Device::nonGathering_nonReordering_noEarlyWriteAck;
    // Normal memory
    let attr1 = MAIR_EL2::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc
        + MAIR_EL2::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc;
    let attr2 =
        MAIR_EL2::Attr2_Normal_Inner::NonCacheable + MAIR_EL2::Attr2_Normal_Outer::NonCacheable;

    MAIR_EL2.write(attr0 + attr1 + attr2);
}

#[derive(Clone, Copy)]
pub struct PageTableImpl;

impl PTEArch for PageTableImpl {
    fn page_size() -> usize {
        0x1000
    }

    fn level() -> usize {
        4
    }

    fn new_pte(config: PTEGeneric) -> usize {
        let mut pte = PTE::from_paddr(config.paddr);
        let mut flags = PTEFlags::empty();

        if config.is_valid {
            flags |= PTEFlags::VALID;
        }

        if !config.is_block {
            flags |= PTEFlags::NON_BLOCK;
        }

        pte.set_mair_idx(MAIRDefault::get_idx(match config.setting.cache_setting {
            CacheSetting::Normal => MAIRKind::Normal,
            CacheSetting::Device => MAIRKind::Device,
            CacheSetting::NonCache => MAIRKind::NonCache,
        }));

        let privilege = &config.setting.privilege_access;

        if !config.setting.is_global {
            flags |= PTEFlags::NG;
        }

        if privilege.readable() {
            flags |= PTEFlags::AF;
        }

        if !privilege.writable() {
            flags |= PTEFlags::AP_RO;
        }

        if !privilege.executable() {
            flags |= PTEFlags::UXN;
        }

        pte.set_flags(flags);

        let out: u64 = pte.into();

        out as _
    }

    fn read_pte(pte: usize) -> PTEGeneric {
        let pte = PTE::from(pte as u64);
        let paddr = pte.paddr();
        let flags = pte.get_flags();
        let is_valid = flags.contains(PTEFlags::VALID);
        let is_block = !flags.contains(PTEFlags::NON_BLOCK);
        let mut privilege_access = AccessSetting::empty();
        let mut user_access = AccessSetting::empty();
        let mut cache_setting = CacheSetting::Normal;
        let is_global = !flags.contains(PTEFlags::NG);

        if is_valid {
            let mair_idx = pte.get_mair_idx();

            cache_setting = match MAIRDefault::from_idx(mair_idx) {
                MAIRKind::Device => CacheSetting::Device,
                MAIRKind::Normal => CacheSetting::Normal,
                MAIRKind::NonCache => CacheSetting::NonCache,
            };

            if flags.contains(PTEFlags::AF) {
                privilege_access |= AccessSetting::Read;
            }

            if !flags.contains(PTEFlags::AP_RO) {
                privilege_access |= AccessSetting::Write;
            }

            if !flags.contains(PTEFlags::UXN) {
                privilege_access |= AccessSetting::Execute;
            }

            if flags.contains(PTEFlags::AP_EL0) {
                user_access |= AccessSetting::Read;

                if !flags.contains(PTEFlags::AP_RO) {
                    user_access |= AccessSetting::Write;
                }
            }
        }

        PTEGeneric {
            paddr,
            is_block,
            is_valid,
            setting: PTESetting {
                is_global,
                privilege_access,
                user_access,
                cache_setting,
            },
        }
    }
}
