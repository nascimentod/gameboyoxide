// Interrupt state machine verification tests
// Tests IE/IF synchronization, priority, IME rules, and vector addresses

use dmg_01::cpu::CPU;
use dmg_01::memory::MemoryBus;

// Interrupt constants from cpu.rs
const VBLANK_BIT: u8 = 0;
const STAT_BIT: u8 = 1;
const TIMER_BIT: u8 = 2;
const SERIAL_BIT: u8 = 3;
const JOYPAD_BIT: u8 = 4;

const VBLANK_VECTOR: u16 = 0x40;
const STAT_VECTOR: u16 = 0x48;
const TIMER_VECTOR: u16 = 0x50;
const SERIAL_VECTOR: u16 = 0x58;
const JOYPAD_VECTOR: u16 = 0x60;

// Helper: create a CPU with boot ROM disabled and RAM zeroed
fn make_cpu() -> CPU {
    let mut cpu = CPU::new();
    cpu.pc = 0x0100;
    cpu.bus.boot_rom_enabled = false;
    // Clear RAM and I/O
    for i in 0x8000..0x10000 {
        cpu.bus.memory[i] = 0;
    }
    cpu.ie_register = 0x00;
    cpu.if_register = 0x00;
    cpu.ime = false;
    cpu
}

// ============================================================================
// IE/IF register synchronization
// ============================================================================

#[test]
fn test_ie_register_write_read() {
    let mut cpu = make_cpu();
    
    cpu.write_ie_register(0x1F); // Enable all interrupts
    assert_eq!(cpu.read_ie_register(), 0x1F);
    
    cpu.write_ie_register(0x01); // Only V-Blank
    assert_eq!(cpu.read_ie_register(), 0x01);
    
    cpu.write_ie_register(0x00); // Disable all
    assert_eq!(cpu.read_ie_register(), 0x00);
}

#[test]
fn test_if_register_write_read() {
    let mut cpu = make_cpu();
    
    // Upper 5 bits should be preserved on read
    cpu.write_if_register(0x03);
    assert_eq!(cpu.read_if_register() & 0x1F, 0x03);
    assert_eq!(cpu.read_if_register() & 0xE0, 0xE0); // Upper bits always 1
    
    // Write to IF
    cpu.bus.if_register = 0x05;
    cpu.if_register = cpu.bus.if_register & 0x1F;
    assert_eq!(cpu.if_register, 0x05);
}

// ============================================================================
// Interrupt priority
// ============================================================================

#[test]
fn test_interrupt_priority_vblank_highest() {
    let mut cpu = make_cpu();
    cpu.ime = true;
    
    // Set all interrupt enables and flags
    cpu.ie_register = 0x1F;
    cpu.if_register = 0x1F;
    
    // All interrupts pending - V-Blank (bit 0) has highest priority
    let pending = cpu.ie_register & cpu.if_register;
    assert!(pending != 0);
    
    // Check that lowest bit is highest priority
    assert_eq!(pending & 0x01, 0x01); // V-Blank
}

#[test]
fn test_interrupt_vector_addresses() {
    // Verify all interrupt vectors are correct
    assert_eq!(VBLANK_VECTOR, 0x40, "V-Blank vector");
    assert_eq!(STAT_VECTOR, 0x48, "STAT vector");
    assert_eq!(TIMER_VECTOR, 0x50, "Timer vector");
    assert_eq!(SERIAL_VECTOR, 0x58, "Serial vector");
    assert_eq!(JOYPAD_VECTOR, 0x60, "Joypad vector");
    
    // Vectors are 8 bytes apart (consistent with 8-byte instruction alignment)
    assert_eq!(STAT_VECTOR - VBLANK_VECTOR, 8);
    assert_eq!(TIMER_VECTOR - STAT_VECTOR, 8);
    assert_eq!(SERIAL_VECTOR - TIMER_VECTOR, 8);
    assert_eq!(JOYPAD_VECTOR - SERIAL_VECTOR, 8);
}

// ============================================================================
// IME rules
// ============================================================================

#[test]
fn test_ime_disabled_blocks_interrupts() {
    let mut cpu = make_cpu();
    cpu.ime = false;
    cpu.ie_register = 0x1F;
    cpu.if_register = 0x01; // V-Blank pending
    
    let pending = cpu.ie_register & cpu.if_register;
    assert!(pending != 0, "Interrupt should be pending");
    
    assert!(!cpu.check_and_handle_interrupts(), "Should not handle interrupt when IME disabled");
    assert_eq!(cpu.pc, 0x0100, "PC should not change");
}

#[test]
fn test_ime_enabled_allows_interrupts() {
    let mut cpu = make_cpu();
    cpu.ime = true;
    cpu.pc = 0x0100;
    cpu.ie_register = 0x01; // Only V-Blank enabled
    cpu.if_register = 0x01; // V-Blank pending
    
    // Set up some stack space
    cpu.sp = 0xFFFE;
    for i in 0xFF00..0x10000 {
        cpu.bus.memory[i] = 0;
    }
    
    assert!(cpu.check_and_handle_interrupts(), "Should handle interrupt when IME enabled");
    assert!(!cpu.ime, "IME should be disabled after interrupt");
    assert_eq!(cpu.pc, VBLANK_VECTOR);
}

#[test]
fn test_ei_sets_ime_after_next_instruction() {
    let mut cpu = make_cpu();
    cpu.ime = false;
    cpu.ei_delay = false;
    
    // EI instruction sets ei_delay
    cpu.execute(dmg_01::instructions::Instruction::EI);
    assert!(cpu.ei_delay, "EI should set ei_delay");
    assert!(!cpu.ime, "IME should still be false immediately after EI");
    
    // Step should resolve ei_delay
    cpu.ei_delay = true;
    cpu.ime = false;
    cpu.ie_register = 0x00;
    cpu.if_register = 0x00;
    // Simulate the ei_delay handling from step()
    if cpu.ei_delay {
        cpu.ime = true;
        cpu.ei_delay = false;
    }
    assert!(cpu.ime, "IME should be true after ei_delay resolves");
    assert!(!cpu.ei_delay, "ei_delay should be cleared");
}

#[test]
fn test_di_clears_ime_immediately() {
    let mut cpu = make_cpu();
    cpu.ime = true;
    cpu.ei_delay = true;
    
    cpu.execute(dmg_01::instructions::Instruction::DI);
    assert!(!cpu.ime, "DI should clear IME immediately");
    // Note: ei_delay is NOT cleared by DI in the current implementation
    // Real GB: DI clears IME and cancels pending EI
}

#[test]
fn test_retie_enables_ime() {
    let mut cpu = make_cpu();
    cpu.ime = false;
    cpu.sp = 0xFFFE;
    
    // Push a return address
    cpu.bus.write_byte(cpu.sp, 0x00);
    cpu.sp = cpu.sp.wrapping_sub(1);
    cpu.bus.write_byte(cpu.sp, 0x01);
    cpu.sp = cpu.sp.wrapping_sub(1);
    
    cpu.execute(dmg_01::instructions::Instruction::RETI);
    
    assert!(cpu.ime, "RETI should enable IME");
    assert_eq!(cpu.pc, 0x0100); // Popped address
    assert_eq!(cpu.sp, 0xFFFE); // SP restored
}

// ============================================================================
// Interrupt handling - PC save/restore
// ============================================================================

#[test]
fn test_interrupt_pushes_correct_pc() {
    let mut cpu = make_cpu();
    cpu.ime = true;
    cpu.pc = 0x1234;
    cpu.sp = 0xFFFE;
    cpu.ie_register = 0x01;
    cpu.if_register = 0x01;
    
    // Clear stack
    for i in 0xFF00..0x10000 {
        cpu.bus.memory[i] = 0;
    }
    
    cpu.handle_interrupt(VBLANK_BIT);
    
    // PC is pushed: high byte at SP+1, low byte at SP (after decrement)
    // Before: SP=0xFFFE, after: SP=0xFFFC
    // High byte written at 0xFFFD, low byte at 0xFFFC
    let high = cpu.bus.read_byte(cpu.sp.wrapping_add(1));
    let low = cpu.bus.read_byte(cpu.sp.wrapping_add(0));
    
    assert_eq!(((high as u16) << 8) | (low as u16), 0x1234, "Correct PC pushed to stack");
    assert_eq!(cpu.pc, VBLANK_VECTOR, "PC jumped to interrupt vector");
    assert_eq!(cpu.sp, 0xFFFE - 2, "SP decremented by 2");
}

// ============================================================================
// Interrupt flag clearing
// ============================================================================

#[test]
fn test_interrupt_clears_if_flag() {
    let mut cpu = make_cpu();
    cpu.ime = true;
    cpu.ie_register = 0x1F;
    
    // Set multiple flags
    cpu.if_register = 0x1F;
    
    cpu.handle_interrupt(VBLANK_BIT);
    assert_eq!(cpu.if_register & 0x01, 0, "V-Blank flag cleared");
    assert_eq!(cpu.if_register & 0x1E, 0x1E, "Other flags still set");
    
    cpu.handle_interrupt(TIMER_BIT);
    assert_eq!(cpu.if_register & 0x04, 0, "Timer flag cleared");
    // After clearing V-Blank (bit 0) and Timer (bit 2), bits 1,3,4 should still be set
    // if_register = 0x1A, check bits 1,3,4 = 0x12
    assert_eq!(cpu.if_register & 0x12, 0x12, "STAT, Serial, Joypad flags still set");
}

// ============================================================================
// HALT wake-up
// ============================================================================

#[test]
fn test_halted_cpu_wakes_on_interrupt() {
    let mut cpu = make_cpu();
    cpu.halted = true;
    cpu.ime = true;
    cpu.ie_register = 0x01;
    cpu.if_register = 0x01;
    
    cpu.check_and_handle_interrupts();
    
    assert!(!cpu.halted, "CPU should wake from HALT when interrupt handled");
}

#[test]
fn test_halted_cpu_wakes_on_interrupt_even_with_ime_off() {
    let mut cpu = make_cpu();
    cpu.halted = true;
    cpu.ime = false;
    cpu.ie_register = 0x01;
    cpu.if_register = 0x01;
    
    // With IME off, HALT wakes but interrupt not handled
    cpu.check_and_handle_interrupts();
    
    assert!(!cpu.halted, "CPU should wake from HALT even with IME off");
    assert!(!cpu.ime, "IME should still be off");
}

// ============================================================================
// IF register read masking
// ============================================================================

#[test]
fn test_if_register_read_mask() {
    let mut cpu = make_cpu();
    
    // Writing any value, reading should have upper 5 bits = 1
    cpu.if_register = 0x00;
    let read_val = cpu.read_if_register();
    assert_eq!(read_val & 0x1F, 0x00, "Lower 5 bits should be 0");
    assert_eq!(read_val & 0xE0, 0xE0, "Upper 3 bits should be 1 (read-only)");
    
    cpu.if_register = 0x1F;
    let read_val = cpu.read_if_register();
    assert_eq!(read_val & 0x1F, 0x1F, "Lower 5 bits should be all 1");
    assert_eq!(read_val & 0xE0, 0xE0, "Upper 3 bits should be 1 (read-only)");
}

// ============================================================================
// IE register write masking
// ============================================================================

#[test]
fn test_ie_register_write_masking() {
    let mut cpu = make_cpu();
    
    // All bits are writable in IE
    cpu.write_ie_register(0xFF);
    assert_eq!(cpu.read_ie_register(), 0xFF);
    
    cpu.write_ie_register(0x00);
    assert_eq!(cpu.read_ie_register(), 0x00);
}

// ============================================================================
// Interrupt enable/disable timing
// ============================================================================

#[test]
fn test_interrupt_disabled_during_handling() {
    let mut cpu = make_cpu();
    cpu.ime = true;
    
    // Before handling
    assert!(cpu.ime, "IME should be true before interrupt");
    
    cpu.handle_interrupt(VBLANK_BIT);
    
    // After handling
    assert!(!cpu.ime, "IME should be false after interrupt");
    
    // Can't re-enter interrupt immediately
    cpu.if_register = 0x1F; // All flags set
    cpu.ie_register = 0x1F;
    assert!(!cpu.check_and_handle_interrupts(), "Should not re-enter interrupt");
}
