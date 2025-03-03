mod boot;
mod cache;
mod cpu;
pub mod mmu;
mod trap;

use core::hint::spin_loop;

use aarch64_cpu::registers::*;
use log::error;
pub use trap::install_trap_vector;

use crate::percpu::CPUHardId;

pub fn shutdown() -> ! {
    error!("shutdown unimplemented");
    loop {
        spin_loop();
    }
}

pub fn is_mmu_enabled() -> bool {
    SCTLR_EL2.matches_any(&[SCTLR_EL2::M::Enable])
}

pub fn cpu_id() -> CPUHardId {
    (MPIDR_EL1.get() as usize & 0xff00ffffff).into()
}
