use spin::Mutex;
pub static VIRTIO_BRIDGE: Mutex<VirtioBridgeRegion> = Mutex::new(VirtioBridgeRegion::default());

pub struct VirtioBridgeRegion {
    base_address: usize, // el1 and el2 shared region addr, el2 virtual address
    pub is_enable: bool,
}

impl VirtioBridgeRegion {
    pub const fn default() -> Self {
        VirtioBridgeRegion {
            base_address: 0,
            is_enable: false,
        }
    }

    pub fn init_addr(&mut self, base_addr: usize) {
        self.base_address = base_addr;
        self.is_enable = true;
    }
}
