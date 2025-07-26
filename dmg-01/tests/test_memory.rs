use dmg_01::memory::MemoryBus;

#[test]
fn test_memory_bus_cartridge_loading() {
    let mut bus = MemoryBus::new();
    let test_rom = vec![0x12, 0x34, 0x56, 0x78];
    bus.load_cartridge(test_rom);
    bus.boot_rom_enabled = false; // Disable boot ROM to access cartridge

    assert_eq!(bus.read_byte(0x0000), 0x12);
    assert_eq!(bus.read_byte(0x0001), 0x34);
    assert_eq!(bus.read_byte(0x0002), 0x56);
    assert_eq!(bus.read_byte(0x0003), 0x78);
}

#[test]
fn test_memory_bus_write_read() {
    let mut bus = MemoryBus::new();

    bus.write_byte(0x8000, 0x42);
    assert_eq!(bus.read_byte(0x8000), 0x42);

    bus.write_byte(0xC000, 0x55);
    assert_eq!(bus.read_byte(0xC000), 0x55);

    bus.write_byte(0xFF80, 0xAA);
    assert_eq!(bus.read_byte(0xFF80), 0xAA);
}