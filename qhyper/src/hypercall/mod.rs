use log::{info, warn, debug};
use crate::error::HvError;
use crate::percpu::PerCpu;
numeric_enum_macro::numeric_enum! {
    #[repr(u64)]
    #[derive(Debug, Eq, PartialEq, Copy, Clone)]
    pub enum HyperCallID {
        VirtioInit = 0,
        VirtioInjectIrq = 1,
        CellStart = 2,
        CellOff = 3,
        CellList = 4,
        ClearInjectIrq = 20,
        IvcInfo = 5,
    }
}

pub type HyperCallResult = core::result::Result<usize, HvError>;

pub struct HyperCall<'live> {
    cpu_data: &'live mut PerCpu,
}

impl<'live> HyperCall<'live> {
    pub fn new(cpu_data: &'live mut PerCpu) -> Self {
        Self { cpu_data }
    }

    pub fn hypercall(&mut self, id: u64, arg0: u64, arg1: u64) -> HyperCallResult {
        let id = match HyperCallID::try_from(id) {
            Ok(id) => id,
            Err(_) => {
                warn!("hypercall id={} unsupported!", id);
                return Ok(0);
            }
        };

        debug!(
            "hypercall: code={:?}, arg0={:#x}, arg1={:#x}",
            id, arg0, arg1
        );

        unsafe {
            match id {
                HyperCallID::VirtioInit => self.hv_virtio_init(arg0),
                _ => {
                    warn!("hypercall id={} unsupported!", id as u64);
                    Ok(0)
                }
            }
        }
    }


    fn hv_virtio_init(&mut self, shared_region_addr: u64) -> HyperCallResult {

        HyperCallResult::Ok(0)
    }
}