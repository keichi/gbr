use std::fs::File;
use std::io::{self,Read,Write};

#[derive(Debug)]
pub struct MMU {
    boot_rom: Vec<u8>,
    rom: Vec<u8>,
    ram: Vec<u8>,
    hram: Vec<u8>,
}

impl MMU {
    pub fn new() -> Self {
        let mut mmu = MMU {
            boot_rom: Vec::new(),
            rom: Vec::new(),
            ram: vec![0; 0x2000],
            hram: vec![0; 0x7f],
        };

        let mut file = File::open("dmg_boot.bin").unwrap();
        if file.read_to_end(&mut mmu.boot_rom).unwrap() != 0x100 {
            panic!("Boot ROM is corrupted");
        }

        let mut file = File::open("06-ld r,r.gb").unwrap();
        if file.read_to_end(&mut mmu.rom).unwrap() != 0x8000 {
            panic!("ROM is corrupted");
        }

        mmu
    }

    fn print_char(&self, val: u8) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        write!(handle, "{}", val as char).unwrap();
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        debug!("MEM write [0x{:04x}] = 0x{:02x}", addr, val);

        match addr {
            // RAM
            0xc000...0xdfff => self.ram[(addr & 0x1fff) as usize] = val,
            // Serial Interface
            0xff01 => self.print_char(val),
            // HRAM
            0xff80...0xfffe => self.hram[(addr & 0x7f) as usize] = val,
            _ => (),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // Boot ROM
            0x0000...0x00ff => self.boot_rom[addr as usize],
            // ROM
            0x0100...0x7fff => self.rom[(addr & 0x7fff) as usize],
            // RAM
            0xc000...0xdfff => self.ram[(addr & 0x1fff) as usize],
            // HRAM
            0xff80...0xfffe => self.hram[(addr & 0x7f) as usize],
            _ => 0xff,
        }
    }

    pub fn write16(&mut self, addr: u16, val: u16) {
        self.write(addr, (val & 0xff) as u8);
        self.write(addr.wrapping_add(1), (val >> 8 & 0xff) as u8);
    }

    pub fn read16(&self, addr: u16) -> u16 {
        let lo = self.read(addr);
        let hi = self.read(addr.wrapping_add(1));

        (hi as u16) << 8 | lo as u16
    }
}
