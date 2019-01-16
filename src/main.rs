#[macro_use]
extern crate log;
extern crate env_logger;

mod cpu;
mod mmu;

fn main() {
    env_logger::init();

    let mut mmu = mmu::MMU::new();
    mmu.load_boot_rom("dmg_boot.bin");
    // mmu.load_rom("04-op r,imm.gb");
    // mmu.load_rom("05-op rp.gb");
    // mmu.load_rom("06-ld r,r.gb");
    // mmu.load_rom("08-misc instrs.gb");
    mmu.load_rom("09-op r,r.gb");

    let mut cpu = cpu::CPU::new(mmu);

    loop {
        cpu.step();
        // cpu.dump();
    }
}
