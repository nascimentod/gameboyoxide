// Game Boy Timer System
// Handles DIV, TIMA, TMA, and TAC registers

pub struct Timer {
    pub div_counter: u16,    // Internal counter for DIV register (increments at CPU speed)
    pub div_register: u8,    // DIV register (0xFF04) - upper 8 bits of div_counter
    pub tima: u8,           // TIMA register (0xFF05) - Timer counter
    pub tma: u8,            // TMA register (0xFF06) - Timer modulo
    pub tac: u8,            // TAC register (0xFF07) - Timer control
    pub timer_counter: u16, // Internal counter for TIMA
    pub timer_overflow: bool, // Flag indicating TIMA overflow (triggers interrupt)
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div_counter: 0,
            div_register: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            timer_counter: 0,
            timer_overflow: false,
        }
    }

    // Step the timer forward by the given number of CPU cycles
    pub fn step(&mut self, cycles: u32) -> bool {
        let mut interrupt_requested = false;

        // Update DIV register (always runs at CPU speed / 4, so 16384 Hz at 4.194304 MHz)
        // DIV increments every 256 CPU cycles
        self.div_counter = self.div_counter.wrapping_add(cycles as u16);
        self.div_register = (self.div_counter >> 8) as u8;

        // Update TIMA if timer is enabled
        if self.is_timer_enabled() {
            let timer_frequency = self.get_timer_frequency();
            self.timer_counter = self.timer_counter.wrapping_add(cycles as u16);

            // Check if we need to increment TIMA
            while self.timer_counter >= timer_frequency {
                self.timer_counter -= timer_frequency;
                
                if self.tima == 0xFF {
                    // TIMA overflow - reset to TMA and request interrupt
                    self.tima = self.tma;
                    self.timer_overflow = true;
                    interrupt_requested = true;
                } else {
                    self.tima = self.tima.wrapping_add(1);
                }
            }
        }

        interrupt_requested
    }

    fn is_timer_enabled(&self) -> bool {
        (self.tac & 0x04) != 0
    }

    fn get_timer_frequency(&self) -> u16 {
        // Returns number of CPU cycles between TIMA increments
        match self.tac & 0x03 {
            0x00 => 1024,  // 4096 Hz
            0x01 => 16,    // 262144 Hz
            0x02 => 64,    // 65536 Hz
            0x03 => 256,   // 16384 Hz
            _ => unreachable!(),
        }
    }

    // Read timer registers
    pub fn read_register(&self, address: u16) -> u8 {
        match address {
            0xFF04 => self.div_register,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8, // Upper 5 bits always read as 1
            _ => 0xFF,
        }
    }

    // Write timer registers
    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0xFF04 => {
                // Writing any value to DIV resets it to 0
                self.div_counter = 0;
                self.div_register = 0;
            }
            0xFF05 => {
                self.tima = value;
            }
            0xFF06 => {
                self.tma = value;
            }
            0xFF07 => {
                self.tac = value & 0x07; // Only lower 3 bits are writable
                // Reset timer counter when TAC is written
                self.timer_counter = 0;
            }
            _ => {}
        }
    }

    // Check and clear timer overflow flag
    pub fn check_and_clear_overflow(&mut self) -> bool {
        if self.timer_overflow {
            self.timer_overflow = false;
            true
        } else {
            false
        }
    }
}