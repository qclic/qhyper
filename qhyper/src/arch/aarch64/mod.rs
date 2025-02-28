mod boot;
mod cache;
mod cpu;
mod mmu;
mod trap;

use aarch64_cpu::registers::*;
pub use trap::install_trap_vector;

pub fn shutdown() {
    //TODO
}

pub fn is_mmu_enabled() -> bool {
    SCTLR_EL2.matches_any(&[SCTLR_EL2::M::Enable])
}
