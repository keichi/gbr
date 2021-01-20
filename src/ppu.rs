use io_device::IODevice;

/// Width of screen in pixels.
const SCREEN_W: u8 = 160;
/// Height of screen in pixels.
const SCREEN_H: u8 = 144;

#[derive(Copy, Clone, PartialEq)]
enum BGPriority {
    Color0,
    Color123,
}

/// Pixel Processing Unit.
pub struct PPU {
    /// VRAM
    vram: [u8; 0x2000],
    /// OAM
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
    frame_buffer: [u8; (SCREEN_W as usize) * (SCREEN_H as usize)],
    /// Current scanline
    scanline: [u8; SCREEN_W as usize],
    /// Background priority
    bg_prio: [BGPriority; SCREEN_W as usize],
}

impl PPU {
    // VRAM map
    // 0x0000-0x07ff: Tile set #1
    // 0x0800-0x0fff: Tile set #2
    // 0x1000-0x17ff: Tile set #3
    // 0x1800-0x1bff: Tile map #1
    // 0x1c00-0x1fff: Tile map #2

    /// Creates a new `PPU`
    pub fn new() -> Self {
        PPU {
            vram: [0; 0x2000],
            oam: [0; 0xa0],
            lcdc: 0x80,
            stat: 0x02,
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
            scanline: [0; SCREEN_W as usize],
            frame_buffer: [0; (SCREEN_W as usize) * (SCREEN_H as usize)],
            bg_prio: [BGPriority::Color0; SCREEN_W as usize],
        }
    }

    /// Fetches tile data from VRAM.
    fn fetch_tile(&self, tile_no: u8, offset_y: u8, tile_data_sel: bool) -> (u8, u8) {
        // Fetch tile data from tile set
        let tile_data_addr = if tile_data_sel {
            // Use tile set #1 (0x0000-0x07ff) and #2 (0x0800-0x0fff)
            (tile_no as u16) << 4
        } else {
            // Use tile set #2 (0x0800-0x0fff) and #3 (0x1000-0x17ff)
            (0x1000 as u16).wrapping_add(((tile_no as i8 as i16) << 4) as u16)
        };
        let row_addr = tile_data_addr + (offset_y << 1) as u16;

        let tile0 = self.vram[row_addr as usize];
        let tile1 = self.vram[(row_addr + 1) as usize];

        (tile0, tile1)
    }

    /// Fetches BG or Window tile data from VRAM.
    fn fetch_bg_window_tile(
        &self,
        tile_x: u8,
        tile_y: u8,
        offset_y: u8,
        tile_map_base: u16,
    ) -> (u8, u8) {
        // Fetch tile index from tile map
        let tile_map_addr = tile_map_base | ((tile_x & 0x1f) as u16 + ((tile_y as u16) << 5));
        let tile_no = self.vram[tile_map_addr as usize];

        self.fetch_tile(tile_no, offset_y, self.lcdc & 0x10 > 0)
    }

    /// Fetches BG tile data from VRAM.
    fn fetch_bg_tile(&self, tile_x: u8, tile_y: u8, offset_y: u8) -> (u8, u8) {
        // Fetch tile index from tile map
        let tile_map_base = if self.lcdc & 0x8 > 0 { 0x1c00 } else { 0x1800 };

        self.fetch_bg_window_tile(tile_x, tile_y, offset_y, tile_map_base)
    }

    /// Fetches Window tile data from VRAM.
    fn fetch_window_tile(&self, tile_x: u8, tile_y: u8, offset_y: u8) -> (u8, u8) {
        // Fetch tile index from tile map
        let tile_map_base = if self.lcdc & 0x40 > 0 { 0x1c00 } else { 0x1800 };

        self.fetch_bg_window_tile(tile_x, tile_y, offset_y, tile_map_base)
    }

    /// Converts color number to brightness using palette.
    fn map_color(&self, color_no: u8, palette: u8) -> u8 {
        match (palette >> (color_no << 1)) & 0x3 {
            0 => 0xff,
            1 => 0xaa,
            2 => 0x55,
            3 | _ => 0x00,
        }
    }

    /// Returns the color number at a given position from tile data.
    fn get_color_no(&self, tile: (u8, u8), bitpos: u8) -> u8 {
        let lo_bit = tile.0 >> bitpos & 1;
        let hi_bit = tile.1 >> bitpos & 1;

        hi_bit << 1 | lo_bit
    }

    /// Renders BG.
    fn render_bg(&mut self) {
        // Tile coordinate
        let mut tile_x = self.scx >> 3;
        let mut tile_y = self.scy.wrapping_add(self.ly) >> 3;

        // Offset of current pixel within tile
        let mut offset_x = self.scx & 0x7;
        let mut offset_y = self.scy.wrapping_add(self.ly) & 0x7;

        let mut tile = self.fetch_bg_tile(tile_x, tile_y, offset_y);

        let mut window = false;

        for x in 0..SCREEN_W {
            // Check if window is enabled
            if self.lcdc & 0x20 > 0 {
                if self.wy <= self.ly && self.wx == x + 7 {
                    tile_x = 0;
                    tile_y = (self.ly - self.wy) >> 3;
                    offset_x = 0;
                    offset_y = (self.ly - self.wy) & 0x7;
                    tile = self.fetch_window_tile(tile_x, tile_y, offset_y);
                    window = true;
                }
            }

            let color_no = self.get_color_no(tile, 7 - offset_x);
            let color = self.map_color(color_no, self.bgp);

            self.bg_prio[x as usize] = if color_no == 0 {
                BGPriority::Color0
            } else {
                BGPriority::Color123
            };

            self.scanline[x as usize] = color;

            offset_x += 1;

            // Move on to next tile
            if offset_x >= 8 {
                offset_x = 0;
                tile_x += 1;

                if window {
                    tile = self.fetch_window_tile(tile_x, tile_y, offset_y);
                } else {
                    tile = self.fetch_bg_tile(tile_x, tile_y, offset_y);
                }
            }
        }
    }

    /// Renders sprites.
    fn render_sprites(&mut self) {
        let mut n_sprites = 0;
        let height = if self.lcdc & 0x4 > 0 { 16 } else { 8 };

        for i in 0..40 {
            // Parse OAM entry
            let entry_addr = i << 2;
            let sprite_y = self.oam[entry_addr];
            let sprite_x = self.oam[entry_addr + 1];
            let flags = self.oam[entry_addr + 3];

            let obj_prio = flags & 0x80 > 0;
            let flip_y = flags & 0x40 > 0;
            let flip_x = flags & 0x20 > 0;
            let palette = if flags & 0x10 > 0 {
                self.obp1
            } else {
                self.obp0
            };

            // Check if sprite is visible on this scanline
            if sprite_y <= self.ly + 16 - height || sprite_y > self.ly + 16 {
                continue;
            }

            // Up to 10 sprites can be rendered on one scanline
            n_sprites += 1;
            if n_sprites > 10 {
                break;
            }

            // Check if sprite is within the screen
            if sprite_x == 0 || sprite_x > SCREEN_W + 8 - 1 {
                continue;
            }

            // Tile number
            let tile_no = if self.lcdc & 0x4 > 0 {
                // 8x16 sprite
                if (self.ly + 8 < sprite_y) ^ flip_y {
                    self.oam[entry_addr + 2] & 0xfe
                } else {
                    self.oam[entry_addr + 2] | 0x01
                }
            } else {
                // 8x8 sprite
                self.oam[entry_addr + 2]
            };

            // Y-offset within the tile
            let offset_y = if flip_y {
                7 - ((self.ly + 16 - sprite_y) & 0x7)
            } else {
                (self.ly + 16 - sprite_y) & 0x7
            };

            // Fetch tile data
            let tile = self.fetch_tile(tile_no, offset_y, true);

            for offset_x in 0..8 {
                if offset_x + sprite_x < 8 {
                    continue;
                }

                let x = offset_x + sprite_x - 8;

                if x >= SCREEN_W {
                    break;
                }

                let bitpos = if flip_x { offset_x } else { 7 - offset_x };
                let color_no = self.get_color_no(tile, bitpos);
                if color_no == 0 {
                    continue;
                }
                if self.bg_prio[x as usize] == BGPriority::Color123 && obj_prio {
                    continue;
                }
                let color = self.map_color(color_no, palette);

                self.scanline[x as usize] = color;
            }
        }
    }

    /// Renders a scanline.
    fn render_scanline(&mut self) {
        if self.lcdc & 0x1 > 0 {
            self.render_bg();
        }
        if self.lcdc & 0x2 > 0 {
            self.render_sprites();
        }

        for x in 0..SCREEN_W {
            let ix = (x as usize) + (self.ly as usize) * (SCREEN_W as usize);
            self.frame_buffer[ix] = self.scanline[x as usize];
        }
    }

    /// Returns the current contents of the frame buffer.
    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    /// Checks LYC interrupt.
    fn update_lyc_interrupt(&mut self) {
        // LYC=LY coincidence interrupt
        if self.ly == self.lyc {
            self.stat |= 0x4;

            if self.stat & 0x40 > 0 {
                self.irq_lcdc = true;
            }
        } else {
            self.stat &= !0x4;
        }
    }

    /// Checks LCD mode interrupt.
    fn update_mode_interrupt(&mut self) {
        // Mode interrupts
        match self.stat & 0x3 {
            // H-Blank interrupt
            0 if self.stat & 0x8 > 0 => self.irq_lcdc = true,
            // V-Blank interrupt
            1 if self.stat & 0x10 > 0 => self.irq_lcdc = true,
            // OAM Search interrupt
            2 if self.stat & 0x20 > 0 => self.irq_lcdc = true,
            _ => (),
        }
    }
}

impl IODevice for PPU {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // VRAM
            0x8000..=0x9fff => {
                // VRAM is inaccessible during pixel transfer
                if self.stat & 0x3 != 3 {
                    self.vram[(addr & 0x1fff) as usize] = val
                }
            }

            // OAM
            0xfe00..=0xfe9f => {
                // OAM is only accessible during H-Blank and V-Blank
                if self.stat & 0x3 == 0 || self.stat & 0x3 == 1 {
                    self.oam[(addr & 0x00ff) as usize] = val;
                }
            }

            // IO registers
            0xff40 => {
                if self.lcdc & 0x80 != val & 0x80 {
                    self.ly = 0;
                    self.counter = 0;

                    let mode = if val & 0x80 > 0 { 2 } else { 0 };
                    self.stat = (self.stat & 0xf8) | mode;
                    self.update_mode_interrupt();
                }

                self.lcdc = val;
            }
            0xff41 => self.stat = (val & 0xf8) | (self.stat & 0x3),
            0xff42 => self.scy = val,
            0xff43 => self.scx = val,
            0xff44 => (),
            0xff45 => {
                if self.lyc != val {
                    self.lyc = val;
                    self.update_lyc_interrupt();
                }
            }
            0xff47 => self.bgp = val,
            0xff48 => self.obp0 = val,
            0xff49 => self.obp1 = val,
            0xff4a => self.wy = val,
            0xff4b => self.wx = val,

            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            // VRAM
            0x8000..=0x9fff => {
                // VRAM is inaccessible during pixel transfer
                if self.stat & 0x3 != 3 {
                    self.vram[(addr & 0x1fff) as usize]
                } else {
                    0xff
                }
            }

            // OAM
            0xfe00..=0xfe9f => {
                // OAM is only accessible during H-Blank and V-Blank
                if self.stat & 0x3 == 0 || self.stat & 0x3 == 1 {
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

            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }

    fn update(&mut self, tick: u8) {
        if self.lcdc & 0x80 == 0 {
            return;
        }

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
                    self.update_mode_interrupt();
                }
            }
            // H-Blank (204 clocks)
            0 => {
                if self.counter >= 204 {
                    self.counter -= 204;
                    self.ly += 1;

                    if self.ly >= SCREEN_H {
                        // Transition to V-Blank mode
                        self.stat = (self.stat & 0xf8) | 1;
                        self.irq_vblank = true;
                    } else {
                        // Transition to OAM Search mode
                        self.stat = (self.stat & 0xf8) | 2;
                    }

                    self.update_lyc_interrupt();
                    self.update_mode_interrupt();
                }
            }
            // V-Blank (4560 clocks or 10 lines)
            1 | _ => {
                if self.counter >= 456 {
                    self.counter -= 456;
                    self.ly += 1;

                    if self.ly >= 154 {
                        // Transition to OAM Search mode
                        self.stat = (self.stat & 0xf8) | 2;
                        self.ly = 0;

                        self.update_mode_interrupt();
                    }

                    self.update_lyc_interrupt();
                }
            }
        }
    }
}
