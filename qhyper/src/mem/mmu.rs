use buddy_system_allocator::Heap;
use log::debug;
pub use page_table_generic::PTEGeneric;
use page_table_generic::{Access, MapConfig};
use spin::MutexGuard;

use crate::{
    arch::mmu::{set_table, TableRef},
    mem::space::SPACE_SET,
    percpu::cpu_data,
};

use super::{space::Space, HEAP_ALLOCATOR};

pub fn init() {
    let data = cpu_data();
    debug!("Init cpu {} MMU", data.id);

    let mut access = HeapGuard(HEAP_ALLOCATOR.lock());

    let mut table = TableRef::create_empty(&mut access).unwrap();

    for space in SPACE_SET.iter() {
        map_space(&mut table, space, &mut access);
    }

    set_table(table);
}

struct HeapGuard<'a>(MutexGuard<'a, Heap<32>>);

impl Access for HeapGuard<'_> {
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

fn map_space(table: &mut TableRef, space: &Space, access: &mut impl Access) {
    let paddr = space.phys.start.as_usize();
    let vaddr = space.virt().start.as_ptr();
    let len = space.phys.size();

    debug!(
        "map {:<8}:[ {: >12x}, {: >12x} ) -> [ {: >12x}, {: >12x} )",
        space.name,
        vaddr as usize,
        space.virt().end.as_usize(),
        paddr,
        space.phys.end.as_usize()
    );

    unsafe {
        table
            .map_region(
                MapConfig::new(vaddr, paddr, space.access, space.cache),
                len,
                true,
                access,
            )
            .unwrap();
    }
}
