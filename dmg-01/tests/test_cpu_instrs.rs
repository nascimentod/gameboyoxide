// Exact-assertion tests for cpu_instrs.gb and dmg_sound.gb.
// Uses PC/register tracking for subtest returns and font-matching for screen results.
use dmg_01::gameboy::GameBoy;
use dmg_01::ppu::{Color, LCD_WIDTH, LCD_HEIGHT};

#[derive(Debug)]
enum ScreenResult {
    Pass,
    Fail(String),
    Unknown,
}

// dmg_sound: 0x23BD stores the first failing test number at D880;
// 0x25F2 prints "Passed" only if it is zero, otherwise 0x26F4 prints
// "Failed" or "Failed #". 0x281A expands the eight-byte font at 0x28CD
// (ASCII 0x20 onward) into VRAM; 0x28B5 writes codes to the 0x9800 map.
fn analyze_dmg_sound_screen(fb: &[Color; LCD_WIDTH * LCD_HEIGHT], font: &[u8]) -> ScreenResult {
    fn contains(fb: &[Color; LCD_WIDTH * LCD_HEIGHT], font: &[u8], text: &[u8]) -> bool {
        if font.len() < 0x300 { return false; }
        for row in 0..18 {
            for col in 0..=20 - text.len() {
                if text.iter().enumerate().all(|(i, &c)| {
                    let glyph = &font[(c as usize - 0x20) * 8..(c as usize - 0x20 + 1) * 8];
                    (0..8).all(|y| (0..8).all(|x| {
                        let black = fb[(row * 8 + y) * LCD_WIDTH + (col + i) * 8 + x] != Color::White;
                        black == (glyph[y] & (0x80 >> x) != 0)
                    }))
                }) { return true; }
            }
        }
        false
    }
    if contains(fb, font, b"Failed") {
        ScreenResult::Fail("dmg_sound reported Failed".to_string())
    } else if contains(fb, font, b"Passed") {
        ScreenResult::Pass
    } else {
        ScreenResult::Unknown
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn dmg_sound_result_glyphs() {
        let rom = std::fs::read("test-roms/dmg_sound.gb").unwrap();
        assert_eq!(&rom[0x25e5..0x25eb], b"Passed");
        assert_eq!(&rom[0x2702..0x2708], b"Failed");
        let font = &rom[0x28cd..0x2bcd];
        let render = |text: &[u8]| {
            let mut fb = [Color::White; LCD_WIDTH * LCD_HEIGHT];
            for (i, &c) in text.iter().enumerate() {
                for y in 0..8 {
                    for x in 0..8 {
                        if font[(c as usize - 32) * 8 + y] & (0x80 >> x) != 0 {
                            fb[(8 * 8 + y) * LCD_WIDTH + (3 + i) * 8 + x] = Color::Black;
                        }
                    }
                }
            }
            fb
        };
        assert!(matches!(analyze_dmg_sound_screen(&render(b"Passed"), font), ScreenResult::Pass));
        assert!(matches!(analyze_dmg_sound_screen(&render(b"Failed #"), font), ScreenResult::Fail(_)));
        assert!(matches!(analyze_dmg_sound_screen(&[Color::Black; LCD_WIDTH * LCD_HEIGHT], font), ScreenResult::Unknown));
        assert!(matches!(analyze_dmg_sound_screen(&render(b"ok"), font), ScreenResult::Unknown));
    }

    // A is the result returned by each banked subtest; zero means success.
    #[test]
    fn dmg_sound_subtests_pass() {
        let mut gb = GameBoy::new();
        gb.load_cartridge(std::fs::read("test-roms/dmg_sound.gb").unwrap());
        let mut results = Vec::new();
        for _ in 0..100_000_000 {
            if gb.cpu.pc == 0x233e {
                let number = gb.cpu.bus.read_byte(0xff85) + 1;
                let result = gb.cpu.registers.a;
                println!("subtest {number}: return {result}");
                assert_eq!(usize::from(number), results.len() + 1);
                results.push(result);
            }
            if gb.cpu.pc == 0x25f2 {
                assert_eq!(results, vec![0; 12], "one or more dmg_sound subtests failed");
                assert_eq!(gb.cpu.bus.read_byte(0xd880), 0);
                return;
            }
            gb.step();
        }
        panic!("dmg_sound did not reach aggregate result; PC={:04x}", gb.cpu.pc);
    }

    // End-to-end regression: the aggregate runner must leave its bank scan,
    // finish, and render a result consistent with its own result byte.
    #[test]
    fn dmg_sound_completes_and_reports_result() {
        let rom = std::fs::read("test-roms/dmg_sound.gb").unwrap();
        let font = rom[0x28cd..0x2bcd].to_vec();
        let mut gb = GameBoy::new();
        gb.load_cartridge(rom);
        let mut finished_scan = false;
        for step in 0..100_000_000 {
            if gb.cpu.pc == 0x2344 { finished_scan = true; }
            gb.step();
            if finished_scan && step % 100_000 == 0 {
                let status = gb.cpu.bus.read_byte(0xd880);
                match analyze_dmg_sound_screen(gb.get_framebuffer(), &font) {
                    ScreenResult::Pass => { assert_eq!(status, 0); return; }
                    ScreenResult::Fail(reason) => panic!("{reason}; first failing subtest {status}"),
                    _ => {}
                }
            }
        }
        panic!("dmg_sound did not complete and render a result; scan finished: {finished_scan}, PC={:04x}", gb.cpu.pc);
    }

}
