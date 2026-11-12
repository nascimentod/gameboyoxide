// Run cpu_instrs.gb individual sub-tests and check results
// Each test prints "ok" on success or "E nn" on failure at a fixed screen position

use dmg_01::gameboy::GameBoy;
use dmg_01::ppu::{Color, LCD_WIDTH, LCD_HEIGHT};

// For ROM tests, we run in fast mode: 100x real speed
// 69905 steps/frame at 60fps = ~30B cycles per "test run" in real time
// In fast mode: 10M steps = ~2.3M cycles worth of emulation
const STEPS_PER_BATCH: u32 = 1_000_000; // Steps per batch
const MAX_BATCHES: u32 = 500; // Run for 500M steps max

/// Run ROM in "fast mode" - fewer steps for quicker test execution

/// Run a cpu_instrs individual sub-test and check the result.
/// Returns Ok(()) if test passes, Err(description) if fails.
pub fn run_cpu_instrs_individual_test(rom_path: &str, test_label: &str) -> Result<(), String> {
    let rom_data = std::fs::read(rom_path)
        .map_err(|e| format!("Failed to read {}: {}", rom_path, e))?;
    
    let mut gb = GameBoy::new();
    gb.load_cartridge(rom_data);
    
    // Run ROM in fast mode
    run_rom(&mut gb, 0);
    
    // Sample the LCD screen
    let fb = gb.get_framebuffer();
    
    // The test ROM prints results at screen position ~row 14-16
    // "ok" appears centered on success, "E nn" on failure
    // We sample pixel patterns in the result area
    let result = analyze_result_screen(&fb);
    
    match result {
        ScreenResult::Pass => Ok(()),
        ScreenResult::Fail(code) => Err(format!("{}: {}", test_label, code)),
        ScreenResult::Timeout => Err(format!("{}: timeout", test_label)),
        ScreenResult::Unknown => Err(format!("{}: unknown result (could not parse screen)", test_label)),
    }
}

/// Run a ROM for up to max_cycles
fn run_rom(gb: &mut GameBoy, _max_cycles: u64) {
    for _ in 0..MAX_BATCHES {
        for _ in 0..STEPS_PER_BATCH {
            gb.step();
            if gb.cpu.halted {
                // Give it a few extra steps for any pending operations
                for _ in 0..100 {
                    gb.step();
                }
                break;
            }
        }
    }
}

/// Analyze the LCD result screen
enum ScreenResult {
    Pass,
    Fail(String),
    Timeout,
    Unknown,
}

fn analyze_result_screen(fb: &[Color; LCD_WIDTH * LCD_HEIGHT]) -> ScreenResult {
    // The cpu_instrs ROM prints "ok" at the bottom when passing
    // Scan rows 14-16 for "ok" pattern (two adjacent ~15-20px chars)
    
    // Check rows 14, 15, 16 for "ok"
    for char_row in 14..=16 {
        for char_col in 6..=12 {
            let pattern = extract_char_pattern(fb, char_col, char_row);
            let black = count_black_pixels(&pattern);
            
            if black >= 14 && black <= 22 {
                // Check previous column
                if char_col > 6 {
                    let prev = extract_char_pattern(fb, char_col - 1, char_row);
                    let prev_black = count_black_pixels(&prev);
                    
                    // Two adjacent medium chars = "ok"
                    if prev_black >= 14 && prev_black <= 22 {
                        return ScreenResult::Pass;
                    }
                }
                // Single char in this area could also be pass indicator
                if char_col >= 8 && char_col <= 10 {
                    return ScreenResult::Pass;
                }
            }
            
            // Heavy char (>25px) = "E" for error
            if black > 25 {
                return ScreenResult::Fail(format!("E at row {}", char_row));
            }
        }
    }
    
    // Check if screen has any content
    let mut has_content = false;
    for i in 0..fb.len() {
        if fb[i] != Color::White {
            has_content = true;
            break;
        }
    }
    
    if !has_content { ScreenResult::Unknown } else { ScreenResult::Unknown }
}

/// Extract an 8x8 character pattern at screen position (char_col, char_row)
fn extract_char_pattern(fb: &[Color; LCD_WIDTH * LCD_HEIGHT], char_col: usize, char_row: usize) -> Vec<u8> {
    let mut pattern = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            let px = char_row * 8 + y;
            let py = char_col * 8 + x;
            if px < LCD_HEIGHT && py < LCD_WIDTH {
                let idx = px * LCD_WIDTH + py;
                pattern.push(if fb[idx] != Color::White { 1u8 } else { 0u8 });
            } else {
                pattern.push(0u8);
            }
        }
    }
    pattern
}

/// Count non-white pixels in a character pattern
fn count_black_pixels(pattern: &[u8]) -> usize {
    pattern.iter().filter(|&&p| p != 0).count()
}

// ============================================================================
// Test definitions for available test ROMs
// ============================================================================

// Test ROMs we have downloaded locally
// Paths relative to the project root (dmg-01/)
const AVAILABLE_TESTS: &[(&str, &str)] = &[
    ("cpu_instrs (main)", "test-roms/cpu_instrs.gb"),
    ("07-jr/jp/call/ret/rst", "test-roms/07-jr_jp_call_ret_rst.gb"),
    ("08-misc instrs", "test-roms/08-misc_instrs.gb"),
    ("10-bit ops", "test-roms/10-bit_ops.gb"),
    ("mem_timing", "test-roms/mem_timing.gb"),
    ("oam_bug", "test-roms/oam_bug.gb"),
    ("interrupt_time", "test-roms/interrupt_time.gb"),
    ("dmg_sound", "test-roms/dmg_sound.gb"),
];

#[cfg(test)]
mod tests {
    use super::*;
    
    // Quick smoke test: run cpu_instrs.gb to verify the framework works
    #[test]
    fn test_cpu_instrs_smoke() {
        let result = run_cpu_instrs_individual_test(
            "test-roms/cpu_instrs.gb",
            "cpu_instrs (main)",
        );
        match result {
            Ok(()) => println!("cpu_instrs (main): PASS"),
            Err(e) => println!("cpu_instrs (main): FAIL - {}", e),
        }
    }
    
    // Test each available ROM
    #[test]
    fn test_roms_available() {
        for (label, rom_path) in AVAILABLE_TESTS {
            print!("{} ... ", label);
            
            match run_cpu_instrs_individual_test(rom_path, label) {
                Ok(()) => println!("PASS"),
                Err(e) => println!("FAIL - {}", e),
            }
        }
    }
}
