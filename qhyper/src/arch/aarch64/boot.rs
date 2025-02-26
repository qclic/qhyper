use core::arch::{asm, naked_asm};

use aarch64_cpu::registers::*;

use crate::mem::VM_VA_OFFSET;

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
            "LDR      x20,  ={va}",
            "STR      x18, [x20]",
            "MOV      x19, x0",        // x19 = dtb_addr
            "LDR      x1,  =_stack_top",
            "SUB      x1,  x1, x18", // X1 == STACK_TOP
            "MOV      sp,  x1",
            // clear icache
            "ic  iallu",
            "BL       {switch_to_el2}",

            va = sym VM_VA_OFFSET,
            this_func = sym primary_entry,
            switch_to_el2 = sym switch_to_el2,
        )
    }
}

fn switch_to_el2() {
    SPSel.write(SPSel::SP::ELx);
    SP_EL0.set(0);
    let current_el = CurrentEL.read(CurrentEL::EL);
    if current_el == 3 {
        // Set EL2 to 64bit and enable the HVC instruction.
        SCR_EL3.write(
            SCR_EL3::NS::NonSecure + SCR_EL3::HCE::HvcEnabled + SCR_EL3::RW::NextELIsAarch64,
        );
        // Set the return address and exception level.
        SPSR_EL3.write(
            SPSR_EL3::M::EL1h
                + SPSR_EL3::D::Masked
                + SPSR_EL3::A::Masked
                + SPSR_EL3::I::Masked
                + SPSR_EL3::F::Masked,
        );
        unsafe {
            asm!(
                "
            adr  x2,      {f}
            msr  elr_el3, x2
            eret
            ",
            f = sym primary_entry,
            );
        }
    }
}

fn init_mmu() {
    let a = VM_VA_OFFSET;
    let b = 1;
    let c = a + b;
}
