use std::fs::File;
use std::io::{self, Read, Write};

use io_device::IODevice;
use ppu::PPU;
use timer::Timer;

pub struct MMU {
    boot_rom: Vec<u8>,
    rom: Vec<u8>,
    ram: [u8; 0x2000],
    hram: [u8; 0x7f],
    timer: Timer,
    ppu: PPU,
    pub int_flag: u8,
    pub int_enable: u8,
    boot_rom_enable: bool,
}

impl MMU {
    pub fn new() -> Self {
        MMU {
            boot_rom: Vec::new(),
            rom: Vec::new(),
            ram: [0; 0x2000],
            hram: [0; 0x7f],
            ppu: PPU::new(),
            timer: Timer::new(),
            int_flag: 0,
            int_enable: 0,
            boot_rom_enable: true,
        }
    }

    #[allow(dead_code)]
    pub fn load_boot_rom(&mut self, fname: &str) {
        let mut file = File::open(fname).unwrap();
        if file.read_to_end(&mut self.boot_rom).unwrap() != 0x100 {
            panic!("Boot ROM is corrupted");
        }
    }

    pub fn load_rom(&mut self, fname: &str) {
        let mut file = File::open(fname).unwrap();
        if file.read_to_end(&mut self.rom).unwrap() != 0x8000 {
            panic!("ROM is corrupted");
        }
    }

    fn print_char(&self, val: u8) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        write!(handle, "{}", val as char).unwrap();
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // VRAM
            0x8000...0x9fff => self.ppu.write(addr, val),
            // RAM
            0xc000...0xdfff => self.ram[(addr & 0x1fff) as usize] = val,
            // Echo RAM
            0xe000...0xfdff => self.ram[(addr - 0x2000) as usize] = val,
            // Serial Interface
            0xff01 => self.print_char(val),
            // Timer
            0xff04...0xff07 => self.timer.write(addr, val),
            // Interrupt flag
            0xff0f => self.int_flag = val,
            // PPU
            0xff40...0xff4b => self.ppu.write(addr, val),
            // Disable Boot ROM
            0xff50 => {
                self.boot_rom_enable = false;
                // self.ppu.dump_frame_buffer();
                // panic!("trap")
            }
            // HRAM
            0xff80...0xfffe => self.hram[(addr & 0x7f) as usize] = val,
            // Interrupt enable
            0xffff => self.int_enable = val,
            _ => (),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // Boot ROM
            0x0000...0x00ff if self.boot_rom_enable => self.boot_rom[addr as usize],
            // ROM
            0x0000...0x7fff => self.rom[(addr & 0x7fff) as usize],
            // VRAM
            0x8000...0x9fff => self.ppu.read(addr),
            // RAM
            0xc000...0xdfff => self.ram[(addr & 0x1fff) as usize],
            // Echo RAM
            0xe000...0xfdff => self.ram[(addr - 0x2000) as usize],
            // Timer
            0xff04...0xff07 => self.timer.read(addr),
            // Interrupt flag
            0xff0f => self.int_flag,
            // PPU
            0xff40...0xff4b => self.ppu.read(addr),
            // HRAM
            0xff80...0xfffe => self.hram[(addr & 0x7f) as usize],
            // Interrupt enable
            0xffff => self.int_enable,
            _ => 0xff,
        }
    }

    pub fn update(&mut self, tick: u8) {
        self.timer.update(tick);
        self.ppu.update(tick);

        if self.timer.irq_pending() {
            self.int_flag |= 0x4;
        }
    }
}
