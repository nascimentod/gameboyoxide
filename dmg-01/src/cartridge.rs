pub struct Cartridge {
    pub rom: Vec<u8>,
    pub rom_size: usize,
    pub ram: Vec<u8>,
    pub ram_size: usize,
    pub mbc_type: u8,
    pub current_rom_bank: usize,
    pub current_ram_bank: usize,
}

impl Cartridge {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let rom_size = rom_data.len();
        let mbc_type = if rom_size > 0x8000 {
            rom_data[0x0147]
        } else {
            0
        };
        let ram_size = match mbc_type {
            0x00 => 0,
            _ => 0x2000,
        };

        Cartridge {
            rom: rom_data,
            rom_size,
            ram: vec![0; ram_size],
            ram_size,
            mbc_type,
            current_rom_bank: 1,
            current_ram_bank: 0,
        }
    }

    pub fn read_rom(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                if (address as usize) < self.rom.len() {
                    self.rom[address as usize]
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
            0x00 => {}
            _ => match address {
                0x2000..=0x3FFF => {
                    let bank = (value & 0x1F) as usize;
                    self.current_rom_bank = if bank == 0 { 1 } else { bank };
                }
                0x4000..=0x5FFF => {
                    self.current_ram_bank = (value & 0x03) as usize;
                }
                _ => {}
            },
        }
    }

    pub fn read_ram(&self, address: u16) -> u8 {
        let ram_address = (self.current_ram_bank * 0x2000) + (address as usize);
        if ram_address < self.ram.len() {
            self.ram[ram_address]
        } else {
            0xFF
        }
    }

    pub fn write_ram(&mut self, address: u16, value: u8) {
        let ram_address = (self.current_ram_bank * 0x2000) + (address as usize);
        if ram_address < self.ram.len() {
            self.ram[ram_address] = value;
        }
    }
}