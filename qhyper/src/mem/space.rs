use arrayvec::ArrayVec;
use memory_addr::VirtAddrRange;
use page_table_generic::{AccessSetting, CacheSetting};

use super::{addr::PhysAddrRange, once::OnceStatic};

pub static SPACE_SET: SpaceSet = SpaceSet(OnceStatic::new(ArrayVec::new_const()));

pub struct SpaceSet(OnceStatic<ArrayVec<Space, 24>>);

impl SpaceSet {
    pub(crate) unsafe fn push(&self, space: Space) {
        (*self.0.get()).push(space);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Space> {
        self.0.iter()
    }
}

#[derive(Clone, Copy)]
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
