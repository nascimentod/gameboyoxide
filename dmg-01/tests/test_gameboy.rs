use dmg_01::gameboy::GameBoy;

#[test]
fn test_gameboy_integration() {
    let mut gameboy = GameBoy::new();
    let test_rom = vec![
        0x3E, 0x42, // LD A, 0x42
        0x06, 0x10, // LD B, 0x10
        0x80, // ADD A, B
        0x76, // HALT
    ];

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