#[macro_use]
extern crate log;
extern crate env_logger;

mod cpu;
mod mmu;

fn main() {
    env_logger::init();

    let mut mmu = mmu::MMU::new();
    let mut cpu = cpu::CPU::new(mmu);

    loop {
        cpu.step();
        cpu.dump();
    }
    cpu.dump();
}
