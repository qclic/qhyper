use core::arch::global_asm;

use aarch64_cpu::registers::*;

use super::context::TrapFrame;

global_asm!(include_str!("trap.S"));

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
enum TrapKind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
enum TrapSource {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[unsafe(no_mangle)]
fn invalid_exception(tf: &TrapFrame, kind: TrapKind, source: TrapSource) {
    panic!(
        "Invalid exception {:?} from {:?}:\n{:#x?}",
        kind, source, tf
    );
}

#[unsafe(no_mangle)]
fn handle_irq_exception(_tf: &TrapFrame) {}

fn handle_instruction_abort(tf: &TrapFrame, iss: u64, is_user: bool) {

    // Only handle Translation fault and Permission fault
}

fn handle_data_abort(tf: &TrapFrame, iss: u64, is_user: bool) {
    let wnr = (iss & (1 << 6)) != 0; // WnR: Write not Read
    let cm = (iss & (1 << 8)) != 0; // CM: Cache maintenance
}

#[unsafe(no_mangle)]
fn handle_sync_exception(tf: &mut TrapFrame) {
    let esr = ESR_EL1.extract();
    let iss = esr.read(ESR_EL1::ISS);
    match esr.read_as_enum(ESR_EL1::EC) {
        Some(ESR_EL1::EC::Value::SVC64) => {}
        Some(ESR_EL1::EC::Value::InstrAbortLowerEL) => handle_instruction_abort(tf, iss, true),
        Some(ESR_EL1::EC::Value::InstrAbortCurrentEL) => handle_instruction_abort(tf, iss, false),
        Some(ESR_EL1::EC::Value::DataAbortLowerEL) => handle_data_abort(tf, iss, true),
        Some(ESR_EL1::EC::Value::DataAbortCurrentEL) => handle_data_abort(tf, iss, false),
        Some(ESR_EL1::EC::Value::Brk64) => {
            tf.elr += 4;
        }
        _ => {}
    }
}
