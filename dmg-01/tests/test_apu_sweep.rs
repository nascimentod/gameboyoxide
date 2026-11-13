use dmg_01::memory::MemoryBus;

fn setup(nr10: u8, frequency: u16) -> MemoryBus {
    let mut b = MemoryBus::new();
    b.write_byte(0xff26, 0x80);
    b.write_byte(0xff10, nr10);
    b.write_byte(0xff12, 0xf0);
    b.write_byte(0xff13, frequency as u8);
    b.write_byte(0xff14, 0x80 | (frequency >> 8) as u8);
    b
}

#[test]
fn trigger_checks_sweep_overflow_immediately() {
    let b = setup(0x11, 1400);
    assert_eq!(b.read_byte(0xff26) & 1, 0);
}

#[test]
fn sweep_clocks_and_checks_second_overflow() {
    let mut b = setup(0x11, 1000);
    assert_eq!(b.read_byte(0xff26) & 1, 1);
    b.step(8192 * 3);
    assert_eq!(b.read_byte(0xff26) & 1, 0); // 1000 -> 1500 -> 2250
}

#[test]
fn clearing_used_negate_disables_channel() {
    let mut b = setup(0x19, 1000);
    assert_eq!(b.read_byte(0xff26) & 1, 1);
    b.write_byte(0xff10, 0x11);
    assert_eq!(b.read_byte(0xff26) & 1, 0);
}

#[test]
fn enabling_length_between_length_clocks_ticks_immediately() {
    let mut b = setup(0, 0);
    b.write_byte(0xff11, 0x3f);
    b.step(8192); // next frame step does not clock length
    b.write_byte(0xff14, 0x40);
    assert_eq!(b.read_byte(0xff26) & 1, 0);
}

#[test]
fn div_reset_only_clocks_on_high_bit() {
    let mut b = setup(0, 0);
    b.write_byte(0xff11, 0x3f);
    b.write_byte(0xff14, 0x40);
    b.step(4095);
    b.write_byte(0xff04, 0);
    assert_eq!(b.read_byte(0xff26) & 1, 1);
    b.step(4096);
    b.write_byte(0xff04, 0);
    assert_eq!(b.read_byte(0xff26) & 1, 0);
}
