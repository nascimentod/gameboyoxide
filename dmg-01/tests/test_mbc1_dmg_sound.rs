use dmg_01::cartridge::Cartridge;

#[test]
fn dmg_sound_subtest_scan_terminates_after_twelve_entries() {
    let rom = std::fs::read("test-roms/dmg_sound.gb").unwrap();
    assert_eq!(rom.len(), 0x10000);
    assert_eq!(rom[0x147], 3); // MBC1
    assert_eq!(rom[0x148], 1); // four 16 KiB banks
    let mut cart = Cartridge::new(rom);
    // ROM 0x231A..0x2335 selects bank (index / 4 + 1), then
    // probes 0x4000 + (index % 4)*0x1000 for the JP opcode C3.
    for index in 0..13u8 {
        let requested_bank = index / 4 + 1;
        let address = 0x4000 + u16::from(index % 4) * 0x1000;
        cart.write_rom(0x2000, requested_bank);
        let byte = cart.read_rom(address);
        if index < 12 {
            assert_eq!(byte, 0xc3, "subtest {}", index + 1);
        } else {
            assert_eq!(cart.current_rom_bank, 0);
            assert_eq!(byte, 0x00); // ROM offset 0, not the first subtest
            assert_ne!(byte, 0xc3);
        }
    }
}

#[test]
fn mbc1_remaps_register_zero_before_disconnected_address_lines() {
    let mut rom = vec![0; 4 * 0x4000];
    for bank in 0..4 { rom[bank * 0x4000] = bank as u8; }
    rom[0x147] = 1;
    let mut cart = Cartridge::new(rom);
    for (register, expected) in [(0, 1), (1, 1), (2, 2), (3, 3), (4, 0), (5, 1), (8, 0), (31, 3), (32, 1)] {
        cart.write_rom(0x2000, register);
        assert_eq!(cart.read_rom(0x4000), expected, "register {register}");
    }
}

#[test]
fn mbc1_upper_bits_affect_switchable_window_in_both_modes() {
    let mut rom = vec![0; 128 * 0x4000];
    for bank in 0..128 { rom[bank * 0x4000] = bank as u8; }
    rom[0x147] = 1;
    let mut cart = Cartridge::new(rom);
    cart.write_rom(0x2000, 2);
    cart.write_rom(0x4000, 2);
    assert_eq!(cart.read_rom(0x4000), 66);
    assert_eq!(cart.read_rom(0x0000), 0);
    cart.write_rom(0x6000, 1);
    assert_eq!(cart.read_rom(0x4000), 66);
    assert_eq!(cart.read_rom(0x0000), 64);
    cart.write_rom(0x2000, 0);
    assert_eq!(cart.read_rom(0x4000), 65);
}
