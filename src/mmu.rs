use std::fs::File;
use std::io::Read;

#[derive(Debug)]
pub struct MMU {
    boot_rom: Vec<u8>,
}

impl MMU {
    pub fn new() -> Self {
        let mut mmu = MMU {
            boot_rom: Vec::new(),
        };

        let mut file = File::open("dmg_boot.bin").unwrap();
        if file.read_to_end(&mut mmu.boot_rom).unwrap() != 0x100 {
            panic!("Boot ROM is corrupted");
        }

        mmu
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        debug!("MEM write [0x{:04x}] = 0x{:02x}", addr, val);
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // Boot ROM
            0x0000...0x00ff => self.boot_rom[addr as usize],
            _ => 0xff,
        }
    }
}
