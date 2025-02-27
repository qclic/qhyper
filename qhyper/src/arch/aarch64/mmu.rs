use core::{
    arch::asm,
    sync::atomic::{fence, Ordering},
};

use aarch64_cpu::{
    asm::barrier::{self, *},
    registers::*,
};
use buddy_system_allocator::Heap;
use page_table_arm::*;
use page_table_generic::*;

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
            flags |= PTEFlags::PXN;
        }

        let user = &config.setting.user_access;

        if user.readable() {
            flags |= PTEFlags::AP_EL0;
        }

        if user.writable() {
            flags |= PTEFlags::AP_EL0;
            flags.remove(PTEFlags::AP_RO);
        }

        if !user.executable() {
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

            if !flags.contains(PTEFlags::PXN) {
                privilege_access |= AccessSetting::Execute;
            }

            if flags.contains(PTEFlags::AP_EL0) {
                user_access |= AccessSetting::Read;

                if !flags.contains(PTEFlags::AP_RO) {
                    user_access |= AccessSetting::Write;
                }
            }

            if !flags.contains(PTEFlags::UXN) {
                user_access |= AccessSetting::Execute;
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

use crate::{
    arch::boot::rust_main,
    mem::{self, stack, VM_VA_OFFSET},
};

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

pub fn init(dtb: *const u8) -> ! {
    MAIRDefault::mair_el2_apply();

    let mut access = TableAlloc(Heap::<32>::new());

    let stack_top = stack().as_ptr_range().end as usize;
    let stack_bottom = stack().as_ptr_range().start as usize;

    unsafe {
        access.0.init(stack_bottom, 1024 * 1024);

        let mut table = PageTableRef::<PageTableImpl>::create_empty(&mut access).unwrap();

        map_k_range(
            &mut table,
            mem::text(),
            AccessSetting::Read | AccessSetting::Execute,
            &mut access,
        );

        map_k_range(
            &mut table,
            mem::rodata(),
            AccessSetting::Read | AccessSetting::Execute,
            &mut access,
        );

        map_k_range(
            &mut table,
            mem::data(),
            AccessSetting::Read | AccessSetting::Execute | AccessSetting::Write,
            &mut access,
        );

        map_k_range(
            &mut table,
            mem::bss(),
            AccessSetting::Read | AccessSetting::Execute | AccessSetting::Write,
            &mut access,
        );

        table
            .map_region(
                MapConfig::new(
                    stack_bottom as _,
                    stack_bottom,
                    AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
                    CacheSetting::Normal,
                ),
                stack_top - stack_bottom,
                true,
                &mut access,
            )
            .unwrap();

        fence(Ordering::SeqCst);

        // Enable page size = 4K, vaddr size = 48 bits, paddr size = 40 bits.
        let tcr_flags0 = TCR_EL2::TG0::KiB_4
            + TCR_EL2::SH0::Inner
            + TCR_EL2::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL2::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL2::T0SZ.val(16);
        TCR_EL2.write(TCR_EL2::PS::Bits_40 + tcr_flags0);

        TTBR0_EL2.write(TTBR0_EL2::BADDR.val(table.paddr() as _));

        barrier::isb(barrier::SY);
        asm!("tlbi vmalle1");
        isb(SY);
        dsb(NSH);
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

fn map_k_range(
    table: &mut PageTableRef<PageTableImpl>,
    range: &[u8],
    privilege_access: AccessSetting,
    access: &mut TableAlloc,
) {
    let vaddr = range.as_ptr();
    let paddr = vaddr as usize - VM_VA_OFFSET;

    unsafe {
        table
            .map_region(
                MapConfig::new(vaddr, paddr, privilege_access, CacheSetting::Normal),
                range.len(),
                false,
                access,
            )
            .unwrap();
    }
}
