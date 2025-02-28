pub use memory_addr::*;

pub trait VirtToPhys {
    fn to_phys(&self) -> PhysAddr;
}

pub trait PhysToVirt {
    fn to_virt(&self) -> VirtAddr;
}

// impl VirtToPhys for VirtAddr {
//     fn to_phys(&self) -> PhysAddr {
//         let ptr = &self.as_ptr();

//         let offset =
//         if text().as_ptr_range().contains(ptr) ||
//             rodata().as_ptr_range().contains(ptr) ||
//             data().as_ptr_range().contains(ptr) ||
//             bss().as_ptr_range().contains(ptr)
//             {
//             VM_VA_OFFSET
//         } else if stack().as_ptr_range().contains(ptr) {

//         } else {
//             panic!("VirtToPhys: invalid pointer")
//         };

//     }
// }
