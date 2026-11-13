use dmg_01::apu::{Apu, CLOCK_RATE};

fn on() -> Apu {
    let mut a = Apu::new();
    a.write(0xff26, 0x80);
    a
}
fn pulse(a: &mut Apu, envelope: u8) {
    a.write(0xff11, 0x80); // 50% duty
    a.write(0xff12, envelope);
    a.write(0xff13, 0xff);
    a.write(0xff14, 0x87); // period 4 T-cycles per duty step
}
fn frames(a: &mut Apu, count: usize) {
    for _ in 0..count {
        a.div_falling_edge();
    }
}

#[test]
fn pulse_frequency_duty_and_retrigger_phase() {
    let mut a = on();
    pulse(&mut a, 0xf0);
    assert_eq!(a.channel_outputs()[0], 0); // first step is silent
    let mut levels = Vec::new();
    for _ in 0..8 {
        a.step(4);
        levels.push(a.channel_outputs()[0]);
    }
    assert_eq!(levels, [0, 0, 0, 0, 15, 15, 15, 15]);
    a.write(0xff14, 0x87);
    assert_eq!(a.channel_outputs()[0], 15); // trigger doesn't reset duty
    a.step(4);
    assert_eq!(a.channel_outputs()[0], 0);
}

#[test]
fn envelope_clocks_at_step_seven_and_stops_at_boundary() {
    let mut a = on();
    pulse(&mut a, 0x21);
    a.step(32);
    assert_eq!(a.channel_outputs()[0], 2);
    frames(&mut a, 7);
    assert_eq!(a.channel_outputs()[0], 2);
    frames(&mut a, 1);
    assert_eq!(a.channel_outputs()[0], 1);
    frames(&mut a, 8);
    assert_eq!(a.channel_outputs()[0], 0);
    frames(&mut a, 80);
    assert_eq!(a.channel_outputs()[0], 0);
    assert_eq!(a.read(0xff26) & 1, 1); // zero volume does not disable DAC
}

#[test]
fn zero_envelope_pace_does_not_automatically_change_volume() {
    let mut a = on();
    pulse(&mut a, 0x58);
    a.step(32);
    frames(&mut a, 128);
    assert_eq!(a.channel_outputs()[0], 5);
    a.write(0xff12, 0x08); // documented portable zombie increment
    assert_eq!(a.channel_outputs()[0], 6);
}

#[test]
fn trigger_before_envelope_step_delays_first_update() {
    let mut a = on();
    frames(&mut a, 7);
    pulse(&mut a, 0x29);
    a.step(32);
    frames(&mut a, 1);
    assert_eq!(a.channel_outputs()[0], 2);
    frames(&mut a, 8);
    assert_eq!(a.channel_outputs()[0], 3);
}

fn wave(a: &mut Apu) {
    for i in 0..16 {
        a.write(0xff30 + i, (i * 0x11) as u8);
    }
    a.write(0xff1a, 0x80);
    a.write(0xff1c, 0x20);
    a.write(0xff1d, 0xfe);
    a.write(0xff1e, 0x87); // period 4, initial delay 6
}

#[test]
fn active_wave_ram_only_accessible_during_fetch_and_address_is_redirected() {
    let mut a = on();
    wave(&mut a);
    assert_eq!(a.read(0xff35), 0xff);
    a.write(0xff35, 0xf7); // ignored outside access window
    a.step(10);
    assert_eq!(a.read(0xff3f), 0);
    a.step(2);
    assert_eq!(a.read(0xff30), 0xff);
    a.step(2);
    assert_eq!(a.read(0xff30), 0x11);
    a.write(0xff3f, 0x42); // writes byte 1, not byte 15
    a.write(0xff1a, 0);
    assert_eq!(a.read(0xff31), 0x42);
    assert_eq!(a.read(0xff35), 0x55);
    assert_eq!(a.read(0xff3f), 0xff);
}

#[test]
fn wave_buffer_skips_first_nibble_and_volume_is_digital_shift() {
    let mut a = on();
    a.write(0xff30, 0xab);
    a.write(0xff1a, 0x80);
    a.write(0xff1c, 0x20);
    a.write(0xff1d, 0xff);
    a.write(0xff1e, 0x87);
    assert_eq!(a.channel_outputs()[2], 0);
    a.step(8);
    assert_eq!(a.channel_outputs()[2], 0xb);
    a.write(0xff1c, 0x40);
    assert_eq!(a.channel_outputs()[2], 5);
    a.write(0xff1c, 0x60);
    assert_eq!(a.channel_outputs()[2], 2);
    a.write(0xff1c, 0);
    assert_eq!(a.channel_outputs()[2], 0);
}

#[test]
fn wave_retrigger_corrupts_aligned_block_only_at_fetch_boundary() {
    let mut a = on();
    wave(&mut a);
    a.step(10 + 9 * 4 - 2); // two cycles before fetching sample 10 (byte 5)
    a.write(0xff1e, 0x87);
    a.write(0xff1a, 0);
    assert_eq!(
        (0..4).map(|i| a.read(0xff30 + i)).collect::<Vec<_>>(),
        [0x44, 0x55, 0x66, 0x77]
    );
}

#[test]
fn noise_lfsr_period_width_and_high_shift() {
    let mut a = on();
    a.write(0xff21, 0xf0);
    a.write(0xff22, 0);
    a.write(0xff23, 0x80);
    a.step(8 * 14);
    assert_eq!(a.channel_outputs()[3], 0);
    a.step(8);
    assert_eq!(a.channel_outputs()[3], 15);
    a.write(0xff22, 8);
    a.write(0xff23, 0x80);
    a.step(8 * 6);
    assert_eq!(a.channel_outputs()[3], 0);
    a.step(8);
    assert_eq!(a.channel_outputs()[3], 15);
    a.write(0xff22, 0xe0);
    a.write(0xff23, 0x80);
    a.step(200_000);
    assert_eq!(a.channel_outputs()[3], 0);
}

#[test]
fn mixer_routing_dac_polarity_and_nr50_zero_is_not_mute() {
    let mut a = on();
    a.write(0xff12, 0xf0); // inactive channel, enabled DAC -> +1
    a.write(0xff25, 0x10);
    a.write(0xff24, 0x70);
    assert_eq!(a.mixed_output(), [0.25, 0.0]);
    a.write(0xff24, 0);
    assert_eq!(a.mixed_output(), [1.0 / 32.0, 0.0]);
    a.write(0xff25, 1);
    assert_eq!(a.mixed_output(), [0.0, 1.0 / 32.0]);
    a.write(0xff12, 0);
    assert_eq!(a.mixed_output(), [0.0; 2]);
}

#[test]
fn second_pulse_and_noise_have_independent_envelopes() {
    let mut a = on();
    a.write(0xff16, 0x80);
    a.write(0xff17, 0x19);
    a.write(0xff18, 0xff);
    a.write(0xff19, 0x87);
    a.write(0xff21, 0x29);
    a.write(0xff22, 0);
    a.write(0xff23, 0x80);
    a.step(120);
    assert_eq!(a.channel_outputs(), [0, 1, 0, 2]);
    frames(&mut a, 8);
    assert_eq!(a.channel_outputs(), [0, 2, 0, 3]);
    a.write(0xff17, 0);
    assert_eq!(a.channel_outputs(), [0, 0, 0, 3]);
}

#[test]
fn generated_pulse_samples_have_expected_pitch_and_stereo_routing() {
    let mut a = on();
    a.set_sample_rate(48_000);
    a.write(0xff11, 0x80);
    a.write(0xff12, 0xf0);
    a.write(0xff13, 0xd6);
    a.write(0xff14, 0x86); // ~439.84 Hz
    a.write(0xff24, 0x77);
    a.write(0xff25, 0x10); // left only
    a.step(CLOCK_RATE / 8);
    let samples = a.take_samples();
    assert_eq!(samples.len(), 6000);
    assert!(samples.iter().all(|s| s[1] == 0.0));
    let rising = samples
        .windows(2)
        .filter(|s| s[0][0] <= 0.0 && s[1][0] > 0.0)
        .count();
    assert!(
        (54..=56).contains(&rising),
        "wrong pulse pitch: {rising} crossings in 1/8 s"
    );
}

#[test]
fn sampling_rate_high_pass_silence_and_bounded_queue() {
    let mut a = on();
    a.set_sample_rate(48_000);
    a.write(0xff12, 8);
    a.write(0xff25, 0x11);
    a.write(0xff24, 0x77);
    let mut count = 0;
    let mut first = None;
    let mut last = 0.0;
    for _ in 0..16 {
        a.step(CLOCK_RATE / 16);
        let samples = a.take_samples();
        count += samples.len();
        first.get_or_insert(samples[0][0]);
        last = samples.last().unwrap()[0];
        assert!(samples
            .iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 1.0));
    }
    assert_eq!(count, 48_000);
    assert!(first.unwrap() > 0.2);
    assert!(last.abs() < 0.001);
    a.write(0xff26, 0);
    a.step(4096);
    assert!(a.take_samples().iter().flatten().all(|x| *x == 0.0));
    a.step(CLOCK_RATE);
    assert_eq!(a.take_samples().len(), 12_000);
}
