use dmg_01::memory::MemoryBus;

#[test]
fn dma_copies_shadow_oam_to_all_160_oam_bytes() {
    let mut b = MemoryBus::new();
    for i in 0..160 {
        b.write_byte(0xc000 + i, (i as u8).wrapping_mul(17));
    }
    b.ppu.oam.fill(0xaa);
    b.write_byte(0xff46, 0xc0);
    b.step(644); // one startup M-cycle plus 160 transfer M-cycles
    for i in 0..160 {
        assert_eq!(b.ppu.oam[i], (i as u8).wrapping_mul(17), "OAM byte {i}");
    }
}

#[test]
fn dma_uses_mapped_vram_not_the_backing_memory_array() {
    let mut b = MemoryBus::new();
    for i in 0..160 {
        b.write_byte(0x8000 + i, i as u8);
    }
    b.write_byte(0xff46, 0x80);
    b.step(644);
    assert_eq!(
        &b.ppu.oam[..],
        &(0..160).map(|i| i as u8).collect::<Vec<_>>()
    );
}

#[test]
fn dma_progresses_one_byte_per_machine_cycle_and_blocks_non_hram_cpu_access() {
    let mut b = MemoryBus::new();
    b.write_byte(0xc000, 0x42);
    b.write_byte(0xc09f, 0x77);
    b.write_byte(0xff80, 0x12);
    b.write_byte(0xff46, 0xc0);
    b.step(4); // startup, no data byte yet
    assert_eq!(b.ppu.oam[0], 0);
    assert_eq!(b.read_byte(0xc000), 0xff);
    assert_eq!(b.read_byte(0xff80), 0x12);
    b.write_byte(0xc000, 0x99); // blocked
    b.write_byte(0xff80, 0x34); // HRAM remains accessible
    b.step(3);
    assert_eq!(b.ppu.oam[0], 0);
    b.step(1);
    assert_eq!(b.ppu.oam[0], 0x42);
    b.step(635);
    assert_eq!(b.ppu.oam[159], 0);
    b.step(1);
    assert_eq!(b.ppu.oam[159], 0x77);
    assert_eq!(b.read_byte(0xc000), 0x42);
    assert_eq!(b.read_byte(0xff80), 0x34);
}

#[test]
fn cpu_can_run_the_standard_dma_wait_routine_from_hram() {
    let mut c = dmg_01::cpu::CPU::new();
    c.bus.boot_rom_enabled = false;
    c.bus.load_cartridge(vec![0; 0x8000]);
    for i in 0..160 {
        c.bus.write_byte(0xc000 + i, i as u8);
    }
    // LD A,C0; LDH (46),A; LD A,40; DEC A; JR NZ,-3; RET.
    for (i, value) in [0x3e, 0xc0, 0xe0, 0x46, 0x3e, 40, 0x3d, 0x20, 0xfd, 0xc9]
        .into_iter()
        .enumerate()
    {
        c.bus.write_byte(0xff80 + i as u16, value);
    }
    c.pc = 0xff80;
    c.sp = 0xfffc;
    c.bus.write_byte(0xfffc, 0);
    c.bus.write_byte(0xfffd, 1);
    for _ in 0..100 {
        if c.pc == 0x100 {
            break;
        }
        c.step();
    }
    assert_eq!(c.pc, 0x100);
    assert_eq!(
        &c.bus.ppu.oam[..],
        &(0..160).map(|i| i as u8).collect::<Vec<_>>()
    );
}

#[test]
fn shadow_oam_dma_and_unsigned_obj_rendering_work_together() {
    use dmg_01::ppu::{Color, PPUMode, LCD_WIDTH};
    let mut b = MemoryBus::new();
    b.write_byte(0xff40, 0x83); // signed background, objects enabled
    for y in 0..8 {
        b.write_byte(0x8010 + y * 2, 0xff);
        b.write_byte(0x8011 + y * 2, 0xff);
    }
    for (offset, value) in [32, 24, 1, 0].into_iter().enumerate() {
        b.write_byte(0xc000 + offset as u16, value);
    }
    b.write_byte(0xff46, 0xc0);
    b.step(644);
    b.ppu.ly = 16;
    b.ppu.stat.mode = PPUMode::Drawing;
    b.ppu.mode_clock = 0;
    b.step(172);
    assert_eq!(b.ppu.framebuffer[16 * LCD_WIDTH + 16], Color::Black);
}

#[test]
fn writing_dma_again_restarts_with_the_new_source() {
    let mut b = MemoryBus::new();
    for i in 0..160 {
        b.write_byte(0xc000 + i, 0x11);
        b.write_byte(0xd000 + i, 0x22);
    }
    b.write_byte(0xff46, 0xc0);
    b.step(44);
    assert_eq!(b.ppu.oam[0], 0x11);
    b.write_byte(0xff46, 0xd0);
    b.step(644);
    assert!(b.ppu.oam.iter().all(|&byte| byte == 0x22));
}
