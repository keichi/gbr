#[macro_use]
extern crate log;
extern crate env_logger;

mod cpu;
mod mmu;

fn main() {
    env_logger::init();

    let mut mmu = mmu::MMU::new();
    let mut cpu = cpu::CPU::new(mmu);

    for _ in 0..5 {
        cpu.step();
    }
    cpu.dump();
}
