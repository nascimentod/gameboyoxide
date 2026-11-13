//! Waveform generators and stereo signal path. Time is in DMG T-cycles.
//! See Pan Docs Audio Registers and Audio Details. No emulator code is used.
use std::collections::VecDeque;

pub const CLOCK_RATE: u32 = 4_194_304;
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

pub(super) struct Signal {
    pulse_timers: [u32; 2],
    pulse_positions: [u8; 2],
    pulse_started: [bool; 2],
    pulse_valid: [bool; 2],
    volumes: [u8; 4],
    envelope_timers: [u8; 4],
    envelope_running: [bool; 4],
    noise_timer: u32,
    noise_lfsr: u16,
    sample_rate: u32,
    sample_phase: u32,
    sample_sum: [f32; 2],
    sample_cycles: u32,
    capacitors: [f32; 2],
    high_pass_factor: f32,
    samples: VecDeque<[f32; 2]>,
}

impl Signal {
    pub fn new() -> Self {
        Self {
            pulse_timers: [0; 2],
            pulse_positions: [0; 2],
            pulse_started: [false; 2],
            pulse_valid: [false; 2],
            volumes: [0; 4],
            envelope_timers: [0; 4],
            envelope_running: [false; 4],
            noise_timer: 0,
            noise_lfsr: 0x7fff,
            sample_rate: DEFAULT_SAMPLE_RATE,
            sample_phase: 0,
            sample_sum: [0.0; 2],
            sample_cycles: 0,
            capacitors: [0.0; 2],
            high_pass_factor: 0.999958_f32.powf(CLOCK_RATE as f32 / DEFAULT_SAMPLE_RATE as f32),
            samples: VecDeque::new(),
        }
    }

    pub fn power_off(&mut self) {
        self.pulse_timers = [0; 2];
        self.pulse_positions = [0; 2];
        self.pulse_started = [false; 2];
        self.pulse_valid = [false; 2];
        self.volumes = [0; 4];
        self.envelope_timers = [0; 4];
        self.envelope_running = [false; 4];
        self.noise_timer = 0;
        self.noise_lfsr = 0x7fff;
    }

    fn pulse_period(r: &[u8; 0x17], ch: usize) -> u32 {
        let frequency = u16::from(r[ch * 5 + 3]) | (u16::from(r[ch * 5 + 4] & 7) << 8);
        (2048 - u32::from(frequency)) * 4
    }

    fn noise_period(r: &[u8; 0x17]) -> u32 {
        const DIVISORS: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];
        DIVISORS[usize::from(r[18] & 7)] << (r[18] >> 4)
    }

    pub fn tick(&mut self, r: &[u8; 0x17], channels: u8, powered: bool, wave: u8) {
        if powered {
            for ch in 0..2 {
                if self.pulse_started[ch] {
                    self.pulse_timers[ch] = self.pulse_timers[ch].saturating_sub(1);
                    if self.pulse_timers[ch] == 0 {
                        self.pulse_timers[ch] = Self::pulse_period(r, ch);
                        self.pulse_positions[ch] = (self.pulse_positions[ch] + 1) & 7;
                        self.pulse_valid[ch] = true;
                    }
                }
            }
            if channels & 8 != 0 {
                self.noise_timer = self.noise_timer.saturating_sub(1);
                if self.noise_timer == 0 {
                    self.noise_timer = Self::noise_period(r);
                    // Clock shifts 14/15 halt the DMG LFSR.
                    if r[18] >> 4 < 14 {
                        let bit = (self.noise_lfsr ^ (self.noise_lfsr >> 1)) & 1;
                        self.noise_lfsr = (self.noise_lfsr >> 1) | (bit << 14);
                        if r[18] & 8 != 0 {
                            self.noise_lfsr = (self.noise_lfsr & !(1 << 6)) | (bit << 6);
                        }
                    }
                }
            }
        }
        // Box integration before decimation reduces aliasing; not a band-limited resampler.
        let mixed = self.mix(r, channels, powered, wave);
        for side in 0..2 {
            self.sample_sum[side] += mixed[side];
        }
        self.sample_cycles += 1;
        self.sample_phase += self.sample_rate;
        if self.sample_phase >= CLOCK_RATE {
            self.sample_phase -= CLOCK_RATE;
            let mut sample = [0.0; 2];
            let dacs_on = (0..4).any(|ch| Self::dac_enabled(r, ch, powered));
            for side in 0..2 {
                let input = self.sample_sum[side] / self.sample_cycles as f32;
                if dacs_on {
                    sample[side] = input - self.capacitors[side];
                    self.capacitors[side] = input - sample[side] * self.high_pass_factor;
                }
            }
            self.sample_sum = [0.0; 2];
            self.sample_cycles = 0;
            // Bound storage during headless runs with no audio consumer.
            if self.samples.len() >= self.sample_rate as usize / 4 {
                self.samples.pop_front();
            }
            self.samples.push_back(sample);
        }
    }

    fn dac_enabled(r: &[u8; 0x17], ch: usize, powered: bool) -> bool {
        powered
            && match ch {
                0 => r[2] & 0xf8 != 0,
                1 => r[7] & 0xf8 != 0,
                2 => r[10] & 0x80 != 0,
                _ => r[17] & 0xf8 != 0,
            }
    }

    pub fn outputs(&self, r: &[u8; 0x17], channels: u8, wave: u8) -> [u8; 4] {
        const DUTIES: [[u8; 8]; 4] = [
            [0, 0, 0, 0, 0, 0, 0, 1],
            [1, 0, 0, 0, 0, 0, 0, 1],
            [1, 0, 0, 0, 0, 1, 1, 1],
            [0, 1, 1, 1, 1, 1, 1, 0],
        ];
        let mut out = [0; 4];
        for ch in 0..2 {
            if channels & (1 << ch) != 0 && self.pulse_valid[ch] {
                let duty = usize::from(r[ch * 5 + 1] >> 6);
                out[ch] = DUTIES[duty][usize::from(self.pulse_positions[ch])] * self.volumes[ch];
            }
        }
        if channels & 4 != 0 {
            let level = (r[12] >> 5) & 3;
            out[2] = if level == 0 { 0 } else { wave >> (level - 1) };
        }
        if channels & 8 != 0 && self.noise_lfsr & 1 == 0 {
            out[3] = self.volumes[3];
        }
        out
    }

    pub fn mix(&self, r: &[u8; 0x17], channels: u8, powered: bool, wave: u8) -> [f32; 2] {
        let digital = self.outputs(r, channels, wave);
        let mut out = [0.0; 2];
        for ch in 0..4 {
            if Self::dac_enabled(r, ch, powered) {
                let analog = 1.0 - f32::from(digital[ch]) / 7.5;
                if r[21] & (0x10 << ch) != 0 {
                    out[0] += analog;
                }
                if r[21] & (1 << ch) != 0 {
                    out[1] += analog;
                }
            }
        }
        // NR50 zero is 1/8 volume, not mute. VIN is not connected.
        out[0] *= f32::from(((r[20] >> 4) & 7) + 1) / 32.0;
        out[1] *= f32::from((r[20] & 7) + 1) / 32.0;
        out
    }

    pub fn set_sample_rate(&mut self, rate: u32) {
        assert!((8_000..=192_000).contains(&rate));
        self.sample_rate = rate;
        self.high_pass_factor = 0.999958_f32.powf(CLOCK_RATE as f32 / rate as f32);
        self.sample_phase = 0;
        self.sample_sum = [0.0; 2];
        self.sample_cycles = 0;
        self.samples.clear();
    }

    pub fn take_samples(&mut self) -> Vec<[f32; 2]> {
        self.samples.drain(..).collect()
    }

    pub fn clock_envelopes(&mut self, r: &[u8; 0x17]) {
        for (ch, index) in [(0, 2), (1, 7), (3, 17)] {
            if !self.envelope_running[ch] {
                continue;
            }
            self.envelope_timers[ch] = self.envelope_timers[ch].saturating_sub(1);
            if self.envelope_timers[ch] == 0 {
                let nr = r[index];
                self.envelope_timers[ch] = if nr & 7 == 0 { 8 } else { nr & 7 };
                if nr & 7 == 0 {
                    continue;
                }
                let volume = self.volumes[ch] as i8 + if nr & 8 != 0 { 1 } else { -1 };
                if (0..=15).contains(&volume) {
                    self.volumes[ch] = volume as u8;
                } else {
                    self.envelope_running[ch] = false;
                }
            }
        }
    }

    pub fn trigger(&mut self, r: &[u8; 0x17], ch: usize, frame_step: u8) {
        if ch != 2 {
            let nr = r[if ch == 3 { 17 } else { ch * 5 + 2 }];
            self.volumes[ch] = nr >> 4;
            self.envelope_timers[ch] = if nr & 7 == 0 { 8 } else { nr & 7 };
            if frame_step == 7 {
                self.envelope_timers[ch] += 1;
            }
            self.envelope_running[ch] = true;
        }
        if ch < 2 {
            // Retrigger preserves both duty phase and the low timer bits.
            self.pulse_timers[ch] = (Self::pulse_period(r, ch) & !3) | (self.pulse_timers[ch] & 3);
            self.pulse_started[ch] = true;
        } else if ch == 3 {
            self.noise_lfsr = 0x7fff;
            self.noise_timer = Self::noise_period(r);
        }
    }

    pub fn envelope_write(&mut self, ch: usize, old: u8, _new: u8, active: bool) {
        // The portable DMG zombie-envelope case documented by Pan Docs.
        // Other live envelope writes vary by DMG revision and remain approximate.
        if active && old & 7 == 0 && old & 8 != 0 && self.envelope_running[ch] {
            self.volumes[ch] = (self.volumes[ch] + 1) & 15;
        }
    }
}
