use dmg_01::gameboy::GameBoy;

#[test]
fn test_gameboy_integration() {
    let mut gameboy = GameBoy::new();
    let mut test_rom = vec![0; 0x8000]; // 32KB ROM
    // Place test code at entry point 0x0100
    test_rom[0x0100] = 0x3E; // LD A, 0x42
    test_rom[0x0101] = 0x42;
    test_rom[0x0102] = 0x06; // LD B, 0x10
    test_rom[0x0103] = 0x10;
    test_rom[0x0104] = 0x80; // ADD A, B
    test_rom[0x0105] = 0x76; // HALT

    gameboy.load_cartridge(test_rom);

    assert_eq!(gameboy.cpu.pc, 0x0100);

    // Check that memory was loaded correctly
    assert_eq!(gameboy.cpu.bus.read_byte(0x0100), 0x3E);
    assert_eq!(gameboy.cpu.bus.read_byte(0x0101), 0x42);

    gameboy.step();
    assert_eq!(gameboy.cpu.registers.a, 0x42);
    assert_eq!(gameboy.cpu.pc, 0x0102);

    gameboy.step();
    assert_eq!(gameboy.cpu.registers.b, 0x10);
    assert_eq!(gameboy.cpu.pc, 0x0104);

    gameboy.step();
    assert_eq!(gameboy.cpu.registers.a, 0x52);
    assert_eq!(gameboy.cpu.pc, 0x0105);

    gameboy.step();
    assert!(gameboy.cpu.halted);
}