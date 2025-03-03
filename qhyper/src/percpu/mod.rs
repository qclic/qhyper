use core::{alloc::Layout, fmt::Display};

use alloc::collections::btree_map::BTreeMap;
use log::debug;
use memory_addr::{pa_range, PhysAddrRange};

use crate::{
    consts::STACK_SIZE,
    mem::{get_fdt, once::OnceStatic, stack0},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct CPUHardId(usize);

impl From<usize> for CPUHardId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct CPUId(usize);

impl Display for CPUHardId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}
impl Display for CPUId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

static HARD_TO_SOFT: OnceStatic<BTreeMap<CPUHardId, CPUId>> = OnceStatic::new(BTreeMap::new());
static SOFT_TO_HARD: OnceStatic<BTreeMap<CPUId, CPUHardId>> = OnceStatic::new(BTreeMap::new());
static PER_CPU: OnceStatic<BTreeMap<CPUId, PerCpu>> = OnceStatic::new(BTreeMap::new());

#[derive(Debug)]
pub struct PerCpu {
    pub id: CPUId,
    pub stack: PhysAddrRange,
}

impl From<CPUHardId> for CPUId {
    fn from(value: CPUHardId) -> Self {
        unsafe { *(*HARD_TO_SOFT.get()).get(&value).unwrap() }
    }
}

impl From<CPUId> for CPUHardId {
    fn from(value: CPUId) -> Self {
        unsafe { *(*SOFT_TO_HARD.get()).get(&value).unwrap() }
    }
}

pub fn init() {
    if let Some(fdt) = get_fdt() {
        for (i, cpu) in fdt.find_nodes("/cpus/cpu").enumerate() {
            let soft_id = CPUId(i);
            if let Some(mut reg) = cpu.reg() {
                let hard_id = CPUHardId(reg.next().unwrap().address as usize);
                unsafe {
                    (*HARD_TO_SOFT.get()).insert(hard_id, soft_id);
                    (*SOFT_TO_HARD.get()).insert(soft_id, hard_id);
                }
            }
        }
    }
    //TODO 非fdt平台

    unsafe {
        for &id in SOFT_TO_HARD.keys() {
            let stack = if id == CPUId(0) {
                stack0()
            } else {
                let stack = alloc::alloc::alloc_zeroed(
                    Layout::from_size_align(STACK_SIZE, 0x1000).unwrap(),
                );
                if stack.is_null() {
                    panic!("alloc stack failed")
                }
                let stack = stack as usize;
                pa_range!(stack..stack + STACK_SIZE)
            };
            (*PER_CPU.get()).insert(id, PerCpu { id, stack });
        }
    }

    debug!("PreCPU data ok");
}

pub fn cpu_data() -> &'static PerCpu {
    unsafe {
        let id = CPUId::from(crate::arch::cpu_id());
        &(*PER_CPU.get())[&id]
    }
}
