use io_device::IODevice;

pub struct PPU {
    vram: [u8; 0x2000],
    oam: [u8; 0xa0],
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
    /// V-Blank interrupt request
    pub irq_vblank: bool,
    /// LCDC interrupt request
    pub irq_lcdc: bool,
    /// Elapsed clocks in current mode
    counter: u16,
    /// Frame buffer
    frame_buffer: [u8; 160 * 144],
}

impl PPU {
    // VRAM map
    // 0x0000-0x07ff: Tile set #1
    // 0x0800-0x0fff: Tile set #2
    // 0x1000-0x17ff: Tile set #3
    // 0x1800-0x1bff: Tile map #1
    // 0x1c00-0x1fff: Tile map #2

    pub fn new() -> Self {
        PPU {
            vram: [0; 0x2000],
            oam: [0; 0xa0],
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
            irq_vblank: false,
            irq_lcdc: false,
            counter: 0,
            frame_buffer: [0; 160 * 144],
        }
    }

    fn fetch_tile(&self, tile_x: u8, tile_y: u8, offset_y: u8) -> (u8, u8) {
        // Fetch tile index from tile map
        // TODO Tile map addressing modes
        let tile_map_addr = 0x1800 | (tile_x as u16 + ((tile_y as u16) << 5));
        let tile_idx = self.vram[tile_map_addr as usize];

        // Fetch tile data from tile set
        let tile_data_base = if self.lcdc & 0x10 > 0 {
            // Use tile set #1 (0x0000-0x07ff) and #2 (0x0800-0x0fff)
            (tile_idx as u16) << 4
        } else {
            // Use tile set #2 (0x0800-0x0fff) and #3 (0x1000-0x17ff)
            (0x1000 as u16).wrapping_add(((tile_idx as i8 as i16) << 4) as u16)
        };
        let tile_data_addr = tile_data_base + (offset_y << 1) as u16;

        let tile0 = self.vram[tile_data_addr as usize];
        let tile1 = self.vram[(tile_data_addr + 1) as usize];

        (tile0, tile1)
    }

    fn map_color(&self, color_no: u8, palette: u8) -> u8 {
        match (palette >> (color_no << 1)) & 0x3 {
            0 => 0xff,
            1 => 0xaa,
            2 => 0x55,
            3 | _ => 0x00,
        }
    }

    fn render_bg(&mut self) {
        // Tile coordinate
        let mut tile_x = self.scx >> 3;
        let tile_y = self.scy.wrapping_add(self.ly) >> 3;

        // Offset of current pixel within tile
        let mut offset_x = self.scx & 0x7;
        let offset_y = self.scy.wrapping_add(self.ly) & 0x7;

        let mut tile = self.fetch_tile(tile_x, tile_y, offset_y);

        for x in 0..160 {
            let lo_bit = tile.0 >> (7 - offset_x) & 1;
            let hi_bit = tile.1 >> (7 - offset_x) & 1;

            let color_no = hi_bit << 1 | lo_bit;
            let color = self.map_color(color_no, self.bgp);

            self.frame_buffer[(x as usize) + (self.ly as usize) * 160] = color;

            offset_x += 1;

            // Move on to next tile
            if offset_x >= 8 {
                offset_x = 0;
                tile_x += 1;

                if tile_x >= 32 {
                    tile_x = 0;
                }

                tile = self.fetch_tile(tile_x, tile_y, offset_y);
            }
        }
    }

    fn fetch_sprite(&self, tile_idx: u8, offset_y: u8) -> (u8, u8) {
        // Fetch tile data from tile set
        let tile_data_base = (tile_idx as u16) << 4;
        let tile_data_addr = tile_data_base + (offset_y << 1) as u16;

        let tile0 = self.vram[tile_data_addr as usize];
        let tile1 = self.vram[(tile_data_addr + 1) as usize];

        (tile0, tile1)
    }

    fn render_sprites(&mut self) {
        // TODO 10 sprites per scanline
        // TODO Flip x and y
        // TODO sprite and background priority

        for i in 0..40 {
            // Parse OAM entry
            let entry_addr = i << 2;
            let sprite_y = self.oam[entry_addr];
            let sprite_x = self.oam[entry_addr + 1];
            let tile_no = self.oam[entry_addr + 2];
            let flags = self.oam[entry_addr + 3];

            let obj_prio = flags & 0x80 > 0;
            let flip_y = flags & 0x40 > 0;
            let flip_x = flags & 0x20 > 0;
            let palette = if flags & 0x10 > 0 {
                self.obp1
            } else {
                self.obp0
            };

            // Check if sprite is visible
            if sprite_y <= self.ly + 8 || sprite_y > self.ly + 16 {
                continue;
            }
            if sprite_x == 0 || sprite_x > 160 + 8 - 1 {
                continue;
            }

            let tile = self.fetch_sprite(tile_no, self.ly + 16 - sprite_y);

            for offset_x in 0..8 {
                let lo_bit = tile.0 >> (7 - offset_x) & 1;
                let hi_bit = tile.1 >> (7 - offset_x) & 1;

                let color_no = hi_bit << 1 | lo_bit;
                if color_no == 0 {
                    continue;
                }
                let color = self.map_color(color_no, palette);

                if offset_x + sprite_x < 8 {
                    continue;
                }

                let x = offset_x + sprite_x - 8;
                self.frame_buffer[(x as usize) + (self.ly as usize) * 160] = color;
            }
        }
    }

    fn render_scanline(&mut self) {
        if self.lcdc & 0x1 > 0 {
            self.render_bg();
        }
        if self.lcdc & 0x2 > 0 {
            self.render_sprites();
        }
    }

    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    pub fn mode(&self) -> u8 {
        self.stat & 0x3
    }

    fn update_lyc_interrupt(&mut self) {
        if self.ly == self.lyc {
            self.stat |= 0x4;

            if self.stat & 0x40 > 0 {
                self.irq_lcdc = true;
            }
        } else {
            self.stat &= !0x4;
        }
    }

    fn update_lcdc_interrupt(&mut self) {
        self.irq_lcdc = match self.stat & 0x3 {
            // H-Blank interrupt
            0 if self.lcdc & 0x8 > 0 => true,
            // V-Blank interrupt
            1 if self.lcdc & 0x10 > 0 => true,
            // OAM Search interrupt
            2 if self.lcdc & 0x20 > 0 => true,
            _ => false,
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

            // OAM
            0xfe00...0xfe9f => {
                // OAM is only accessible during H-Blank and V-Blank
                if self.stat & 0x3 == 0 || self.stat & 0x3 == 1 || self.lcdc & 0x80 == 0 {
                    self.oam[(addr & 0x00ff) as usize] = val;
                }
            }

            // IO registers
            0xff40 => self.lcdc = val,
            0xff41 => self.stat = (val & 0xf8) | (self.stat & 0x3),
            0xff42 => self.scy = val,
            0xff43 => self.scx = val,
            0xff44 => self.ly = 0,
            0xff45 => self.lyc = val,
            0xff47 => self.bgp = val,
            0xff48 => self.obp0 = val,
            0xff49 => self.obp1 = val,
            0xff4a => self.wy = val,
            0xff4b => self.wx = val,

            _ => panic!("Invalid address: 0x{:04x}", addr),
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

            // OAM
            0xfe00...0xfe9f => {
                // OAM is only accessible during H-Blank and V-Blank
                if self.stat & 0x3 == 0 || self.stat & 0x3 == 1 || self.lcdc & 0x80 == 0 {
                    self.oam[(addr & 0x00ff) as usize]
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

            _ => panic!("Invalid address: 0x{:04x}", addr),
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
                    self.render_scanline();
                }
            }
            // Pixel Transfer (172 clocks)
            3 => {
                if self.counter >= 172 {
                    self.counter -= 172;
                    // Transition to H-Blank mode
                    self.stat = self.stat & 0xf8;
                    self.update_lcdc_interrupt();
                }
            }
            // H-Blank (204 clocks)
            0 => {
                if self.counter >= 204 {
                    self.counter -= 204;
                    self.ly += 1;

                    self.update_lyc_interrupt();

                    if self.ly >= 144 {
                        // Transition to V-Blank mode
                        self.stat = (self.stat & 0xf8) | 1;
                        self.irq_vblank = true;
                        self.update_lcdc_interrupt();
                    } else {
                        // Transition to OAM Search mode
                        self.stat = (self.stat & 0xf8) | 2;
                        self.update_lcdc_interrupt();
                    }
                }
            }
            // V-Blank (4560 clocks or 10 lines)
            1 | _ => {
                if self.counter >= 456 {
                    self.counter -= 456;
                    self.ly += 1;

                    self.update_lyc_interrupt();

                    if self.ly >= 154 {
                        // Transition to OAM Search mode
                        self.stat = (self.stat & 0xf8) | 2;
                        self.ly = 0;
                        self.update_lcdc_interrupt();
                    }
                }
            }
        }
    }
}
