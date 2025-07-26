use dmg_01::ppu::{PPU, Color, PPUMode, Palette};

#[test]
fn test_ppu_initialization() {
    let ppu = PPU::new();
    
    assert_eq!(ppu.ly, 0);
    assert_eq!(ppu.stat.mode, PPUMode::OAMSearch);
    assert_eq!(ppu.lcdc.lcd_enable, true);
    assert_eq!(ppu.lcdc.bg_window_enable, true);
}

#[test]
fn test_ppu_mode_transitions() {
    let mut ppu = PPU::new();
    
    // Start in OAM search mode
    assert_eq!(ppu.stat.mode, PPUMode::OAMSearch);
    
    // Step through OAM search (80 cycles)
    ppu.step(80);
    assert_eq!(ppu.stat.mode, PPUMode::Drawing);
    
    // Step through drawing (172 cycles)
    ppu.step(172);
    assert_eq!(ppu.stat.mode, PPUMode::HBlank);
    
    // Step through H-blank (204 cycles)
    ppu.step(204);
    assert_eq!(ppu.stat.mode, PPUMode::OAMSearch);
    assert_eq!(ppu.ly, 1);
}

#[test]
fn test_ppu_vblank_timing() {
    let mut ppu = PPU::new();
    
    // Simulate full frame to V-blank
    for scanline in 0..144 {
        ppu.ly = scanline;
        ppu.step(80);  // OAM search
        ppu.step(172); // Drawing
        ppu.step(204); // H-blank
    }
    
    assert_eq!(ppu.stat.mode, PPUMode::VBlank);
    assert_eq!(ppu.ly, 144);
    assert!(ppu.vblank_interrupt);
}

#[test]
fn test_palette_conversion() {
    let palette = Palette::from_byte(0b11100100);
    
    assert_eq!(palette.colors[0], Color::White);
    assert_eq!(palette.colors[1], Color::LightGray);
    assert_eq!(palette.colors[2], Color::DarkGray);
    assert_eq!(palette.colors[3], Color::Black);
}

#[test]
fn test_color_to_rgb() {
    assert_eq!(Color::White.to_rgb(), (255, 255, 255));
    assert_eq!(Color::LightGray.to_rgb(), (192, 192, 192));
    assert_eq!(Color::DarkGray.to_rgb(), (96, 96, 96));
    assert_eq!(Color::Black.to_rgb(), (0, 0, 0));
}

#[test]
fn test_ppu_register_access() {
    let mut ppu = PPU::new();
    
    // Test LCDC register
    ppu.write_register(0xFF40, 0x91);
    assert_eq!(ppu.read_register(0xFF40), 0x91);
    
    // Test scroll registers
    ppu.write_register(0xFF42, 0x10); // SCY
    ppu.write_register(0xFF43, 0x20); // SCX
    assert_eq!(ppu.read_register(0xFF42), 0x10);
    assert_eq!(ppu.read_register(0xFF43), 0x20);
    
    // Test LY register (read-only)
    ppu.write_register(0xFF44, 0x50);
    assert_eq!(ppu.read_register(0xFF44), 0x00); // Should remain 0
}

#[test]
fn test_framebuffer_initialization() {
    let ppu = PPU::new();
    let framebuffer = ppu.get_framebuffer();
    
    // All pixels should be white initially
    for &pixel in framebuffer.iter() {
        assert_eq!(pixel, Color::White);
    }
}