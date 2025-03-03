use buddy_system_allocator::Heap;
use log::debug;
use page_table_generic::Access;
pub use page_table_generic::PTEGeneric;
use spin::MutexGuard;

use crate::percpu::cpu_data;

use super::HEAP_ALLOCATOR;

pub fn init() {
    let data = cpu_data();
    debug!("Init cpu {} MMU", data.id);

    let access = HeapGuard(HEAP_ALLOCATOR.lock());

    
}

struct HeapGuard<'a>(MutexGuard<'a, Heap<32>>);

impl Access for HeapGuard<'_> {
    fn va_offset(&self) -> usize {
        0
    }

    unsafe fn alloc(&mut self, layout: core::alloc::Layout) -> Option<core::ptr::NonNull<u8>> {
        todo!()
    }

    unsafe fn dealloc(&mut self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        todo!()
    }
}