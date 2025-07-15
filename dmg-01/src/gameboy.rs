use crate::cpu::CPU;

pub struct GameBoy {
    pub cpu: CPU,
}

impl GameBoy {
    pub fn new() -> Self {
        GameBoy { cpu: CPU::new() }
    }

    pub fn load_cartridge(&mut self, rom_data: Vec<u8>) {
        self.cpu.bus.load_cartridge(rom_data);
    }

    pub fn run(&mut self) {
        loop {
            self.cpu.step();
            if self.cpu.halted {
                break;
            }
        }
    }

    pub fn step(&mut self) {
        self.cpu.step();
    }
}