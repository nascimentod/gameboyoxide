use crate::cartridge::Cartridge;

pub struct MemoryBus {
    pub memory: [u8; 0x10000],
    pub cartridge: Option<Cartridge>,
}

impl MemoryBus {
    pub fn new() -> Self {
        MemoryBus {
            memory: [0; 0x10000],
            cartridge: None,
        }
    }

    pub fn load_cartridge(&mut self, rom_data: Vec<u8>) {
        let cartridge = Cartridge::new(rom_data);
        self.cartridge = Some(cartridge);

        // Copy ROM data to memory starting at 0x0100 (Game Boy boot address)
        if let Some(cartridge) = &self.cartridge {
            for (i, &byte) in cartridge.rom.iter().enumerate() {
                if i + 0x0100 < 0x8000 {
                    self.memory[i + 0x0100] = byte;
                }
            }
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => {
                // For addresses 0x0100-0x7FFF, read from memory (loaded ROM)
                // For addresses 0x0000-0x00FF, read from cartridge if available
                if address >= 0x0100 {
                    self.memory[address as usize]
                } else if let Some(cartridge) = &self.cartridge {
                    cartridge.read_rom(address)
                } else {
                    0xFF
                }
            }
            0x8000..=0x9FFF => self.memory[address as usize],
            0xA000..=0xBFFF => {
                if let Some(cartridge) = &self.cartridge {
                    cartridge.read_ram(address - 0xA000)
                } else {
                    0xFF
                }
            }
            0xC000..=0xFDFF => self.memory[address as usize],
            0xFE00..=0xFE9F => self.memory[address as usize],
            0xFEA0..=0xFEFF => 0x00,
            0xFF00..=0xFF7F => self.memory[address as usize],
            0xFF80..=0xFFFE => self.memory[address as usize],
            0xFFFF => self.memory[address as usize],
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x7FFF => {
                if let Some(cartridge) = &mut self.cartridge {
                    cartridge.write_rom(address, value);
                }
            }
            0x8000..=0x9FFF => self.memory[address as usize] = value,
            0xA000..=0xBFFF => {
                if let Some(cartridge) = &mut self.cartridge {
                    cartridge.write_ram(address - 0xA000, value);
                }
            }
            0xC000..=0xFDFF => self.memory[address as usize] = value,
            0xFE00..=0xFE9F => self.memory[address as usize] = value,
            0xFEA0..=0xFEFF => {}
            0xFF00..=0xFF7F => self.memory[address as usize] = value,
            0xFF80..=0xFFFE => self.memory[address as usize] = value,
            0xFFFF => self.memory[address as usize] = value,
        }
    }
}