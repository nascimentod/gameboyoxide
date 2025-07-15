use dmg_01::cartridge::Cartridge;

#[test]
fn test_cartridge_rom_banking() {
    let mut cartridge = Cartridge::new(vec![0; 0x10000]);
    cartridge.rom[0x0000] = 0x12;
    cartridge.rom[0x4000] = 0x34;
    cartridge.rom[0x8000] = 0x56;

    assert_eq!(cartridge.read_rom(0x0000), 0x12);
    assert_eq!(cartridge.read_rom(0x4000), 0x34);

    cartridge.current_rom_bank = 2;
    assert_eq!(cartridge.read_rom(0x4000), 0x56);
}