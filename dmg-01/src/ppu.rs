// Game Boy PPU (Picture Processing Unit) implementation
// Handles LCD display, sprites, backgrounds, and timing

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;
pub const TILE_SIZE: usize = 8;
pub const TILE_MAP_SIZE: usize = 32;
pub const VRAM_SIZE: usize = 0x2000;
pub const OAM_SIZE: usize = 0xA0;

// PPU Registers (memory-mapped I/O)
pub const LCDC_ADDR: u16 = 0xFF40;  // LCD Control
pub const STAT_ADDR: u16 = 0xFF41;  // LCD Status
pub const SCY_ADDR: u16 = 0xFF42;   // Scroll Y
pub const SCX_ADDR: u16 = 0xFF43;   // Scroll X
pub const LY_ADDR: u16 = 0xFF44;    // LCD Y-Coordinate
pub const LYC_ADDR: u16 = 0xFF45;   // LY Compare
pub const DMA_ADDR: u16 = 0xFF46;   // DMA Transfer
pub const BGP_ADDR: u16 = 0xFF47;   // BG Palette Data
pub const OBP0_ADDR: u16 = 0xFF48;  // Object Palette 0
pub const OBP1_ADDR: u16 = 0xFF49;  // Object Palette 1
pub const WY_ADDR: u16 = 0xFF4A;    // Window Y Position
pub const WX_ADDR: u16 = 0xFF4B;    // Window X Position

// PPU modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PPUMode {
    HBlank = 0,      // Mode 0: Horizontal blank
    VBlank = 1,      // Mode 1: Vertical blank
    OAMSearch = 2,   // Mode 2: OAM search
    Drawing = 3,     // Mode 3: Drawing to LCD
}

// LCD Control register flags
#[derive(Debug, Clone, Copy)]
pub struct LCDControl {
    pub lcd_enable: bool,           // Bit 7: LCD Display Enable
    pub window_tile_map: bool,      // Bit 6: Window Tile Map Display Select
    pub window_enable: bool,        // Bit 5: Window Display Enable
    pub bg_window_tile_data: bool,  // Bit 4: BG & Window Tile Data Select
    pub bg_tile_map: bool,          // Bit 3: BG Tile Map Display Select
    pub sprite_size: bool,          // Bit 2: Sprite Size (0=8x8, 1=8x16)
    pub sprite_enable: bool,        // Bit 1: Sprite Display Enable
    pub bg_window_enable: bool,     // Bit 0: BG & Window Display Enable
}

// LCD Status register
#[derive(Debug, Clone, Copy)]
pub struct LCDStatus {
    pub lyc_interrupt: bool,        // Bit 6: LYC=LY Interrupt
    pub oam_interrupt: bool,        // Bit 5: Mode 2 OAM Interrupt
    pub vblank_interrupt: bool,     // Bit 4: Mode 1 V-Blank Interrupt
    pub hblank_interrupt: bool,     // Bit 3: Mode 0 H-Blank Interrupt
    pub lyc_flag: bool,             // Bit 2: LYC=LY Flag
    pub mode: PPUMode,              // Bits 1-0: Mode Flag
}

// Sprite attributes
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub y: u8,          // Y position
    pub x: u8,          // X position
    pub tile: u8,       // Tile number
    pub flags: u8,      // Attributes/flags
}

impl Sprite {
    pub fn priority(&self) -> bool {
        (self.flags & 0x80) != 0
    }

    pub fn y_flip(&self) -> bool {
        (self.flags & 0x40) != 0
    }

    pub fn x_flip(&self) -> bool {
        (self.flags & 0x20) != 0
    }

    pub fn palette(&self) -> bool {
        (self.flags & 0x10) != 0
    }
}

// Color values (2-bit grayscale)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    White = 0,
    LightGray = 1,
    DarkGray = 2,
    Black = 3,
}

impl Color {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0x3 {
            0 => Color::White,
            1 => Color::LightGray,
            2 => Color::DarkGray,
            3 => Color::Black,
            _ => unreachable!(),
        }
    }

    pub fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            Color::White => (255, 255, 255),
            Color::LightGray => (192, 192, 192),
            Color::DarkGray => (96, 96, 96),
            Color::Black => (0, 0, 0),
        }
    }
}

// Palette for color conversion
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub colors: [Color; 4],
}

impl Palette {
    pub fn new() -> Self {
        Palette {
            colors: [Color::White, Color::LightGray, Color::DarkGray, Color::Black],
        }
    }

    pub fn from_byte(byte: u8) -> Self {
        Palette {
            colors: [
                Color::from_bits(byte),
                Color::from_bits(byte >> 2),
                Color::from_bits(byte >> 4),
                Color::from_bits(byte >> 6),
            ],
        }
    }

    pub fn get_color(&self, index: u8) -> Color {
        self.colors[(index & 0x3) as usize]
    }
}

// Main PPU structure
pub struct PPU {
    // Registers
    pub lcdc: LCDControl,
    pub stat: LCDStatus,
    pub scy: u8,                    // Scroll Y
    pub scx: u8,                    // Scroll X
    pub ly: u8,                     // Current scanline
    pub lyc: u8,                    // LY Compare
    pub dma: u8,                    // DMA register
    pub bgp: Palette,               // Background palette
    pub obp0: Palette,              // Object palette 0
    pub obp1: Palette,              // Object palette 1
    pub wy: u8,                     // Window Y position
    pub wx: u8,                     // Window X position

    // Memory
    pub vram: [u8; VRAM_SIZE],      // Video RAM (0x8000-0x9FFF)
    pub oam: [u8; OAM_SIZE],        // Object Attribute Memory (0xFE00-0xFE9F)

    // Internal state
    pub mode_clock: u32,            // Clock cycles in current mode
    pub framebuffer: [Color; LCD_WIDTH * LCD_HEIGHT],
    pub bg_priority: [bool; LCD_WIDTH], // Background priority for sprites
    
    // Timing
    pub vblank_interrupt: bool,     // V-blank interrupt pending
    pub stat_interrupt: bool,       // STAT interrupt pending
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            lcdc: LCDControl {
                lcd_enable: true,
                window_tile_map: false,
                window_enable: false,
                bg_window_tile_data: true,
                bg_tile_map: false,
                sprite_size: false,
                sprite_enable: false,
                bg_window_enable: true,
            },
            stat: LCDStatus {
                lyc_interrupt: false,
                oam_interrupt: false,
                vblank_interrupt: false,
                hblank_interrupt: false,
                lyc_flag: false,
                mode: PPUMode::OAMSearch,
            },
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bgp: Palette::new(),
            obp0: Palette::new(),
            obp1: Palette::new(),
            wy: 0,
            wx: 0,
            vram: [0; VRAM_SIZE],
            oam: [0; OAM_SIZE],
            mode_clock: 0,
            framebuffer: [Color::White; LCD_WIDTH * LCD_HEIGHT],
            bg_priority: [false; LCD_WIDTH],
            vblank_interrupt: false,
            stat_interrupt: false,
        }
    }

    pub fn step(&mut self, cycles: u32) {
        if !self.lcdc.lcd_enable {
            return;
        }

        self.mode_clock += cycles;

        match self.stat.mode {
            PPUMode::OAMSearch => {
                if self.mode_clock >= 80 {
                    self.mode_clock = 0;
                    self.stat.mode = PPUMode::Drawing;
                }
            }
            PPUMode::Drawing => {
                if self.mode_clock >= 172 {
                    self.mode_clock = 0;
                    self.stat.mode = PPUMode::HBlank;
                    self.render_scanline();
                    
                    if self.stat.hblank_interrupt {
                        self.stat_interrupt = true;
                    }
                }
            }
            PPUMode::HBlank => {
                if self.mode_clock >= 204 {
                    self.mode_clock = 0;
                    self.ly += 1;
                    
                    if self.ly == 144 {
                        // Enter V-blank
                        self.stat.mode = PPUMode::VBlank;
                        self.vblank_interrupt = true;
                        
                        if self.stat.vblank_interrupt {
                            self.stat_interrupt = true;
                        }
                    } else {
                        // Next scanline
                        self.stat.mode = PPUMode::OAMSearch;
                        
                        if self.stat.oam_interrupt {
                            self.stat_interrupt = true;
                        }
                    }
                    
                    // Check LYC=LY interrupt
                    self.stat.lyc_flag = self.ly == self.lyc;
                    if self.stat.lyc_flag && self.stat.lyc_interrupt {
                        self.stat_interrupt = true;
                    }
                }
            }
            PPUMode::VBlank => {
                if self.mode_clock >= 456 {
                    self.mode_clock = 0;
                    self.ly += 1;
                    
                    if self.ly > 153 {
                        // Reset to start of frame
                        self.ly = 0;
                        self.stat.mode = PPUMode::OAMSearch;
                        
                        if self.stat.oam_interrupt {
                            self.stat_interrupt = true;
                        }
                    }
                    
                    // Check LYC=LY interrupt
                    self.stat.lyc_flag = self.ly == self.lyc;
                    if self.stat.lyc_flag && self.stat.lyc_interrupt {
                        self.stat_interrupt = true;
                    }
                }
            }
        }
    }

    fn render_scanline(&mut self) {
        if self.ly >= LCD_HEIGHT as u8 {
            return;
        }

        let scanline = self.ly as usize;
        
        // Debug: Check LCDC settings and rendering status
        static mut LCDC_DEBUG_COUNT: u32 = 0;
        unsafe {
            LCDC_DEBUG_COUNT += 1;
            if LCDC_DEBUG_COUNT == 1000 { // Debug after some scanlines
                println!("LCDC Debug: lcd_enable={}, bg_window_enable={}, sprite_enable={}, window_enable={}", 
                    self.lcdc.lcd_enable, self.lcdc.bg_window_enable, self.lcdc.sprite_enable, self.lcdc.window_enable);
                println!("PPU State: LY={}, SCX={}, SCY={}", self.ly, self.scx, self.scy);
            }
        }
        
        // Clear background priority for this scanline
        for i in 0..LCD_WIDTH {
            self.bg_priority[i] = false;
        }

        // Render background
        if self.lcdc.bg_window_enable {
            self.render_background(scanline);
        } else {
            // Debug: Background disabled
            static mut BG_DISABLED_COUNT: u32 = 0;
            unsafe {
                BG_DISABLED_COUNT += 1;
                if BG_DISABLED_COUNT == 1 {
                    println!("Background rendering is DISABLED (bg_window_enable=false)");
                }
            }
        }

        // Render window
        if self.lcdc.window_enable && self.lcdc.bg_window_enable {
            self.render_window(scanline);
        }

        // Render sprites
        if self.lcdc.sprite_enable {
            self.render_sprites(scanline);
        }
    }

    fn render_background(&mut self, scanline: usize) {
        let scroll_y = self.scy.wrapping_add(scanline as u8);
        let tile_row = (scroll_y / 8) as usize;
        let tile_y = (scroll_y % 8) as usize;

        // Debug: Check if VRAM has any non-zero data
        static mut DEBUG_COUNT: u32 = 0;
        unsafe {
            DEBUG_COUNT += 1;
            if DEBUG_COUNT == 2000 { // Debug after Pokemon Red starts
                let non_zero_vram = self.vram.iter().filter(|&&b| b != 0).count();
                println!("Pokemon Red VRAM: {} non-zero bytes", non_zero_vram);
                if non_zero_vram > 0 {
                    println!("VRAM 0x1000-0x1020 (signed tile area): {:02X?}", &self.vram[0x1000..0x1020]);
                }
            }
        }

        // Debug first pixel of each scanline to understand what's happening
        static mut SCANLINE_DEBUG_COUNT: u32 = 0;
        unsafe {
            SCANLINE_DEBUG_COUNT += 1;
            if SCANLINE_DEBUG_COUNT <= 5 { // Debug first 5 scanlines only
                let scroll_x = self.scx;
                let tile_col = (scroll_x / 8) as usize;
                let tile_x = (scroll_x % 8) as usize;
                let tile_map_base = if self.lcdc.bg_tile_map { 0x1C00 } else { 0x1800 };
                let tile_map_addr = tile_map_base + (tile_row % 32) * 32 + (tile_col % 32);
                let tile_id = self.vram[tile_map_addr];
                
                println!("Scanline {}: scroll=({},{}), tile_row={}, tile_col={}, tile_map_addr=0x{:04X}, tile_id=0x{:02X}", 
                    scanline, self.scx, self.scy, tile_row, tile_col, tile_map_addr, tile_id);
                println!("  LCDC: bg_tile_map={}, bg_window_tile_data={}", self.lcdc.bg_tile_map, self.lcdc.bg_window_tile_data);
            }
        }

        for pixel in 0..LCD_WIDTH {
            let scroll_x = self.scx.wrapping_add(pixel as u8);
            let tile_col = (scroll_x / 8) as usize;
            let tile_x = (scroll_x % 8) as usize;

            // Get tile map address
            let tile_map_base = if self.lcdc.bg_tile_map { 0x1C00 } else { 0x1800 };
            let tile_map_addr = tile_map_base + (tile_row % 32) * 32 + (tile_col % 32);
            let tile_id = self.vram[tile_map_addr];

            // Get tile data
            let color = self.get_tile_pixel(tile_id, tile_x, tile_y);
            let final_color = self.bgp.get_color(color);

            // Debug: Check first few pixels to see what colors are being set
            static mut PIXEL_DEBUG_COUNT: u32 = 0;
            unsafe {
                PIXEL_DEBUG_COUNT += 1;
                if PIXEL_DEBUG_COUNT <= 5 {
                    println!("Pixel #{}: raw_color={}, final_color={:?}", PIXEL_DEBUG_COUNT, color, final_color);
                }
            }

            // Set pixel in framebuffer
            self.framebuffer[scanline * LCD_WIDTH + pixel] = final_color;
            
            // Set background priority
            self.bg_priority[pixel] = color != 0;
        }
    }

    fn render_window(&mut self, scanline: usize) {
        if self.wy > scanline as u8 {
            return;
        }

        let window_y = (scanline as u8).wrapping_sub(self.wy);
        let tile_row = (window_y / 8) as usize;
        let tile_y = (window_y % 8) as usize;

        for pixel in 0..LCD_WIDTH {
            let window_x = (pixel as u8).wrapping_sub(self.wx.wrapping_sub(7));
            if window_x >= LCD_WIDTH as u8 {
                continue;
            }

            let tile_col = (window_x / 8) as usize;
            let tile_x = (window_x % 8) as usize;

            // Get tile map address
            let tile_map_base = if self.lcdc.window_tile_map { 0x1C00 } else { 0x1800 };
            let tile_map_addr = tile_map_base + (tile_row % 32) * 32 + (tile_col % 32);
            let tile_id = self.vram[tile_map_addr];

            // Get tile data
            let color = self.get_tile_pixel(tile_id, tile_x, tile_y);
            let final_color = self.bgp.get_color(color);

            // Set pixel in framebuffer
            self.framebuffer[scanline * LCD_WIDTH + pixel] = final_color;
            
            // Set background priority
            self.bg_priority[pixel] = color != 0;
        }
    }

    fn render_sprites(&mut self, scanline: usize) {
        let sprite_height = if self.lcdc.sprite_size { 16 } else { 8 };
        let mut sprites_on_line = Vec::new();

        // Find sprites on this scanline
        for sprite_index in 0..40 {
            let sprite_addr = sprite_index * 4;
            let sprite_y = self.oam[sprite_addr];
            let sprite_x = self.oam[sprite_addr + 1];
            let tile_id = self.oam[sprite_addr + 2];
            let flags = self.oam[sprite_addr + 3];

            // Check if sprite is on this scanline
            let sprite_top = sprite_y.wrapping_sub(16);
            let sprite_bottom = sprite_top + sprite_height;
            
            if scanline as u8 >= sprite_top && (scanline as u8) < sprite_bottom {
                sprites_on_line.push(Sprite {
                    y: sprite_y,
                    x: sprite_x,
                    tile: tile_id,
                    flags,
                });
            }
        }

        // Sort sprites by X position (priority)
        sprites_on_line.sort_by_key(|s| s.x);

        // Render sprites (max 10 per scanline)
        for sprite in sprites_on_line.iter().take(10) {
            self.render_sprite(sprite, scanline, sprite_height);
        }
    }

    fn render_sprite(&mut self, sprite: &Sprite, scanline: usize, sprite_height: u8) {
        let sprite_y = sprite.y.wrapping_sub(16);
        let sprite_x = sprite.x.wrapping_sub(8);
        
        if sprite_x >= LCD_WIDTH as u8 {
            return;
        }

        let line_in_sprite = if sprite.y_flip() {
            sprite_height - 1 - ((scanline as u8).wrapping_sub(sprite_y))
        } else {
            (scanline as u8).wrapping_sub(sprite_y)
        };

        let tile_id = if sprite_height == 16 {
            sprite.tile & 0xFE
        } else {
            sprite.tile
        };

        let tile_y = (line_in_sprite % 8) as usize;
        let tile_id = if sprite_height == 16 && line_in_sprite >= 8 {
            tile_id + 1
        } else {
            tile_id
        };

        for pixel_x in 0..8 {
            let screen_x = sprite_x.wrapping_add(pixel_x);
            if screen_x >= LCD_WIDTH as u8 {
                break;
            }

            let tile_x = if sprite.x_flip() {
                7 - pixel_x
            } else {
                pixel_x
            } as usize;

            let color = self.get_tile_pixel(tile_id, tile_x, tile_y);
            if color == 0 {
                continue; // Transparent pixel
            }

            // Check sprite priority
            if sprite.priority() && self.bg_priority[screen_x as usize] {
                continue; // Background has priority
            }

            // Get sprite palette
            let palette = if sprite.palette() { self.obp1 } else { self.obp0 };
            let final_color = palette.get_color(color);

            // Set pixel in framebuffer
            self.framebuffer[scanline * LCD_WIDTH + screen_x as usize] = final_color;
        }
    }

    fn get_tile_pixel(&self, tile_id: u8, x: usize, y: usize) -> u8 {
        let tile_data_base = if self.lcdc.bg_window_tile_data {
            0x0000
        } else {
            0x1000
        };

        let tile_addr = if self.lcdc.bg_window_tile_data {
            // Unsigned mode: tiles 0-255 at 0x8000-0x8FF0
            tile_data_base + (tile_id as usize) * 16
        } else {
            // Signed mode: tiles -128 to 127 at 0x8800 + (signed_id * 16)
            // Base 0x8800 corresponds to tile index 0 in signed mode
            let signed_tile_id = tile_id as i8 as i16;
            let signed_addr = tile_data_base as i16 + (signed_tile_id * 16);
            signed_addr as usize
        };

        let byte_offset = y * 2;
        let low_byte = self.vram[tile_addr + byte_offset];
        let high_byte = self.vram[tile_addr + byte_offset + 1];

        let bit_pos = 7 - x;
        let low_bit = (low_byte >> bit_pos) & 1;
        let high_bit = (high_byte >> bit_pos) & 1;

        let color = (high_bit << 1) | low_bit;
        
        // Debug tile data access for Pokemon Red
        static mut TILE_DEBUG_COUNT: u32 = 0;
        unsafe {
            TILE_DEBUG_COUNT += 1;
            if TILE_DEBUG_COUNT <= 10 { // Debug first 10 tile accesses
                let signed_id = tile_id as i8;
                println!("Tile Debug #{}: tile_id=0x{:02X} (signed={}), mode={}, base=0x{:04X}, addr=0x{:04X}, low=0x{:02X}, high=0x{:02X}, color={}", 
                    TILE_DEBUG_COUNT, tile_id, signed_id, self.lcdc.bg_window_tile_data, tile_data_base, tile_addr, low_byte, high_byte, color);
                if color != 0 {
                    println!("*** NON-ZERO COLOR FOUND! *** Checking VRAM around 0x{:04X}", tile_addr);
                    for i in 0..16 {
                        print!("{:02X} ", self.vram[tile_addr + i]);
                    }
                    println!();
                }
                // Show what tile IDs Pokemon Red is actually using
                println!("Pokemon Red using tile ID: 0x{:02X} at screen position - signed addressing = {}", tile_id, !self.lcdc.bg_window_tile_data);
            }
        }
        
        color
    }

    // Memory-mapped I/O read
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            LCDC_ADDR => self.lcdc_to_byte(),
            STAT_ADDR => self.stat_to_byte(),
            SCY_ADDR => self.scy,
            SCX_ADDR => self.scx,
            LY_ADDR => self.ly,
            LYC_ADDR => self.lyc,
            DMA_ADDR => self.dma,
            BGP_ADDR => self.palette_to_byte(&self.bgp),
            OBP0_ADDR => self.palette_to_byte(&self.obp0),
            OBP1_ADDR => self.palette_to_byte(&self.obp1),
            WY_ADDR => self.wy,
            WX_ADDR => self.wx,
            _ => 0xFF,
        }
    }

    // Memory-mapped I/O write
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            LCDC_ADDR => {
                self.lcdc = self.byte_to_lcdc(value);
                if value != 0 {
                    println!("LCDC set to: 0x{:02X} - LCD:{} BG:{} Sprites:{} BG_Map:{} Tile_Data:{}", 
                             value, self.lcdc.lcd_enable, self.lcdc.bg_window_enable, 
                             self.lcdc.sprite_enable, self.lcdc.bg_tile_map, self.lcdc.bg_window_tile_data);
                }
            },
            STAT_ADDR => self.stat = self.byte_to_stat(value),
            SCY_ADDR => {
                self.scy = value;
                if value != 0 {
                    println!("SCY set to: {}", value);
                }
            },
            SCX_ADDR => {
                self.scx = value;
                if value != 0 {
                    println!("SCX set to: {}", value);
                }
            },
            LY_ADDR => {}, // Read-only
            LYC_ADDR => self.lyc = value,
            DMA_ADDR => self.dma = value,
            BGP_ADDR => {
                self.bgp = Palette::from_byte(value);
                if value != 0 {
                    println!("BGP set to: 0x{:02X} - Color mapping: 0={:?}, 1={:?}, 2={:?}, 3={:?}", 
                        value, self.bgp.colors[0], self.bgp.colors[1], self.bgp.colors[2], self.bgp.colors[3]);
                }
            },
            OBP0_ADDR => self.obp0 = Palette::from_byte(value),
            OBP1_ADDR => self.obp1 = Palette::from_byte(value),
            WY_ADDR => self.wy = value,
            WX_ADDR => self.wx = value,
            _ => {},
        }
    }

    // Helper functions for register conversion
    fn lcdc_to_byte(&self) -> u8 {
        let mut byte = 0;
        if self.lcdc.lcd_enable { byte |= 0x80; }
        if self.lcdc.window_tile_map { byte |= 0x40; }
        if self.lcdc.window_enable { byte |= 0x20; }
        if self.lcdc.bg_window_tile_data { byte |= 0x10; }
        if self.lcdc.bg_tile_map { byte |= 0x08; }
        if self.lcdc.sprite_size { byte |= 0x04; }
        if self.lcdc.sprite_enable { byte |= 0x02; }
        if self.lcdc.bg_window_enable { byte |= 0x01; }
        byte
    }

    fn byte_to_lcdc(&self, byte: u8) -> LCDControl {
        LCDControl {
            lcd_enable: (byte & 0x80) != 0,
            window_tile_map: (byte & 0x40) != 0,
            window_enable: (byte & 0x20) != 0,
            bg_window_tile_data: (byte & 0x10) != 0,
            bg_tile_map: (byte & 0x08) != 0,
            sprite_size: (byte & 0x04) != 0,
            sprite_enable: (byte & 0x02) != 0,
            bg_window_enable: (byte & 0x01) != 0,
        }
    }

    fn stat_to_byte(&self) -> u8 {
        let mut byte = 0;
        if self.stat.lyc_interrupt { byte |= 0x40; }
        if self.stat.oam_interrupt { byte |= 0x20; }
        if self.stat.vblank_interrupt { byte |= 0x10; }
        if self.stat.hblank_interrupt { byte |= 0x08; }
        if self.stat.lyc_flag { byte |= 0x04; }
        byte |= self.stat.mode as u8;
        byte
    }

    fn byte_to_stat(&self, byte: u8) -> LCDStatus {
        LCDStatus {
            lyc_interrupt: (byte & 0x40) != 0,
            oam_interrupt: (byte & 0x20) != 0,
            vblank_interrupt: (byte & 0x10) != 0,
            hblank_interrupt: (byte & 0x08) != 0,
            lyc_flag: self.stat.lyc_flag, // Read-only
            mode: self.stat.mode, // Read-only
        }
    }

    fn palette_to_byte(&self, palette: &Palette) -> u8 {
        let mut byte = 0;
        for (i, color) in palette.colors.iter().enumerate() {
            byte |= (*color as u8) << (i * 2);
        }
        byte
    }

    pub fn get_framebuffer(&self) -> &[Color; LCD_WIDTH * LCD_HEIGHT] {
        &self.framebuffer
    }

    pub fn clear_interrupts(&mut self) {
        self.vblank_interrupt = false;
        self.stat_interrupt = false;
    }
}