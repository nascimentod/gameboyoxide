pub struct Cartridge {
    pub rom: Vec<u8>,
    pub rom_size: usize,
    pub ram: Vec<u8>,
    pub ram_size: usize,
    pub mbc_type: u8,
    pub current_rom_bank: usize,
    pub current_ram_bank: usize,
    pub ram_enabled: bool,
    
    // MBC1-specific fields
    pub mbc1_rom_bank_low: u8,    // 5-bit ROM bank (bits 0-4)
    pub mbc1_rom_bank_high: u8,   // 2-bit ROM bank (bits 5-6) or RAM bank
    pub mbc1_banking_mode: bool,  // false = ROM banking mode, true = RAM banking mode
    pub rom_bank_mask: usize,     // Mask for ROM bank based on ROM size
}

impl Cartridge {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let rom_size = rom_data.len();
        let mbc_type = if rom_size >= 0x8000 {
            rom_data[0x0147]
        } else {
            0
        };
        let ram_size = if rom_size > 0x0149 {
            match rom_data[0x0149] {
                0x00 => 0,
                0x01 => 0x800,   // 2KB
                0x02 => 0x2000,  // 8KB
                0x03 => 0x8000,  // 32KB
                0x04 => 0x20000, // 128KB
                0x05 => 0x10000, // 64KB
                _ => 0x2000,
            }
        } else {
            0
        };

        // Calculate ROM bank mask based on ROM size
        let rom_bank_mask = match rom_size {
            0x8000 => 1,           // 32KB = 2 banks (bank 0 + 1)
            0x10000 => 3,          // 64KB = 4 banks
            0x20000 => 7,          // 128KB = 8 banks  
            0x40000 => 15,         // 256KB = 16 banks
            0x80000 => 31,         // 512KB = 32 banks
            0x100000 => 63,        // 1MB = 64 banks
            0x200000 => 127,       // 2MB = 128 banks (max for MBC1)
            _ => 127,              // Default to max
        };

        Cartridge {
            rom: rom_data,
            rom_size,
            ram: vec![0; ram_size],
            ram_size,
            mbc_type,
            current_rom_bank: 1,
            current_ram_bank: 0,
            ram_enabled: false,
            mbc1_rom_bank_low: 1,
            mbc1_rom_bank_high: 0,
            mbc1_banking_mode: false,
            rom_bank_mask,
        }
    }

    /// Battery-backed cartridge types supported by the current MBC1/MBC3
    /// implementation. RTC-only cartridges have no RAM image to persist.
    pub fn has_battery(&self) -> bool {
        matches!(self.mbc_type, 0x03 | 0x0f | 0x10 | 0x13)
    }

    pub fn read_rom(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                // Bank 0 area - may be affected by banking mode in MBC1
                let bank = if self.mbc_type >= 0x01 && self.mbc_type <= 0x03 {
                    // MBC1: In RAM banking mode, high bits can affect bank 0
                    if self.mbc1_banking_mode {
                        ((self.mbc1_rom_bank_high as usize) << 5) & self.rom_bank_mask
                    } else {
                        0
                    }
                } else {
                    0
                };
                
                let rom_address = (bank * 0x4000) + (address as usize);
                if rom_address < self.rom.len() {
                    self.rom[rom_address]
                } else {
                    0xFF
                }
            }
            0x4000..=0x7FFF => {
                let bank_offset = self.current_rom_bank * 0x4000;
                let rom_address = bank_offset + ((address - 0x4000) as usize);
                if rom_address < self.rom.len() {
                    self.rom[rom_address]
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    pub fn write_rom(&mut self, address: u16, value: u8) {
        match self.mbc_type {
            0x00 => {
                // MBC0 - No banking, just RAM enable/disable
                if address <= 0x1FFF {
                    self.ram_enabled = (value & 0x0F) == 0x0A;
                }
            }
            0x01..=0x03 => {
                // MBC1 - Complete implementation
                match address {
                    0x0000..=0x1FFF => {
                        // RAM Enable
                        self.ram_enabled = (value & 0x0F) == 0x0A;
                    }
                    0x2000..=0x3FFF => {
                        // ROM Bank Number (low 5 bits)
                        self.mbc1_rom_bank_low = value & 0x1F;
                        self.update_mbc1_rom_bank();
                    }
                    0x4000..=0x5FFF => {
                        // RAM Bank Number OR Upper Bits of ROM Bank Number
                        self.mbc1_rom_bank_high = value & 0x03;
                        if self.mbc1_banking_mode {
                            // RAM Banking Mode: This selects RAM bank
                            self.current_ram_bank = (value & 0x03) as usize;
                        }
                        self.update_mbc1_rom_bank();
                    }
                    0x6000..=0x7FFF => {
                        // Banking Mode Select
                        self.mbc1_banking_mode = (value & 0x01) != 0;
                        if !self.mbc1_banking_mode {
                            // ROM Banking Mode: Reset RAM bank to 0
                            self.current_ram_bank = 0;
                        } else {
                            // RAM Banking Mode: Use mbc1_rom_bank_high for RAM bank
                            self.current_ram_bank = self.mbc1_rom_bank_high as usize;
                        }
                        self.update_mbc1_rom_bank();
                    }
                    _ => {}
                }
            }
            0x0F..=0x13 => {
                // MBC3
                match address {
                    0x0000..=0x1FFF => {
                        self.ram_enabled = (value & 0x0F) == 0x0A;
                    }
                    0x2000..=0x3FFF => {
                        let bank = (value & 0x7F) as usize;
                        self.current_rom_bank = if bank == 0 { 1 } else { bank };
                    }
                    0x4000..=0x5FFF => {
                        if value <= 0x03 {
                            self.current_ram_bank = value as usize;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn read_ram(&self, address: u16) -> u8 {
        if !self.ram_enabled {
            return 0xFF;
        }
        let ram_address = (self.current_ram_bank * 0x2000) + (address as usize);
        if ram_address < self.ram.len() {
            self.ram[ram_address]
        } else {
            0xFF
        }
    }

    pub fn write_ram(&mut self, address: u16, value: u8) {
        if !self.ram_enabled {
            return;
        }
        let ram_address = (self.current_ram_bank * 0x2000) + (address as usize);
        if ram_address < self.ram.len() {
            self.ram[ram_address] = value;
        }
    }
    
    fn update_mbc1_rom_bank(&mut self) {
        // MBC1 substitutes 1 only when the five-bit register is zero.
        // Missing ROM address lines are masked *after* that substitution:
        // on a 4-bank cartridge, selecting 4 therefore maps bank 0, not 1.
        // The upper register affects the switchable window in both modes.
        let low = if self.mbc1_rom_bank_low == 0 { 1 } else { self.mbc1_rom_bank_low };
        let bank = ((self.mbc1_rom_bank_high as usize) << 5) | low as usize;
        self.current_rom_bank = bank & self.rom_bank_mask;
    }
}