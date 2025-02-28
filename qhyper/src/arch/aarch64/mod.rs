mod boot;
mod cache;
mod cpu;
mod mmu;
mod trap;

pub use trap::install_trap_vector;

pub fn shutdown() {
    //TODO
}
