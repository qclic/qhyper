use core::{cell::UnsafeCell, ptr::slice_from_raw_parts};

use aux_mini::AuxMini;
use fdt_parser::Fdt;
use pl011::Pl011;

use crate::mem::space::Space;
mod aux_mini;
mod pl011;

static mut REG_BASE: usize = 0;
static UART: UartWapper = UartWapper(UnsafeCell::new(Uart::None));

struct UartWapper(UnsafeCell<Uart>);

unsafe impl Send for UartWapper {}
unsafe impl Sync for UartWapper {}

impl UartWapper {
    fn set(&self, uart: Uart) {
        unsafe {
            *self.0.get() = uart;
        }
    }
}

fn uart() -> &'static Uart {
    unsafe { &*UART.0.get() }
}

pub fn reg_range() -> &'static [u8] {
    unsafe { &*slice_from_raw_parts(REG_BASE as *const u8, 0x1000) }
}

pub fn put(byte: u8) {
    unsafe {
        match uart() {
            Uart::Pl011(uart) => uart.write(REG_BASE, byte),
            Uart::AuxMini(uart) => uart.write(REG_BASE, byte),
            Uart::None => {}
        }
    }
}
pub fn init_by_fdt(fdt: Fdt) -> Option<()> {
    let node = fdt.chosen()?;
    let stdout = node.stdout()?;

    unsafe {
        REG_BASE = stdout.node.reg()?.next()?.address as _;
    };

    for c in stdout.node.compatibles() {
        if c.contains("brcm,bcm2835-aux-uart") {
            UART.set(Uart::AuxMini(aux_mini::AuxMini {}));
            break;
        }

        if c.contains("arm,pl011") {
            UART.set(Uart::Pl011(Pl011 {}));
            break;
        }

        if c.contains("arm,primecell") {
            UART.set(Uart::Pl011(Pl011 {}));
            break;
        }
    }

    Some(())
}

enum Uart {
    None,
    Pl011(Pl011),
    AuxMini(AuxMini),
}

pub fn dbg(s: &str) {
    for c in s.bytes() {
        put(c);
    }
}

pub fn dbg_tb(s: &str, l: usize) {
    let mut b = s.bytes();
    for _ in 0..l {
        put(b.next().unwrap_or(b' '));
    }
}

pub fn dbgln(s: &str) {
    dbg(s);
    dbg("\r\n");
}

pub fn dbg_hexln(v: u64) {
    dbg_hex(v);
    dbg("\r\n");
}
pub fn dbg_mem(name: &str, mem: &[u8]) {
    let range = mem.as_ptr_range();
    dbg_range(name, (range.start as usize)..(range.end as usize));
}
pub fn dbg_range(name: &str, range: core::ops::Range<usize>) {
    dbg(name);
    dbg(": [");
    dbg_hex(range.start as _);
    dbg(", ");
    dbg_hex(range.end as _);
    dbg(")\r\n");
}

pub fn dbg_hex(v: u64) {
    const HEX_BUF_SIZE: usize = 20; // 最大长度，包括前缀"0x"和数字
    let mut hex_buf: [u8; HEX_BUF_SIZE] = [b'0'; HEX_BUF_SIZE];
    let mut n = v;
    dbg("0x");

    if n == 0 {
        dbg("0");
        return;
    }
    let mut i = 0;
    while n > 0 {
        let digit = n & 0xf;
        let ch = if digit < 10 {
            b'0' + digit as u8
        } else {
            b'a' + (digit - 10) as u8
        };
        n >>= 4; // 右移四位
        hex_buf[i] = ch;
        i += 1;
    }
    let s = &hex_buf[..i];
    for ch in s.iter().rev() {
        put(*ch);
    }
}
