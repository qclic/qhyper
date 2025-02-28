use core::arch::global_asm;

use aarch64_cpu::registers::*;
use log::{error, trace};

use crate::{arch::shutdown, println};

use super::cpu::GeneralRegisters;

global_asm!(
    include_str!("./trap.S"),
    sym handle_exit
);

extern "C" {
    fn _trap_vector();
}

pub fn install_trap_vector() {
    // Set the trap vector.
    VBAR_EL2.set(_trap_vector as usize as _);
}

/*From hyp_vec->handle_vmexit x0:guest regs x1:exit_reason sp =stack_top-32*8*/
pub fn handle_exit(regs: &mut GeneralRegisters) -> ! {
    let mpidr = MPIDR_EL1.get();
    // let _cpu_id = mpidr_to_cpuid(mpidr);
    trace!("cpu exit, exit_reson:{:#x?}", regs.exit_reason);
    match regs.exit_reason as u64 {
        ExceptionType::EXIT_REASON_EL1_IRQ => irqchip_handle_irq_el1(),
        // ExceptionType::EXIT_REASON_EL1_ABORT => arch_handle_trap_el1(regs),
        ExceptionType::EXIT_REASON_EL2_ABORT => handle_trap_el2(regs),
        ExceptionType::EXIT_REASON_EL2_IRQ => irqchip_handle_irq_el2(),
        _ => arch_dump_exit(regs.exit_reason),
    }
    unsafe { vmreturn(regs as *const _ as usize) }
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod ExceptionType {
    pub const EXIT_REASON_EL2_ABORT: u64 = 0x0;
    pub const EXIT_REASON_EL2_IRQ: u64 = 0x1;
    pub const EXIT_REASON_EL1_ABORT: u64 = 0x2;
    pub const EXIT_REASON_EL1_IRQ: u64 = 0x3;
}

fn arch_dump_exit(reason: u64) {
    //TODO hypervisor coredump
    error!("Unsupported Exit:{:#x?}, elr={:#x?}", reason, ELR_EL2.get());
    shutdown();
}

fn irqchip_handle_irq_el1() {
    trace!("irq from el1");
    // gic_handle_irq();
}

fn irqchip_handle_irq_el2() {
    error!("irq not handle from el2");
    shutdown();
}

#[naked]
pub unsafe extern "C" fn vmreturn(_gu_regs: usize) -> ! {
    unsafe {
        core::arch::naked_asm!(
            "
        /* x0: guest registers */
        mov	sp, x0
        ldp	x1, x0, [sp], #16	/* x1 is the exit_reason */
        ldp	x1, x2, [sp], #16
        ldp	x3, x4, [sp], #16
        ldp	x5, x6, [sp], #16
        ldp	x7, x8, [sp], #16
        ldp	x9, x10, [sp], #16
        ldp	x11, x12, [sp], #16
        ldp	x13, x14, [sp], #16
        ldp	x15, x16, [sp], #16
        ldp	x17, x18, [sp], #16
        ldp	x19, x20, [sp], #16
        ldp	x21, x22, [sp], #16
        ldp	x23, x24, [sp], #16
        ldp	x25, x26, [sp], #16
        ldp	x27, x28, [sp], #16
        ldp	x29, x30, [sp], #16
        /*now el2 sp point to per cpu stack top*/
        eret                            //ret to el2_entry hvc #0 now,depend on ELR_EL2
        
    ",
        )
    }
}

fn handle_trap_el2(_regs: &mut GeneralRegisters) {
    let elr = ELR_EL2.get();
    let esr = ESR_EL2.get();
    let far = FAR_EL2.get();
    match ESR_EL2.read_as_enum(ESR_EL2::EC) {
        Some(ESR_EL2::EC::Value::HVC64) => {
            println!("EL2 Exception: HVC64 call, ELR_EL2: {:#x?}", ELR_EL2.get());
        }
        Some(ESR_EL2::EC::Value::SMC64) => {
            println!("EL2 Exception: SMC64 call, ELR_EL2: {:#x?}", ELR_EL2.get());
        }
        Some(ESR_EL2::EC::Value::DataAbortCurrentEL) => {
            println!(
                "EL2 Exception: Data Abort, ELR_EL2: {:#x?}, ESR_EL2: {:#x?}, FAR_EL2: {:#x?}",
                elr, esr, far
            );
            shutdown();
        }
        Some(ESR_EL2::EC::Value::InstrAbortCurrentEL) => {
            println!(
                "EL2 Exception: Instruction Abort, ELR_EL2: {:#x?}, FAR_EL2: {:#x?}",
                ELR_EL2.get(),
                FAR_EL2.get()
            );
        }
        _ => {
            println!(
                "Unhandled EL2 Exception: EC={:#x?}",
                ESR_EL2.read(ESR_EL2::EC)
            );
        }
    }
    shutdown();
}
