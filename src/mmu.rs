use std::fs::File;
use std::io::Read;

use io_device::IODevice;
use joypad::Joypad;
use ppu::PPU;
use timer::Timer;

pub struct MMU {
    boot_rom: Vec<u8>,
    rom: Vec<u8>,
    ram: [u8; 0x2000],
    hram: [u8; 0x7f],
    pub joypad: Joypad,
    timer: Timer,
    // TODO should this be public?
    pub ppu: PPU,
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
            joypad: Joypad::new(),
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

    // TODO OAM DMA Timing
    fn do_dma(&mut self, val: u8) {
        if val < 0x80 || 0xdf < val {
            panic!("Invalid DMA source address")
        }

        let src_base = (val as u16) << 8;
        let dst_base = 0xfe00;

        for i in 0..0xa0 {
            let tmp = self.read(src_base | i);
            self.write(dst_base | i, tmp);
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // VRAM
            0x8000...0x9fff => self.ppu.write(addr, val),
            // RAM
            0xc000...0xdfff => self.ram[(addr & 0x1fff) as usize] = val,
            // Echo RAM
            0xe000...0xfdff => self.ram[((addr - 0x2000) & 0x1fff) as usize] = val,
            // OAM
            0xfe00...0xfe9f => self.ppu.write(addr, val),
            // Joypad
            0xff00 => self.joypad.write(addr, val),
            // Timer
            0xff04...0xff07 => self.timer.write(addr, val),
            // Interrupt flag
            0xff0f => self.int_flag = val,
            // PPU
            0xff40...0xff45 | 0xff47...0xff4b => self.ppu.write(addr, val),
            // OAM DMA
            0xff46 => self.do_dma(val),
            // Disable Boot ROM
            0xff50 => {
                self.boot_rom_enable = false;
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
            0xe000...0xfdff => self.ram[((addr - 0x2000) & 0x1fff) as usize],
            // OAM
            0xfe00...0xfe9f => self.ppu.read(addr),
            // Joypad
            0xff00 => self.joypad.read(addr),
            // Timer
            0xff04...0xff07 => self.timer.read(addr),
            // Interrupt flag
            0xff0f => self.int_flag,
            // PPU
            0xff40...0xff45 | 0xff47...0xff4b => self.ppu.read(addr),
            // HRAM
            0xff80...0xfffe => self.hram[(addr & 0x7f) as usize],
            // Interrupt enable
            0xffff => self.int_enable,
            _ => 0xff,
        }
    }

    pub fn update(&mut self, tick: u8) {
        self.ppu.update(tick);
        self.timer.update(tick);

        if self.ppu.irq_vblank {
            self.int_flag |= 0x1;
            self.ppu.irq_vblank = false;
        }

        if self.ppu.irq_lcdc {
            self.int_flag |= 0x2;
            self.ppu.irq_lcdc = false;
        }

        if self.timer.irq {
            self.int_flag |= 0x4;
            self.timer.irq = false;
        }
    }
}
