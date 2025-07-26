use dmg_01::cartridge::Cartridge;

#[test]
fn test_mbc1_basic_rom_banking() {
    // Create a mock ROM with MBC1 type (0x01)
    let mut rom = vec![0; 0x100000]; // 1MB ROM (64 banks)
    
    // Set cartridge type to MBC1 at 0x0147
    rom[0x0147] = 0x01;
    
    // Set ROM size to 1MB (code 0x06) at 0x0148  
    rom[0x0148] = 0x06;
    
    // Set RAM size to 8KB (code 0x02) at 0x0149
    rom[0x0149] = 0x02;
    
    // Fill different banks with unique data
    for bank in 0..64 {
        let offset = bank * 0x4000;
        if offset < rom.len() {
            rom[offset] = bank as u8; // First byte of each bank = bank number
        }
    }
    
    let mut cartridge = Cartridge::new(rom);
    
    // Test initial state - should be in ROM banking mode, bank 1
    assert_eq!(cartridge.current_rom_bank, 1);
    assert_eq!(cartridge.mbc1_banking_mode, false);
    assert_eq!(cartridge.mbc1_rom_bank_low, 1);
    assert_eq!(cartridge.mbc1_rom_bank_high, 0);
    
    // Test bank 0 read (should always be bank 0 in ROM mode)
    assert_eq!(cartridge.read_rom(0x0000), 0);
    
    // Test bank 1 read  
    assert_eq!(cartridge.read_rom(0x4000), 1);
    
    // Test writing to bank register (0x2000-0x3FFF)
    cartridge.write_rom(0x2001, 5); // Select bank 5
    assert_eq!(cartridge.current_rom_bank, 5);
    assert_eq!(cartridge.read_rom(0x4000), 5);
    
    // Test bank 0 write (should map to bank 1)
    cartridge.write_rom(0x2002, 0); // Try to select bank 0
    assert_eq!(cartridge.current_rom_bank, 1); // Should be 1, not 0
    assert_eq!(cartridge.read_rom(0x4000), 1);
    
    // Test high bit banking (0x4000-0x5FFF)
    cartridge.write_rom(0x2003, 0x1F); // Set low 5 bits to max
    cartridge.write_rom(0x4001, 0x01); // Set high 2 bits
    
    // In ROM banking mode, this should give us bank (1 << 5) + 31 = 32 + 31 = 63
    assert_eq!(cartridge.current_rom_bank, 63);
    assert_eq!(cartridge.read_rom(0x4000), 63);
}

#[test]  
fn test_mbc1_banking_mode_switching() {
    let mut rom = vec![0; 0x100000]; // 1MB ROM
    rom[0x0147] = 0x02; // MBC1+RAM
    rom[0x0148] = 0x06; // 1MB ROM
    rom[0x0149] = 0x02; // 8KB RAM
    
    // Fill banks with unique data
    for bank in 0..64 {
        let offset = bank * 0x4000;
        if offset < rom.len() {
            rom[offset] = bank as u8;
        }
    }
    
    let mut cartridge = Cartridge::new(rom);
    
    // Set up some banking state
    cartridge.write_rom(0x2000, 0x05); // ROM bank low = 5
    cartridge.write_rom(0x4000, 0x02); // High bits = 2
    
    // Should be in ROM banking mode initially
    assert_eq!(cartridge.mbc1_banking_mode, false);
    assert_eq!(cartridge.current_rom_bank, (5 + (2 << 5)) & 63); // 5 + 64 = 69, masked to 5 for 1MB ROM
    assert_eq!(cartridge.current_ram_bank, 0);
    
    // Switch to RAM banking mode
    cartridge.write_rom(0x6000, 0x01);
    assert_eq!(cartridge.mbc1_banking_mode, true);
    assert_eq!(cartridge.current_rom_bank, 5); // Should lose high bits
    assert_eq!(cartridge.current_ram_bank, 2); // Should use high bits for RAM
    
    // Bank 0 should now be affected by high bits in RAM mode  
    // Bank 64 (2 << 5) doesn't exist in 1MB ROM, so it wraps to bank 0
    assert_eq!(cartridge.read_rom(0x0000), 0); // Should read from bank 0 (64 & 63 = 0)
    
    // Switch back to ROM banking mode
    cartridge.write_rom(0x6000, 0x00);
    assert_eq!(cartridge.mbc1_banking_mode, false);
    assert_eq!(cartridge.current_ram_bank, 0); // RAM bank reset
    assert_eq!(cartridge.read_rom(0x0000), 0); // Bank 0 normal again
}

#[test]
fn test_mbc1_ram_enable_disable() {
    let mut rom = vec![0; 0x8000]; // 32KB ROM
    rom[0x0147] = 0x02; // MBC1+RAM
    rom[0x0149] = 0x02; // 8KB RAM
    
    let mut cartridge = Cartridge::new(rom);
    
    // RAM should be disabled initially
    assert_eq!(cartridge.ram_enabled, false);
    
    // Reading/writing should return 0xFF/be ignored when disabled
    assert_eq!(cartridge.read_ram(0x0000), 0xFF);
    cartridge.write_ram(0x0000, 0x42);
    assert_eq!(cartridge.read_ram(0x0000), 0xFF);
    
    // Enable RAM
    cartridge.write_rom(0x0000, 0x0A);
    assert_eq!(cartridge.ram_enabled, true);
    
    // Now RAM should work
    cartridge.write_ram(0x0000, 0x42);
    assert_eq!(cartridge.read_ram(0x0000), 0x42);
    
    // Disable RAM  
    cartridge.write_rom(0x1000, 0x00);
    assert_eq!(cartridge.ram_enabled, false);
    assert_eq!(cartridge.read_ram(0x0000), 0xFF);
}

#[test]
fn test_mbc1_large_rom_masking() {
    // Test with 2MB ROM (128 banks) - maximum for MBC1
    let mut rom = vec![0; 0x200000]; 
    rom[0x0147] = 0x03; // MBC1+RAM+BATTERY
    rom[0x0148] = 0x07; // 2MB ROM
    
    for bank in 0..128 {
        let offset = bank * 0x4000;
        if offset < rom.len() {
            rom[offset] = bank as u8;
        }
    }
    
    let mut cartridge = Cartridge::new(rom);
    
    // Test maximum bank selection
    cartridge.write_rom(0x2000, 0x1F); // Low 5 bits = 31
    cartridge.write_rom(0x4000, 0x03); // High 2 bits = 3
    
    // Should give us bank 31 + (3 << 5) = 31 + 96 = 127
    assert_eq!(cartridge.current_rom_bank, 127);
    assert_eq!(cartridge.read_rom(0x4000), 127);
}