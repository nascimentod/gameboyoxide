use dmg_01::memory::MemoryBus;

fn start(length: u8, control: u8) -> MemoryBus {
    let mut b = MemoryBus::new();
    b.write_byte(0xff26, 0x80);
    b.write_byte(0xff12, 0xf0);
    b.write_byte(0xff11, 64 - length);
    b.write_byte(0xff14, control);
    b
}
fn active(b: &MemoryBus) -> bool {
    b.read_byte(0xff26) & 1 != 0
}

#[test]
fn retrigger_of_zero_length_in_non_length_phase_loads_max_minus_one() {
    let mut b = start(1, 0xc0);
    b.step(8192);
    assert!(!active(&b));
    b.write_byte(0xff14, 0xc0);
    b.step(62 * 16384);
    assert!(active(&b));
    b.step(16384);
    assert!(!active(&b));
}

#[test]
fn enable_trigger_can_clock_length_twice() {
    let mut b = start(1, 0x80); // disabled length, running channel
    b.step(8192);
    b.write_byte(0xff14, 0xc0); // 1 -> 0 -> 64 -> 63
    b.step(62 * 16384);
    assert!(active(&b));
    b.step(16384);
    assert!(!active(&b));
}

#[test]
fn power_cycle_preserves_dmg_lengths_and_div_phase_but_resets_sequencer() {
    let mut b = start(2, 0xc0);
    b.step(8192 + 4096); // length = 1, divider halfway to next edge
    b.write_byte(0xff26, 0);
    b.write_byte(0xff26, 0x80);
    b.write_byte(0xff12, 0xf0);
    b.write_byte(0xff14, 0xc0); // existing length 1, frame step reset to 0
    b.step(4095);
    assert!(active(&b));
    b.step(1);
    assert!(!active(&b));
}

#[test]
fn div_reset_while_powered_off_does_not_advance_sequencer() {
    let mut b = start(2, 0xc0);
    b.write_byte(0xff26, 0);
    b.step(4096);
    b.write_byte(0xff04, 0);
    b.write_byte(0xff26, 0x80);
    b.write_byte(0xff12, 0xf0);
    b.write_byte(0xff14, 0xc0);
    b.step(8192);
    assert!(active(&b));
    b.step(8192);
    assert!(active(&b)); // step 1 doesn't clock length
    b.step(8192);
    assert!(!active(&b));
}

#[test]
fn length_writes_while_powered_off_survive_power_on() {
    let mut b = MemoryBus::new();
    b.write_byte(0xff11, 63);
    b.write_byte(0xff26, 0x80);
    b.write_byte(0xff12, 8);
    b.write_byte(0xff14, 0xc0);
    b.step(8192);
    assert!(!active(&b));
}

#[test]
fn stepping_in_batches_preserves_waveform_and_frame_event_order() {
    let mut a = start(32, 0xc0);
    let mut b = start(32, 0xc0);
    for bus in [&mut a, &mut b] {
        bus.write_byte(0xff10, 0x19);
        bus.write_byte(0xff13, 0x80);
        bus.write_byte(0xff14, 0xc3);
        bus.write_byte(0xff24, 0x77);
        bus.write_byte(0xff25, 0x11);
    }
    a.step(200_000);
    for _ in 0..50_000 {
        b.step(4);
    }
    assert_eq!(a.read_byte(0xff26), b.read_byte(0xff26));
    assert_eq!(a.apu.channel_outputs(), b.apu.channel_outputs());
    assert_eq!(a.apu.take_samples(), b.apu.take_samples());
}
