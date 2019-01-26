use std::env;

#[macro_use]
extern crate log;
extern crate env_logger;

mod cpu;
mod io_device;
mod mmu;
mod timer;

fn main() {
    env_logger::init();

    let mut mmu = mmu::MMU::new();

    let args: Vec<String> = env::args().collect();
    mmu.load_rom(&args[1]);

    let mut cpu = cpu::CPU::new(mmu);

    loop {
        cpu.step();
    }
}
