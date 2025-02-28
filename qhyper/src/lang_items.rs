use core::panic::PanicInfo;

use log::error;

use crate::arch::shutdown;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("kernel panic: {:?}", info);
    shutdown()
}
