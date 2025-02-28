use core::{
    arch::naked_asm,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut, NonNull},
};

use aarch64_cpu::{asm::barrier, registers::*};
use fdt_parser::Fdt;

use crate::{
    arch::{cache, mmu},
    debug::{self, dbg, dbg_hexln, dbgln},
    mem::{self, boot_stack},
    vm_main,
};

const FLAG_LE: usize = 0b0;
const FLAG_PAGE_SIZE_4K: usize = 0b10;
const FLAG_ANY_MEM: usize = 0b1000;

#[naked]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.head")]
/// The entry point of the kernel.
pub unsafe extern "C" fn _start() -> ! {
    unsafe {
        naked_asm!(
            // code0/code1
            "nop",
            "bl {entry}",
            // text_offset
            ".quad 0",
            // image_size
            ".quad _kernel_size",
            // flags
            ".quad {flags}",
            // Reserved fields
            ".quad 0",
            ".quad 0",
            ".quad 0",
            // magic - yes 0x644d5241 is the same as ASCII string "ARM\x64"
            ".ascii \"ARM\\x64\"",
            // Another reserved field at the end of the header
            ".byte 0, 0, 0, 0",
            flags = const FLAG_LE | FLAG_PAGE_SIZE_4K | FLAG_ANY_MEM,
            entry = sym primary_entry,
        )
    }
}

#[naked]
#[unsafe(link_section = ".text.boot")]
/// The entry point of the kernel.
unsafe extern "C" fn primary_entry() -> ! {
    unsafe {
        naked_asm!(
            "ADR      x11, .",
            "LDR      x10, ={this_func}",
            "SUB      x18, x10, x11", // x18 = va_offset
            "MOV      x19, x0",        // x19 = dtb_addr
            // disable cache and MMU
            "mrs x1, sctlr_el2",
            "bic x1, x1, #0xf",
            "msr sctlr_el2, x1",
            // setup stack
            "LDR      x1,  =_stack_top",
            "SUB      x1,  x1, x18", // X1 == STACK_TOP
            "MOV      sp,  x1",
            // cache_invalidate(0): clear dl1$
            "mov x0, #0",
            "bl  {cache_invalidate}",
            "mov x0, #2",
            "bl  {cache_invalidate}",
            // clear icache
            "ic  iallu",
            "BL       {clean_bss}",
            "MOV      x0,  x18",
            "BL       {set_va}",
            "BL       {switch_to_el2}",
            "BL       {enable_fp}",
            "MOV      x0,  x19",
            "BL       {init_debug}",
            "BL       {setup_el2}",
            "BL       {mmu_init}",
            set_va = sym set_va,
            this_func = sym primary_entry,
            switch_to_el2 = sym switch_to_el2,
            clean_bss = sym mem::clean_bss,
            mmu_init = sym mmu::init,
            enable_fp = sym enable_fp,
            init_debug = sym init_debug,
            setup_el2 = sym setup_el2,
            cache_invalidate = sym cache::cache_invalidate,
        )
    }
}

fn set_va(va: usize) {
    unsafe {
        mem::set_va(va);
    }
}

fn save_fdt<'a>(ptr: *mut u8) -> Option<Fdt<'a>> {
    let stack_top = boot_stack().as_ptr_range().end;
    let fdt = fdt_parser::Fdt::from_ptr(NonNull::new(ptr)?).ok()?;
    let len = fdt.total_size();

    unsafe {
        let dst = &mut *slice_from_raw_parts_mut(stack_top as usize as _, len);
        let src = &*slice_from_raw_parts(ptr, len);
        dst.copy_from_slice(src);

        mem::set_fdt(ptr, len);
    }

    mem::get_fdt()
}

fn switch_to_el2() {
    SPSel.write(SPSel::SP::ELx);
    let current_el = CurrentEL.read(CurrentEL::EL);
    if current_el == 3 {
        SCR_EL3.write(
            SCR_EL3::NS::NonSecure + SCR_EL3::HCE::HvcEnabled + SCR_EL3::RW::NextELIsAarch64,
        );
        SPSR_EL3.write(
            SPSR_EL3::M::EL2h
                + SPSR_EL3::D::Masked
                + SPSR_EL3::A::Masked
                + SPSR_EL3::I::Masked
                + SPSR_EL3::F::Masked,
        );
        ELR_EL3.set(LR.get());
        aarch64_cpu::asm::eret();
    }

    // Set EL1 to 64bit.
    // Enable `IMO` and `FMO` to make sure that:
    // * Physical IRQ interrupts are taken to EL2;
    // * Virtual IRQ interrupts are enabled;
    // * Physical FIQ interrupts are taken to EL2;
    // * Virtual FIQ interrupts are enabled.
    HCR_EL2.modify(
        HCR_EL2::VM::Enable
            + HCR_EL2::RW::EL1IsAarch64
            + HCR_EL2::IMO::EnableVirtualIRQ // Physical IRQ Routing.
            + HCR_EL2::FMO::EnableVirtualFIQ // Physical FIQ Routing.
            + HCR_EL2::TSC::EnableTrapEl1SmcToEl2,
    );
}
fn enable_fp() {
    CPACR_EL1.write(CPACR_EL1::FPEN::TrapNothing);
    barrier::isb(barrier::SY);
}
pub fn rust_main() -> ! {
    dbgln("mmu enabled");

    vm_main()
}

fn init_debug(fdt: *mut u8) -> Option<()> {
    let fdt = save_fdt(fdt)?;
    debug::init_by_fdt(fdt);
    dbg("VA_OFFSET: ");
    dbg_hexln(mem::va_offset() as _);

    if CurrentEL.read(CurrentEL::EL) != 2 {
        debug::dbgln("Not in EL2!");
        panic!("");
    }
    Some(())
}

fn setup_el2() {
    // Set EL1 to 64bit.
    // Enable `IMO` and `FMO` to make sure that:
    // * Physical IRQ interrupts are taken to EL2;
    // * Virtual IRQ interrupts are enabled;
    // * Physical FIQ interrupts are taken to EL2;
    // * Virtual FIQ interrupts are enabled.
    HCR_EL2.modify(
        HCR_EL2::VM::Enable
            + HCR_EL2::RW::EL1IsAarch64
            + HCR_EL2::IMO::EnableVirtualIRQ // Physical IRQ Routing.
            + HCR_EL2::FMO::EnableVirtualFIQ // Physical FIQ Routing.
            + HCR_EL2::TSC::EnableTrapEl1SmcToEl2,
    );
}
