use std::env;

#[macro_use]
extern crate log;
extern crate env_logger;

mod cpu;
mod mmu;

fn main() {
    env_logger::init();

    let mut mmu = mmu::MMU::new();
    // mmu.load_boot_rom("dmg_boot.bin");
    // mmu.load_rom("01-special.gb");
    // mmu.load_rom("03-op sp,hl.gb");
    // mmu.load_rom("04-op r,imm.gb");
    // mmu.load_rom("05-op rp.gb");
    // mmu.load_rom("06-ld r,r.gb");
    // mmu.load_rom("07-jr,jp,call,ret,rst.gb");
    // mmu.load_rom("08-misc instrs.gb");
    // mmu.load_rom("09-op r,r.gb");
    // mmu.load_rom("10-bit ops.gb");
    // mmu.load_rom("11-op a,(hl).gb");

    let args: Vec<String> = env::args().collect();
    mmu.load_rom(&args[1]);

    let mut cpu = cpu::CPU::new(mmu);

    loop {
        cpu.step();
        cpu.check_interrupt();
        // cpu.dump();
    }
}
