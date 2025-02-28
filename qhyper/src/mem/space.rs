use core::cell::UnsafeCell;

use arrayvec::ArrayVec;
use memory_addr::VirtAddrRange;
use page_table_generic::{AccessSetting, CacheSetting};

use super::addr::PhysAddrRange;

pub static SPACE_SET: SpaceSet = SpaceSet(UnsafeCell::new(ArrayVec::new_const()));

pub struct SpaceSet(UnsafeCell<ArrayVec<Space, 24>>);
unsafe impl Send for SpaceSet {}
unsafe impl Sync for SpaceSet {}

impl SpaceSet {
    pub(crate) unsafe fn push(&self, space: Space) {
        (*self.0.get()).push(space);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Space> {
        unsafe { (*self.0.get()).iter() }
    }
}

pub struct Space {
    pub name: &'static str,
    pub phys: PhysAddrRange,
    pub offset: usize,
    pub access: AccessSetting,
    pub cache: CacheSetting,
}

impl Space {
    pub fn virt(&self) -> VirtAddrRange {
        VirtAddrRange::new(
            (self.phys.start.as_usize() + self.offset).into(),
            (self.phys.end.as_usize() + self.offset).into(),
        )
    }
}
