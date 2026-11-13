use dmg_01::memory::MemoryBus;

#[test]
fn dmg_register_masks_and_power_cycle() {
    let mut bus = MemoryBus::new();
    let masks = [
        0x80, 0x3f, 0, 0xff, 0xbf, 0xff, 0x3f, 0, 0xff, 0xbf, 0x7f, 0xff, 0x9f, 0xff, 0xbf, 0xff,
        0xff, 0, 0, 0xbf, 0, 0,
    ];
    assert_eq!(bus.read_byte(0xff26), 0x70);
    bus.write_byte(0xff26, 0x80);
    for (i, &mask) in masks.iter().enumerate() {
        let addr = 0xff10 + i as u16;
        bus.write_byte(addr, 0);
        assert_eq!(bus.read_byte(addr), mask, "{addr:04x}");
        bus.write_byte(addr, 0xff);
        assert_eq!(bus.read_byte(addr), 0xff, "{addr:04x}");
    }
    bus.write_byte(0xff1a, 0); // stop CH3 before accessing wave RAM on DMG
    bus.write_byte(0xff30, 0x42);
    bus.write_byte(0xff26, 0);
    assert_eq!(bus.read_byte(0xff26), 0x70);
    assert_eq!(bus.read_byte(0xff12), 0);
    assert_eq!(bus.read_byte(0xff30), 0x42);
    bus.write_byte(0xff12, 0xff);
    assert_eq!(bus.read_byte(0xff12), 0);
    bus.write_byte(0xff26, 0x80);
    assert_eq!(bus.read_byte(0xff12), 0);
}

#[test]
fn length_clock_uses_div_falling_edge() {
    let mut bus = MemoryBus::new();
    bus.write_byte(0xff26, 0x80);
    bus.write_byte(0xff12, 0xf0);
    bus.write_byte(0xff11, 0x3f); // one length tick
    bus.write_byte(0xff14, 0xc0); // trigger, length enabled
    assert_eq!(bus.read_byte(0xff26) & 1, 1);
    bus.step(8191);
    assert_eq!(bus.read_byte(0xff26) & 1, 1);
    bus.step(1);
    assert_eq!(bus.read_byte(0xff26) & 1, 0);
}

#[test]
fn triggers_respect_dac_and_nr52_is_read_only_except_power() {
    let mut bus = MemoryBus::new();
    bus.write_byte(0xff26, 0x8f);
    assert_eq!(bus.read_byte(0xff26), 0xf0);
    for trigger in [0xff14, 0xff19, 0xff1e, 0xff23] {
        bus.write_byte(trigger, 0x80);
    }
    assert_eq!(bus.read_byte(0xff26), 0xf0);
    for (dac, value) in [
        (0xff12, 0xf0),
        (0xff17, 0x08),
        (0xff1a, 0x80),
        (0xff21, 0xf0),
    ] {
        bus.write_byte(dac, value);
    }
    for trigger in [0xff14, 0xff19, 0xff1e, 0xff23] {
        bus.write_byte(trigger, 0x80);
    }
    assert_eq!(bus.read_byte(0xff26), 0xff);
    for dac in [0xff12, 0xff17, 0xff1a, 0xff21] {
        bus.write_byte(dac, 0);
    }
    assert_eq!(bus.read_byte(0xff26), 0xf0);
}
