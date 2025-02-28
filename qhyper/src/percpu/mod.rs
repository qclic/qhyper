use crate::consts::STACK_SIZE;

pub struct PerCpu {
    pub id: usize,
    pub stack: [u8; STACK_SIZE],
}



pub fn init() {

}
