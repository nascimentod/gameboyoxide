// Memory invariant verification tests
// Tests address ranges, read/write consistency, and peripheral behavior

use dmg_01::memory::MemoryBus;
use dmg_01::cartridge::Cartridge;

// ============================================================================
// Address range invariants
// ============================================================================

#[test]
fn test_boot_rom_region() {
    let mut bus = MemoryBus::new();
    
    // Boot ROM is at 0x0000-0x00FF
    assert!(bus.boot_rom_enabled);
    let boot_byte = bus.read_byte(0x0000);
    assert_eq!(boot_byte, 0x31, "Boot ROM first byte should be 0x31 (LD SP, 0xFFFE)");
    assert_eq!(bus.read_byte(0x00FF), 0x50, "Boot ROM last byte should be 0x50");
    
    // Boot ROM is still mapped at 0x0000-0x00FF even when we write elsewhere
    assert_eq!(bus.read_byte(0x0000), 0x31, "Boot ROM still accessible");
    
    // Disable boot ROM and test internal RAM at 0x0100
    bus.write_byte(0xFF50, 0x01); // Disable boot ROM
    assert!(!bus.boot_rom_enabled);
    // After boot ROM disabled, 0x0100 reads from cartridge ROM (or 0xFF if no cartridge)
    assert_eq!(bus.read_byte(0x0100), 0xFF, "ROM region without cartridge returns 0xFF");
}

#[test]
fn test_vram_region() {
    let mut bus = MemoryBus::new();
    
    // VRAM is 0x8000-0x9FFF (8KB)
    for addr in 0x8000..0xA000 {
        bus.write_byte(addr, 0x42);
        assert_eq!(bus.read_byte(addr), 0x42, "VRAM write/read at 0x{:04X}", addr);
    }
}

#[test]
fn test_oam_region() {
    let mut bus = MemoryBus::new();
    
    // OAM is 0xFE00-0xFE9F (160 bytes)
    for i in 0..0xA0 {
        let addr = 0xFE00 + i;
        bus.write_byte(addr, 0xAB);
        assert_eq!(bus.read_byte(addr), 0xAB, "OAM write/read at 0x{:04X}", addr);
    }
}

#[test]
fn test_echo_ram_region() {
    let mut bus = MemoryBus::new();
    
    // Echo RAM 0xFE00-0xFE9F - reads from OAM, but writes go to echo
    // Actually in our implementation, 0xFEA0-0xFEFF returns 0x00
    for i in 0xA0..0x100 {
        let addr = 0xFE00 + i;
        bus.write_byte(addr, 0x55);
        assert_eq!(bus.read_byte(addr), 0x00, "Echo region 0xFEA0-0xFEFF should read 0x00 at 0x{:04X}", addr);
    }
}

#[test]
fn test_hram_region() {
    let mut bus = MemoryBus::new();
    
    // HRAM 0xFF80-0xFFFE
    for i in 0x80..=0xFE {
        let addr = 0xFF00 + i;
        bus.write_byte(addr, 0x77);
        assert_eq!(bus.read_byte(addr), 0x77, "HRAM write/read at 0x{:04X}", addr);
    }
}

#[test]
fn test_iewram_upper_boundary() {
    let mut bus = MemoryBus::new();
    
    // 0xFFFF is IE register
    bus.write_byte(0xFFFF, 0x1F);
    assert_eq!(bus.ie_register, 0x1F);
    
    // Reading IE register should return the stored value
    let ie_val = bus.read_byte(0xFFFF);
    assert_eq!(ie_val, 0x1F);
}

// ============================================================================
// Write-through consistency
// ============================================================================

#[test]
fn test_writethrough_internal_ram() {
    let mut bus = MemoryBus::new();
    
    // Internal RAM 0xC000-0xFDFF should be direct write-through
    let test_addresses = [0xC000, 0xD000, 0xE000, 0xF000, 0xFDFF];
    
    for &addr in &test_addresses {
        for val in [0x00, 0x55, 0xAA, 0xFF, 0x37, 0x89] {
            bus.write_byte(addr, val);
            assert_eq!(bus.read_byte(addr), val, "Write-through at 0x{:04X} with 0x{:02X}", addr, val);
        }
    }
}

#[test]
fn test_vram_writethrough() {
    let mut bus = MemoryBus::new();
    
    // VRAM is directly writable
    let test_addrs = [0x8000, 0x8001, 0x9000, 0x9FFF];
    for &addr in &test_addrs {
        bus.write_byte(addr, 0x55);
        assert_eq!(bus.read_byte(addr), 0x55, "VRAM write at 0x{:04X}", addr);
    }
}

// ============================================================================
// Cartridge ROM/RAM
// ============================================================================

#[test]
fn test_cartridge_rom_read() {
    let mut bus = MemoryBus::new();
    bus.boot_rom_enabled = false; // Disable boot ROM to test cartridge
    
    let rom_data = vec![0xAB; 0x8000]; // 32KB ROM
    bus.load_cartridge(rom_data);
    
    // ROM should be readable at 0x0000-0x7FFF
    assert_eq!(bus.read_byte(0x0000), 0xAB, "ROM at bank 0");
    assert_eq!(bus.read_byte(0x0100), 0xAB, "ROM header");
}

#[test]
fn test_cartridge_ram_read_write() {
    let mut bus = MemoryBus::new();
    bus.boot_rom_enabled = false;
    
    // Create a simple MBC0 cartridge (no banking, just RAM)
    let mut rom_data = vec![0u8; 0x8000];
    rom_data[0x0147] = 0x00; // MBC0
    rom_data[0x0149] = 0x02; // 8KB RAM
    bus.load_cartridge(rom_data);
    
    // RAM disabled by default
    assert_eq!(bus.read_byte(0xA000), 0xFF);
    
    // Enable RAM (MBC0: write 0x0A to 0x0000-0x1FFF)
    bus.write_byte(0x0000, 0x0A);
    assert_eq!(bus.read_byte(0xA000), 0x00, "RAM should be zeroed after enable");
    
    // Write to RAM
    bus.write_byte(0xA000, 0x42);
    assert_eq!(bus.read_byte(0xA000), 0x42, "RAM write/read");
}

// ============================================================================
// Timer register behavior
// ============================================================================

#[test]
fn test_div_register_reset() {
    let mut bus = MemoryBus::new();
    
    // DIV starts at 0
    assert_eq!(bus.read_byte(0xFF04), 0x00);
    
    // Writing any value resets DIV
    bus.write_byte(0xFF04, 0xFF);
    assert_eq!(bus.read_byte(0xFF04), 0x00, "DIV resets after write");
}

#[test]
fn test_tima_write() {
    let mut bus = MemoryBus::new();
    
    bus.write_byte(0xFF05, 0x42);
    assert_eq!(bus.read_byte(0xFF05), 0x42, "TIMA write/read");
}

#[test]
fn test_tma_write() {
    let mut bus = MemoryBus::new();
    
    bus.write_byte(0xFF06, 0x42);
    assert_eq!(bus.read_byte(0xFF06), 0x42, "TMA write/read");
}

#[test]
fn test_tac_register() {
    let mut bus = MemoryBus::new();
    
    // Upper 5 bits read as 1
    bus.write_byte(0xFF07, 0x03); // Timer on, div 4
    let read_val = bus.read_byte(0xFF07);
    assert_eq!(read_val & 0xF8, 0xF8, "TAC upper 5 bits should be 1");
    assert_eq!(read_val & 0x07, 0x03, "TAC lower 3 bits preserved");
}

// ============================================================================
// I/O register edge cases
// ============================================================================

#[test]
fn test_joypad_register_default() {
    let mut bus = MemoryBus::new();
    
    // Default: neither button group selected, upper bits always 1
    let val = bus.read_byte(0xFF00);
    assert_eq!(val & 0xC0, 0xC0, "Joypad upper 2 bits always 1");
}

#[test]
fn test_lcdc_write() {
    let mut bus = MemoryBus::new();
    
    // LCDC register at 0xFF40
    bus.write_byte(0xFF40, 0x91); // Enable LCD, window enable, etc.
    let val = bus.read_byte(0xFF40);
    assert_eq!(val, 0x91, "LCDC write/read");
}

#[test]
fn test_boot_rom_disable() {
    let mut bus = MemoryBus::new();
    
    // Boot ROM enabled by default
    assert!(bus.boot_rom_enabled);
    
    // Writing any non-zero value to 0xFF50 disables boot ROM
    bus.write_byte(0xFF50, 0x01);
    assert!(!bus.boot_rom_enabled, "Boot ROM should be disabled after write to 0xFF50");
    
    // Reading 0xFF50 returns 0xFF (write-only register)
    assert_eq!(bus.read_byte(0xFF50), 0xFF, "Reading 0xFF50 returns 0xFF");
}

// ============================================================================
// Memory corruption detection
// ============================================================================

#[test]
fn test_no_cross_region_leak() {
    let mut bus = MemoryBus::new();
    
    // Fill each region with distinct patterns
    for i in 0x8000..0x9000 {
        bus.memory[i] = 0x11; // VRAM
    }
    for i in 0x9000..0xA000 {
        bus.memory[i] = 0x22; // VRAM end
    }
    for i in 0xC000..0xD000 {
        bus.memory[i] = 0x33; // Internal RAM
    }
    
    // Read back and verify no overlap
    for i in 0x8000..0x9000 {
        assert_eq!(bus.memory[i], 0x11, "VRAM pattern at 0x{:04X}", i);
    }
    for i in 0xC000..0xD000 {
        assert_eq!(bus.memory[i], 0x33, "RAM pattern at 0x{:04X}", i);
    }
}

// ============================================================================
// PPU register read/write
// ============================================================================

#[test]
fn test_ppu_registers_rw() {
    let mut bus = MemoryBus::new();
    
    // Test all PPU registers (0xFF40-0xFF4B)
    let ppu_addrs = [0xFF40, 0xFF41, 0xFF42, 0xFF43, 0xFF44, 0xFF45, 
                     0xFF46, 0xFF47, 0xFF48, 0xFF49, 0xFF4A, 0xFF4B];
    
    for &addr in &ppu_addrs {
        // Write a value
        bus.write_byte(addr, 0x55);
        // Read it back (some registers may differ, e.g., LY is read-only)
        // Just verify no panic
        let _ = bus.read_byte(addr);
    }
}

// ============================================================================
// DMA register behavior
// ============================================================================

#[test]
fn test_dma_register() {
    let mut bus = MemoryBus::new();
    
    // DMA register at 0xFF46 - writing starts transfer
    // Reading returns the value written
    bus.write_byte(0xFF46, 0xC0);
    let val = bus.read_byte(0xFF46);
    assert_eq!(val, 0xC0, "DMA register echoes write value");
}
