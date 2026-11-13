#[path = "apu_signal.rs"]
mod signal;
pub use signal::{CLOCK_RATE, DEFAULT_SAMPLE_RATE};

/// DMG register interface and frame sequencer. All timing uses T-cycles.
/// References: Pan Docs Audio Registers / Audio Details; blargg dmg_sound.
pub struct Apu {
    registers: [u8; 0x17],
    wave_ram: [u8; 16],
    powered: bool,
    channels: u8,
    lengths: [u16; 4],
    frame_step: u8,
    sweep_shadow: u16,
    sweep_timer: u8,
    sweep_enabled: bool,
    sweep_negated: bool,
    wave_timer: u32,
    wave_position: u8,
    wave_buffer: u8,
    wave_access: u8,
    signal: signal::Signal,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            registers: [0; 0x17],
            wave_ram: [0; 16],
            powered: false,
            channels: 0,
            lengths: [0; 4],
            frame_step: 0,
            sweep_shadow: 0,
            sweep_timer: 0,
            sweep_enabled: false,
            sweep_negated: false,
            wave_timer: 0,
            wave_position: 0,
            wave_buffer: 0,
            wave_access: 0,
            signal: signal::Signal::new(),
        }
    }

    fn wave_period(&self) -> u32 {
        let frequency = u16::from(self.registers[13]) | (u16::from(self.registers[14] & 7) << 8);
        (2048 - u32::from(frequency)) * 2
    }

    /// Advance waveform timers in 4,194,304 Hz DMG T-cycles.
    pub fn step(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.wave_access = self.wave_access.saturating_sub(1);
            if self.channels & 4 != 0 {
                self.wave_timer = self.wave_timer.saturating_sub(1);
                if self.wave_timer == 0 {
                    self.wave_timer = self.wave_period();
                    self.wave_position = (self.wave_position + 1) & 31;
                    self.wave_buffer = self.wave_ram[usize::from(self.wave_position / 2)];
                    self.wave_access = 2;
                }
            }
            self.signal.tick(
                &self.registers,
                self.channels,
                self.powered,
                self.wave_sample(),
            );
        }
    }

    fn wave_sample(&self) -> u8 {
        if self.wave_position & 1 == 0 {
            self.wave_buffer >> 4
        } else {
            self.wave_buffer & 15
        }
    }

    /// Digital channel levels before DAC conversion.
    pub fn channel_outputs(&self) -> [u8; 4] {
        self.signal
            .outputs(&self.registers, self.channels, self.wave_sample())
    }

    /// Stereo mixer before high-pass filtering, normalized to [-1, 1].
    pub fn mixed_output(&self) -> [f32; 2] {
        self.signal.mix(
            &self.registers,
            self.channels,
            self.powered,
            self.wave_sample(),
        )
    }

    pub fn set_sample_rate(&mut self, rate: u32) {
        self.signal.set_sample_rate(rate);
    }
    pub fn take_samples(&mut self) -> Vec<[f32; 2]> {
        self.signal.take_samples()
    }

    /// Clock the frame sequencer on the falling edge of DIV bit 12.
    pub fn div_falling_edge(&mut self) {
        if !self.powered {
            return;
        }
        if self.frame_step % 2 == 0 {
            for (channel, &control) in [4usize, 9, 14, 19].iter().enumerate() {
                if self.registers[control] & 0x40 != 0 && self.lengths[channel] > 0 {
                    self.lengths[channel] -= 1;
                    if self.lengths[channel] == 0 {
                        self.channels &= !(1 << channel);
                    }
                }
            }
        }
        if self.frame_step == 2 || self.frame_step == 6 {
            self.clock_sweep();
        }
        if self.frame_step == 7 {
            self.signal.clock_envelopes(&self.registers);
        }
        self.frame_step = (self.frame_step + 1) & 7;
    }

    // https://gbdev.io/pandocs/Audio_details.html#pulse-channel-with-sweep-ch1
    fn sweep_calculate(&mut self) -> Option<u16> {
        let nr10 = self.registers[0];
        let delta = self.sweep_shadow >> (nr10 & 7);
        let result = if nr10 & 8 != 0 {
            self.sweep_negated = true;
            self.sweep_shadow - delta
        } else {
            self.sweep_shadow + delta
        };
        if result > 2047 {
            self.channels &= !1;
            None
        } else {
            Some(result)
        }
    }

    fn trigger_sweep(&mut self) {
        self.sweep_shadow = u16::from(self.registers[3]) | (u16::from(self.registers[4] & 7) << 8);
        let pace = (self.registers[0] >> 4) & 7;
        self.sweep_timer = if pace == 0 { 8 } else { pace };
        self.sweep_enabled = pace != 0 || self.registers[0] & 7 != 0;
        self.sweep_negated = false;
        if self.registers[0] & 7 != 0 {
            self.sweep_calculate();
        }
    }

    fn clock_sweep(&mut self) {
        if self.sweep_timer == 0 {
            return;
        }
        self.sweep_timer -= 1;
        if self.sweep_timer != 0 {
            return;
        }
        let pace = (self.registers[0] >> 4) & 7;
        self.sweep_timer = if pace == 0 { 8 } else { pace };
        if self.sweep_enabled && pace != 0 {
            if let Some(next) = self.sweep_calculate() {
                if self.registers[0] & 7 != 0 {
                    self.sweep_shadow = next;
                    self.registers[3] = next as u8;
                    self.registers[4] = (self.registers[4] & !7) | (next >> 8) as u8;
                    self.sweep_calculate();
                }
            }
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        const MASK: [u8; 0x17] = [
            0x80, 0x3f, 0x00, 0xff, 0xbf, 0xff, 0x3f, 0x00, 0xff, 0xbf, 0x7f, 0xff, 0x9f, 0xff,
            0xbf, 0xff, 0xff, 0x00, 0x00, 0xbf, 0x00, 0x00, 0x70,
        ];
        match address {
            0xff10..=0xff25 => {
                let i = (address - 0xff10) as usize;
                self.registers[i] | MASK[i]
            }
            0xff26 => {
                0x70 | if self.powered {
                    0x80 | self.channels
                } else {
                    0
                }
            }
            0xff27..=0xff2f => 0xff,
            0xff30..=0xff3f => {
                if self.channels & 4 == 0 {
                    self.wave_ram[(address - 0xff30) as usize]
                } else if self.wave_access != 0 {
                    self.wave_ram[usize::from(self.wave_position / 2)]
                } else {
                    0xff
                }
            }
            _ => 0xff,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xff26 => {
                let on = value & 0x80 != 0;
                if self.powered && !on {
                    self.signal.power_off();
                    self.registers = [0; 0x17];
                    self.channels = 0;
                    self.sweep_shadow = 0;
                    self.sweep_timer = 0;
                    self.sweep_enabled = false;
                    self.sweep_negated = false;
                    self.wave_timer = 0;
                    self.wave_position = 0;
                    self.wave_buffer = 0;
                    self.wave_access = 0;
                }
                if on && !self.powered {
                    self.frame_step = 0;
                }
                self.powered = on;
            }
            0xff30..=0xff3f => {
                if self.channels & 4 == 0 {
                    self.wave_ram[(address - 0xff30) as usize] = value;
                } else if self.wave_access != 0 {
                    self.wave_ram[usize::from(self.wave_position / 2)] = value;
                }
            }
            0xff10..=0xff25 if self.powered => {
                let i = (address - 0xff10) as usize;
                let old = self.registers[i];
                if let Some(ch) = match address {
                    0xff12 => Some(0),
                    0xff17 => Some(1),
                    0xff21 => Some(3),
                    _ => None,
                } {
                    self.signal
                        .envelope_write(ch, old, value, self.channels & (1 << ch) != 0);
                }
                if address == 0xff10 && old & 8 != 0 && value & 8 == 0 && self.sweep_negated {
                    self.channels &= !1;
                }
                if address == 0xff1e && value & 0x80 != 0 {
                    // DMG retrigger while the wave fetch is in progress corrupts
                    // the first byte, or the first aligned four-byte block.
                    if self.channels & 4 != 0 && self.wave_timer == 2 {
                        let index = usize::from(((self.wave_position + 1) & 31) / 2);
                        if index < 4 {
                            self.wave_ram[0] = self.wave_ram[index];
                        } else {
                            let start = index & !3;
                            self.wave_ram.copy_within(start..start + 4, 0);
                        }
                    }
                }
                self.registers[i] = value;
                if matches!(address, 0xff14 | 0xff19 | 0xff1e | 0xff23) {
                    let channel = match address {
                        0xff14 => 0,
                        0xff19 => 1,
                        0xff1e => 2,
                        _ => 3,
                    };
                    let extra_clock = self.frame_step % 2 == 1;
                    if extra_clock
                        && old & 0x40 == 0
                        && value & 0x40 != 0
                        && self.lengths[channel] > 0
                    {
                        self.lengths[channel] -= 1;
                        if self.lengths[channel] == 0 {
                            self.channels &= !(1 << channel);
                        }
                    }
                    if value & 0x80 != 0 && self.lengths[channel] == 0 {
                        self.lengths[channel] = if channel == 2 { 256 } else { 64 };
                        if extra_clock && value & 0x40 != 0 {
                            self.lengths[channel] -= 1;
                        }
                    }
                }
                match address {
                    0xff11 => self.lengths[0] = 64 - u16::from(value & 63),
                    0xff16 => self.lengths[1] = 64 - u16::from(value & 63),
                    0xff1b => self.lengths[2] = 256 - u16::from(value),
                    0xff20 => self.lengths[3] = 64 - u16::from(value & 63),
                    _ => {}
                }
                // Disabling a channel's DAC immediately clears its status.
                match address {
                    0xff12 if value & 0xf8 == 0 => self.channels &= !1,
                    0xff17 if value & 0xf8 == 0 => self.channels &= !2,
                    0xff1a if value & 0x80 == 0 => self.channels &= !4,
                    0xff21 if value & 0xf8 == 0 => self.channels &= !8,
                    _ => {}
                }
                if value & 0x80 != 0 {
                    let (bit, dac): (u8, bool) = match address {
                        0xff14 => (1, self.registers[2] & 0xf8 != 0),
                        0xff19 => (2, self.registers[7] & 0xf8 != 0),
                        0xff1e => (4, self.registers[10] & 0x80 != 0),
                        0xff23 => (8, self.registers[17] & 0xf8 != 0),
                        _ => (0, false),
                    };
                    if bit != 0 {
                        let channel = bit.trailing_zeros() as usize;
                        if self.lengths[channel] == 0 {
                            self.lengths[channel] = if channel == 2 { 256 } else { 64 };
                        }
                    }
                    if dac {
                        self.channels |= bit;
                    }
                    if bit != 0 {
                        self.signal.trigger(
                            &self.registers,
                            bit.trailing_zeros() as usize,
                            self.frame_step,
                        );
                    }
                    if address == 0xff14 {
                        self.trigger_sweep();
                    }
                    if address == 0xff1e {
                        self.wave_position = 0;
                        self.wave_timer = self.wave_period() + 6;
                        self.wave_access = 0;
                    }
                }
            }
            // On DMG the length fields remain writable while powered off.
            0xff11 | 0xff16 | 0xff1b | 0xff20 => {
                let i = (address - 0xff10) as usize;
                self.registers[i] = if address == 0xff1b {
                    value
                } else {
                    value & 0x3f
                };
                let channel = match address {
                    0xff11 => 0,
                    0xff16 => 1,
                    0xff1b => 2,
                    _ => 3,
                };
                self.lengths[channel] = if channel == 2 {
                    256 - u16::from(value)
                } else {
                    64 - u16::from(value & 63)
                };
            }
            _ => {}
        }
    }
}
