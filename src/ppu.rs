use std::fs::File;
use std::io::Write;

use io_device::IODevice;

pub struct PPU {
    vram: [u8; 0x2000],
    /// LCD Control
    lcdc: u8,
    /// Status
    stat: u8,
    /// Scroll Y
    scy: u8,
    /// Scroll X
    scx: u8,
    /// Y-Coordinate
    ly: u8,
    /// LY Compare
    lyc: u8,
    /// DMA Transfer and Start Address
    dma: u8,
    /// Background Palette Data
    bgp: u8,
    /// Object Palette 0 Data
    obp0: u8,
    /// Object Palette 1 Data
    obp1: u8,
    /// Window Y Position
    wy: u8,
    /// Window X Position minus 7
    wx: u8,
    /// Interrupt request
    irq: bool,
    /// Elapsed clocks in current mode
    counter: u16,
    /// Frame buffer
    frame_buffer: [u8; 160 * 144],
}

impl PPU {
    // VRAM map
    // 0x0000-0x07ff: Tile data block #1
    // 0x0800-0x0fff: Tile data block #2
    // 0x1000-0x17ff: Tile data block #3
    // 0x1800-0x1bff: Tile map #1
    // 0x1c00-0x1fff: Tile map #2

    pub fn new() -> Self {
        PPU {
            vram: [0; 0x2000],
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            irq: false,
            counter: 0,
            frame_buffer: [0; 160 * 144],
        }
    }

    fn render(&mut self) {
        // TODO don't fetch tile index and data for every pixel
        // TODO Recheck type of every variable
        for i in 0..160 {
            let tile_x = (self.scx.wrapping_add(i) >> 3) as u16;
            let tile_y = (self.scy.wrapping_add(self.ly) >> 3) as u16;

            // Fetch tile index from tile map
            let tile_map_addr = 0x1800 | (tile_x + (tile_y << 5)) as u16;
            let tile_idx = self.vram[tile_map_addr as usize];

            // Offset of current pixel within tile
            let offset_x = self.scx.wrapping_add(i) & 0x7;
            let offset_y = self.scy.wrapping_add(self.ly) & 0x7;

            // Fetch tile data
            let tile_data_addr = ((tile_idx as u16) << 4) + (offset_y << 1) as u16;
            let tile0 = self.vram[tile_data_addr as usize];
            let tile1 = self.vram[(tile_data_addr + 1) as usize];

            let lo_bit = tile0 >> (7 - offset_x) & 1;
            let hi_bit = tile1 >> (7 - offset_x) & 1;

            let color = hi_bit << 1 | lo_bit;

            self.frame_buffer[(i as usize) + (self.ly as usize) * 160] = color;
        }
    }

    pub fn dump_frame_buffer(&mut self) {
        let mut buffer = File::create("foo.pgm").unwrap();

        writeln!(buffer, "P2").unwrap();
        writeln!(buffer, "160 144").unwrap();
        writeln!(buffer, "1").unwrap();

        for y in 0..144 {
            for x in 0..160 {
                write!(buffer, "{} ", self.frame_buffer[x + y * 160]).unwrap();
            }

            writeln!(buffer).unwrap();
        }
    }
}

impl IODevice for PPU {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // VRAM
            0x8000...0x9fff => {
                // VRAM is inaccessible during pixel transfer
                if self.stat & 0x3 != 3 || self.lcdc & 0x80 == 0 {
                    self.vram[(addr & 0x1fff) as usize] = val
                }
            }
            // IO registers
            0xff40 => self.lcdc = val,
            0xff41 => self.stat = (val & 0xf8) | (self.stat & 0x3),
            0xff42 => self.scy = val,
            0xff43 => self.scx = val,
            0xff44 => self.ly = 0,
            0xff45 => self.lyc = val,
            0xff46 => self.dma = val,
            0xff47 => self.bgp = val,
            0xff48 => self.obp0 = val,
            0xff49 => self.obp1 = val,
            0xff4a => self.wy = val,
            0xff4b => self.wx = val,

            _ => panic!("invalid address: 0x{:04x}", addr),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            // VRAM
            0x8000...0x9fff => {
                // VRAM is inaccessible during pixel transfer
                if self.stat & 0x3 != 3 || self.lcdc & 0x80 == 0 {
                    self.vram[(addr & 0x1fff) as usize]
                } else {
                    0xff
                }
            }
            // IO registers
            0xff40 => self.lcdc,
            0xff41 => self.stat,
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.ly,
            0xff45 => self.lyc,
            0xff46 => self.dma,
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,

            _ => panic!("invalid address: 0x{:04x}", addr),
        }
    }

    fn update(&mut self, tick: u8) {
        self.counter += tick as u16;

        match self.stat & 0x3 {
            // OAM Search (80 clocks)
            2 => {
                if self.counter >= 80 {
                    self.counter -= 80;
                    // Transition to Pixel Transfer mode
                    self.stat = (self.stat & 0xf8) | 3;
                    self.render();
                }
            }
            // Pixel Transfer (172 clocks)
            3 => {
                if self.counter >= 172 {
                    self.counter -= 172;
                    // Transition to H-Blank mode
                    self.stat = self.stat & 0xf8;
                }
            }
            // H-Blank (204 clocks)
            0 => {
                if self.counter >= 204 {
                    self.counter -= 204;
                    self.ly += 1;

                    if self.ly >= 144 {
                        // Transition to V-Blank mode
                        self.stat = (self.stat & 0xf8) | 1;
                    } else {
                        // Transition to OAM Search mode
                        self.stat = (self.stat & 0xf8) | 2;
                    }
                }
            }
            // V-Blank (4560 clocks or 10 lines)
            1 => {
                if self.counter >= 456 {
                    self.counter -= 456;
                    self.ly += 1;

                    if self.ly >= 154 {
                        // Transition to OAM Search mode
                        self.stat = (self.stat & 0xf8) | 2;
                        self.ly = 0;
                    }
                }
            }
            _ => panic!("Wrong"),
        }
    }

    fn irq_pending(&self) -> bool {
        self.irq
    }
}
