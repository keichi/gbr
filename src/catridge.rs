use std::fs::File;
use std::io::Read;

use io_device::IODevice;

pub struct Catridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    mbc_type: u8,
    ram_enable: bool,
    bank_no_upper: u8,
    bank_no_lower: u8,
    num_rom_banks: u8,
    mode: bool,
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
        let num_rom_banks = 2 << rom[0x0148];
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

        debug!("ROM size {}", rom_size);
        debug!("RAM size {}", ram_size);
        debug!("MBC type {}", mbc_type);

        Catridge {
            rom: rom,
            ram: vec![0; ram_size],
            mbc_type: mbc_type,
            ram_enable: false,
            bank_no_upper: 0,
            bank_no_lower: 0,
            num_rom_banks: num_rom_banks,
            mode: false,
        }
    }

    fn rom_bank_no(&self) -> u8 {
        let bank_no = if self.mode {
            self.bank_no_lower
        } else {
            self.bank_no_upper << 5 | self.bank_no_lower
        };

        let bank_no = match bank_no {
            0 | 0x20 | 0x40 | 0x60 => bank_no + 1,
            _ => bank_no,
        };

        bank_no & (self.num_rom_banks - 1)
    }

    fn ram_bank_no(&self) -> u8 {
        if self.mode {
            self.bank_no_upper
        } else {
            0
        }
    }
}

impl IODevice for Catridge {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // RAM enable
            0x0000...0x1fff => self.ram_enable = val & 0x0f > 0,
            // ROM bank number (lower 5 bits)
            0x2000...0x3fff => self.bank_no_lower = val & 0x1f,
            // RAM bank number or ROM bank number (upper 2 bits)
            0x4000...0x5fff => self.bank_no_upper = val & 0x03,
            // ROM/RAM mode select
            0x6000...0x7fff => self.mode = val & 0x01 > 0,
            // RAM bank 00-03
            0xa000...0xbfff => {
                if !self.ram_enable {
                    return;
                }
                let offset = (8 * 1024) * self.ram_bank_no() as usize;
                self.ram[(addr & 0x1fff) as usize + offset] = val
            }
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            // ROM bank 00
            0x0000...0x3fff => self.rom[addr as usize],
            // ROM bank 01-7f
            0x4000...0x7fff => {
                let offset = (16 * 1024) * self.rom_bank_no() as usize;
                self.rom[(addr & 0x3fff) as usize + offset]
            }
            // RAM bank 00-03
            0xa000...0xbfff => {
                if !self.ram_enable {
                    return 0xff;
                }
                let offset = (8 * 1024) * self.ram_bank_no() as usize;
                self.ram[(addr & 0x1fff) as usize + offset]
            }
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    fn update(&mut self, _tick: u8) {}
}
