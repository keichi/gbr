use io_device::IODevice;

#[derive(Debug)]
pub struct PPU {
    vram: Vec<u8>,
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
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            vram: vec![0; 0x2000],
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
        }
    }
}

impl IODevice for PPU {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // VRAM
            0x8000...0x9fff => self.vram[(addr & 0x1fff) as usize] = val,

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
            0x8000...0x9fff => self.vram[(addr & 0x1fff) as usize],

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

                    if self.ly >= 143 {
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

                    if self.ly >= 153 {
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
