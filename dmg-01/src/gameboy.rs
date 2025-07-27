use crate::cpu::CPU;
use crate::ppu::{Color, LCD_WIDTH, LCD_HEIGHT};

pub struct GameBoy {
    pub cpu: CPU,
}

impl GameBoy {
    pub fn new() -> Self {
        GameBoy { cpu: CPU::new() }
    }

    pub fn load_cartridge(&mut self, rom_data: Vec<u8>) {
        self.cpu.bus.load_cartridge(rom_data);
        // Let the boot ROM run naturally - it will set up the Nintendo logo and initialize the system
        // The boot ROM will disable itself when complete (writes to 0xFF50)
        println!("Boot ROM enabled - will run Nintendo logo sequence");
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

    pub fn get_framebuffer(&self) -> &[Color; LCD_WIDTH * LCD_HEIGHT] {
        self.cpu.bus.ppu.get_framebuffer()
    }

    pub fn get_framebuffer_rgb(&self) -> Vec<(u8, u8, u8)> {
        self.cpu.bus.ppu.get_framebuffer()
            .iter()
            .map(|color| color.to_rgb())
            .collect()
    }

    // Joypad input methods
    pub fn press_button(&mut self, button: crate::joypad::JoypadButton) {
        self.cpu.press_button(button);
    }

    pub fn release_button(&mut self, button: crate::joypad::JoypadButton) {
        self.cpu.release_button(button);
    }

    pub fn get_joypad_state(&self) -> crate::joypad::JoypadState {
        self.cpu.get_joypad_state()
    }
}