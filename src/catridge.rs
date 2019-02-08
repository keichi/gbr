use std::fs::File;
use std::io::Read;

use io_device::IODevice;

pub struct Catridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mbc_type: u8,
    rom_bank: u8,
    ram_bank: u8,
}

impl Catridge {
    pub fn new(fname: &str) -> Self {
        let mut rom = Vec::new();
        let mut file = File::open(fname).unwrap();
        file.read_to_end(&mut rom).unwrap();

        let rom_size: usize = match rom[0x0148] {
            0 => 32 * 1024,
            n => 32 * 1024 << (n as usize),
        };
        let ram_size: usize = match rom[0x0149] {
            0 => 0,
            1 => 2 * 1024,
            2 => 8 * 1024,
            3 => 32 * 1024,
            4 => 128 * 1024,
            5 => 64 * 1024,
            _ => panic!("RAM size invalid"),
        };
        let mbc_type = rom[0x0147];

        let mut chksum: u8 = 0;
        for i in 0x0134..0x014d {
            chksum = chksum.wrapping_sub(rom[i]).wrapping_sub(1);
        }

        if rom_size != rom.len() {
            panic!("ROM file invalid");
        }

        if chksum != rom[0x014d] {
            panic!("ROM header is corrupted");
        }

        println!("ROM size {}", rom_size);
        println!("RAM size {}", ram_size);
        println!("MBC type {}", mbc_type);

        Catridge {
            rom: rom,
            ram: vec![0; ram_size],
            mbc_type: mbc_type,
            rom_bank: 0,
            ram_bank: 0,
        }
    }
}

impl IODevice for Catridge {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000...0x1fff => {
                // TODO RAM enable/disable
            }
            0x2000...0x3fff => {
                self.rom_bank = (self.rom_bank & 0xe0) | (val & 0x07);
            }
            0x4000...0x5fff => {
                self.rom_bank = (self.rom_bank & 0x07) | (val & 0xe0);
            }
            0xa000...0xbfff => println!("dame"),
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000...0x3fff => self.rom[addr as usize],
            0x4000...0x7fff => {
                let bank = match self.rom_bank {
                    0 | 0x20 | 0x40 | 0x60 => self.rom_bank + 1,
                    _ => self.rom_bank,
                };
                let offset = (16 * 1024) * bank as usize;
                self.rom[(addr & 0x3fff) as usize + offset]
            }
            0xa000...0xbfff => {
                println!("dame");
                0xff
            }
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    fn update(&mut self, _tick: u8) {}
}
