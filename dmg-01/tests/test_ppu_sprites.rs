use dmg_01::ppu::{Color, PPUMode, Palette, LCD_WIDTH, PPU};

fn ppu() -> PPU {
    let mut p = PPU::new();
    p.vram.fill(0);
    p.oam.fill(0);
    p.write_register(0xff40, 0x83); // LCD + OBJ + BG, signed BG tiles
    p.obp0 = Palette::from_byte(0xe4);
    p.obp1 = Palette::from_byte(0xfc);
    p
}
fn tile(p: &mut PPU, id: usize, color: u8) {
    for row in 0..8 {
        p.vram[id * 16 + row * 2] = if color & 1 != 0 { 0xff } else { 0 };
        p.vram[id * 16 + row * 2 + 1] = if color & 2 != 0 { 0xff } else { 0 };
    }
}
fn object(p: &mut PPU, index: usize, x: u8, y: u8, id: u8, flags: u8) {
    p.oam[index * 4..index * 4 + 4].copy_from_slice(&[y, x, id, flags]);
}
fn line(p: &mut PPU, y: u8) {
    p.ly = y;
    p.stat.mode = PPUMode::Drawing;
    p.mode_clock = 0;
    p.step(172);
}
fn pixel(p: &PPU, x: usize, y: usize) -> Color {
    p.framebuffer[y * LCD_WIDTH + x]
}

#[test]
fn sprites_always_use_unsigned_8000_tiles_even_with_signed_background() {
    let mut p = ppu();
    tile(&mut p, 1, 3);
    object(&mut p, 0, 24, 32, 1, 0);
    line(&mut p, 16);
    assert_eq!(pixel(&p, 16, 16), Color::Black);
    assert_eq!(pixel(&p, 24, 16), Color::White);
}

#[test]
fn eight_by_sixteen_sprites_ignore_low_tile_bit_and_flip_both_halves() {
    let mut p = ppu();
    p.lcdc.sprite_size = true;
    tile(&mut p, 2, 1);
    tile(&mut p, 3, 2);
    object(&mut p, 0, 8, 16, 3, 0);
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::LightGray);
    line(&mut p, 8);
    assert_eq!(pixel(&p, 0, 8), Color::DarkGray);
    p.oam[3] = 0x40;
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::DarkGray);
    line(&mut p, 8);
    assert_eq!(pixel(&p, 0, 8), Color::LightGray);
}

#[test]
fn partially_offscreen_sprites_clip_instead_of_wrapping() {
    let mut p = ppu();
    tile(&mut p, 1, 3);
    object(&mut p, 0, 4, 12, 1, 0); // x=-4, y=-4
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::Black);
    assert_eq!(pixel(&p, 3, 0), Color::Black);
    assert_eq!(pixel(&p, 4, 0), Color::White);
}

#[test]
fn smaller_x_then_lower_oam_index_wins_overlap() {
    let mut p = ppu();
    tile(&mut p, 1, 1);
    tile(&mut p, 2, 2);
    object(&mut p, 0, 10, 16, 1, 0);
    object(&mut p, 1, 8, 16, 2, 0);
    line(&mut p, 0);
    assert_eq!(pixel(&p, 2, 0), Color::DarkGray);
    p.oam[1] = 8;
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::LightGray);
}

#[test]
fn transparent_object_pixels_do_not_hide_lower_priority_objects() {
    let mut p = ppu();
    tile(&mut p, 2, 2);
    object(&mut p, 0, 8, 16, 1, 0);
    object(&mut p, 1, 8, 16, 2, 0);
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::DarkGray);
}

#[test]
fn first_ten_y_matching_oam_entries_are_selected_before_x_sorting() {
    let mut p = ppu();
    tile(&mut p, 1, 3);
    for i in 0..10 {
        object(&mut p, i, 0, 16, 1, 0);
    } // hidden X still consumes a slot
    object(&mut p, 10, 8, 16, 1, 0);
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::White);
    // First ten still count even when they are further right than object 10.
    for i in 0..10 {
        p.oam[i * 4 + 1] = 160;
    }
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::White);
}

#[test]
fn winning_object_behind_background_does_not_reveal_lower_priority_object() {
    let mut p = ppu();
    tile(&mut p, 0x100, 1); // signed BG tile 0 at 9000
    tile(&mut p, 1, 2);
    tile(&mut p, 2, 3);
    object(&mut p, 0, 8, 16, 1, 0x80);
    object(&mut p, 1, 8, 16, 2, 0);
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::LightGray);
}

#[test]
fn horizontal_flip_and_palette_one_are_applied_to_sprite_pixels() {
    let mut p = ppu();
    p.vram[16] = 0x80; // tile 1 first row: one pixel at left
    object(&mut p, 0, 8, 16, 1, 0x30);
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::White);
    assert_eq!(pixel(&p, 7, 0), Color::Black);
}

#[test]
fn disabling_background_clears_old_sprite_pixels_each_scanline() {
    let mut p = ppu();
    p.lcdc.bg_window_enable = false;
    tile(&mut p, 1, 3);
    object(&mut p, 0, 8, 16, 1, 0);
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::Black);
    p.oam.fill(0);
    line(&mut p, 0);
    assert_eq!(pixel(&p, 0, 0), Color::White);
}
