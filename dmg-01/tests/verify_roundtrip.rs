// Round-trip verification tests
// Verify that instruction decoding and execution are deterministic and consistent

use dmg_01::cpu::CPU;
use dmg_01::instructions::*;
use proptest::prelude::*;

fn make_cpu() -> CPU {
    let mut cpu = CPU::new();
    cpu.pc = 0xC000;
    for i in 0x4000..0x8000 {
        cpu.bus.memory[i] = 0;
    }
    cpu
}

// ============================================================================
// Determinism tests: same input → same output
// ============================================================================

#[test]
fn test_add_determinism() {
    let mut cpu1 = make_cpu();
    let mut cpu2 = make_cpu();
    
    cpu1.registers.a = 0x42;
    cpu2.registers.a = 0x42;
    cpu1.registers.b = 0x13;
    cpu2.registers.b = 0x13;
    
    cpu1.add(0x13);
    cpu2.add(0x13);
    
    assert_eq!(cpu1.registers.a, cpu2.registers.a);
    assert_eq!(cpu1.registers.f.zero, cpu2.registers.f.zero);
    assert_eq!(cpu1.registers.f.carry, cpu2.registers.f.carry);
    assert_eq!(cpu1.registers.f.half_carry, cpu2.registers.f.half_carry);
}

#[test]
fn test_sub_determinism() {
    let mut cpu1 = make_cpu();
    let mut cpu2 = make_cpu();
    
    cpu1.registers.a = 0x10;
    cpu2.registers.a = 0x10;
    cpu1.registers.b = 0x05;
    cpu2.registers.b = 0x05;
    
    cpu1.sub(0x05);
    cpu2.sub(0x05);
    
    assert_eq!(cpu1.registers.a, cpu2.registers.a);
    assert_eq!(cpu1.registers.f.zero, cpu2.registers.f.zero);
    assert_eq!(cpu1.registers.f.carry, cpu2.registers.f.carry);
}

// ============================================================================
// Instruction decoding round-trip
// ============================================================================

#[test]
fn test_nop_roundtrip() {
    let mut cpu = make_cpu();
    cpu.pc = 0xC000;
    cpu.bus.memory[0xC000] = 0x00;
    
    let before_pc = cpu.pc;
    let before_sp = cpu.sp;
    let before_ime = cpu.ime;
    
    let instruction = cpu.fetch_decode();
    assert!(matches!(instruction, Instruction::NOP));
    assert_eq!(cpu.pc, before_pc + 1);
    
    cpu.execute(instruction);
    assert_eq!(cpu.pc, before_pc + 1); // PC unchanged after execute
    assert_eq!(cpu.sp, before_sp);
    assert_eq!(cpu.ime, before_ime);
}

#[test]
fn test_ld_d8_roundtrip() {
    let mut cpu = make_cpu();
    cpu.pc = 0xC000;
    cpu.bus.memory[0xC000] = 0x3E; // LD A, d8
    cpu.bus.memory[0xC001] = 0x42;
    
    let before_pc = cpu.pc;
    
    let instruction = cpu.fetch_decode();
    assert!(matches!(instruction, Instruction::LD(LoadTarget::AFromD8)));
    assert_eq!(cpu.pc, before_pc + 1); // Only advanced past opcode
    
    cpu.execute(instruction);
    assert_eq!(cpu.registers.a, 0x42, "LD A,d8 should set A to the immediate value");
    
    // Execute again - should load same value
    cpu.registers.a = 0x00;
    cpu.pc = 0xC000;
    let instruction = cpu.fetch_decode();
    cpu.execute(instruction);
    assert_eq!(cpu.registers.a, 0x42, "Second execution should also load 0x42");
}

// ============================================================================
// 16-bit LD round-trips
// ============================================================================

#[test]
fn test_ld_bc_d16_roundtrip() {
    let mut cpu = make_cpu();
    cpu.pc = 0xC000;
    cpu.bus.memory[0xC000] = 0x01; // LD BC, d16
    cpu.bus.memory[0xC001] = 0x34;
    cpu.bus.memory[0xC002] = 0x12;
    
    let instruction = cpu.fetch_decode();
    assert!(matches!(instruction, Instruction::LD(LoadTarget::BCFromD16)));
    assert_eq!(cpu.pc, 0xC001); // After fetch: opcode byte consumed
    
    cpu.execute(instruction);
    assert_eq!(cpu.registers.get_bc(), 0x1234, "BC should be 0x1234");
    assert_eq!(cpu.pc, 0xC003); // After execute: PC advanced past all 3 bytes
}

#[test]
fn test_ld_sp_d16_roundtrip() {
    let mut cpu = make_cpu();
    cpu.sp = 0x0000;
    cpu.pc = 0xC000;
    cpu.bus.memory[0xC000] = 0x31; // LD SP, d16
    cpu.bus.memory[0xC001] = 0xAB;
    cpu.bus.memory[0xC002] = 0xCD;
    
    let instruction = cpu.fetch_decode();
    cpu.execute(instruction);
    assert_eq!(cpu.sp, 0xCDAB);
}

// ============================================================================
// ADD HL round-trips
// ============================================================================

#[test]
fn test_add_hl_bc_roundtrip() {
    let mut cpu = make_cpu();
    cpu.registers.set_hl(0x1234);
    cpu.registers.set_bc(0x5678);
    
    let hl_val = cpu.registers.get_hl();
    let bc_val = cpu.registers.get_bc();
    
    // Expected result
    let result = (hl_val as u32) + (bc_val as u32);
    let expected_hl = (result & 0xFFFF) as u16;
    let expected_carry = result > 0xFFFF;
    let expected_half = ((hl_val & 0x0FFF) as u32) + ((bc_val & 0x0FFF) as u32) > 0x0FFF;
    
    cpu.add_hl(AddHLTarget::BC);
    
    assert_eq!(cpu.registers.get_hl(), expected_hl, "ADD HL,BC result");
    assert_eq!(cpu.registers.f.carry, expected_carry, "ADD HL,BC carry");
    assert_eq!(cpu.registers.f.half_carry, expected_half, "ADD HL,BC half_carry");
    assert!(!cpu.registers.f.subtract, "ADD HL clears subtract flag");
}

// ============================================================================
// Jump/Call/CALL round-trips
// ============================================================================

#[test]
fn test_unconditional_jump_roundtrip() {
    let mut cpu = make_cpu();
    cpu.pc = 0xC000;
    cpu.bus.memory[0xC000] = 0xE9; // JP HL
    
    let hl_val = 0x1234;
    cpu.registers.set_hl(hl_val);
    
    let instruction = cpu.fetch_decode();
    assert!(matches!(instruction, Instruction::JPHL));
    
    cpu.execute(instruction);
    assert_eq!(cpu.pc, hl_val);
}

#[test]
fn test_jr_relative_roundtrip() {
    let mut cpu = make_cpu();
    cpu.pc = 0xC010;
    cpu.bus.memory[0xC010] = 0x18; // JR e
    cpu.bus.memory[0xC011] = 0x05; // +5 relative offset
    
    let instruction = cpu.fetch_decode();
    assert!(matches!(instruction, Instruction::JR(JumpCondition::Always)));
    assert_eq!(cpu.pc, 0xC011); // After fetch: past opcode byte
    
    // execute reads offset byte, advances PC, then applies offset
    // PC after fetch = 0xC011
    // execute: read offset at 0xC011 (0x05), PC = 0xC012, then PC = 0xC012 + 5 = 0xC017
    cpu.execute(instruction);
    assert_eq!(cpu.pc, 0xC017, "JR should jump to PC after fetch + 1 + offset");
}

// ============================================================================
// Stack consistency after PUSH/POP
// ============================================================================

#[test]
fn test_push_pop_stack_consistency() {
    let mut cpu = make_cpu();
    cpu.sp = 0xFFFE;
    
    // Push HL, then push BC
    cpu.registers.set_hl(0x1234);
    cpu.registers.set_bc(0x5678);
    
    cpu.push(StackTarget::HL);
    let sp_after_hl = cpu.sp;
    cpu.push(StackTarget::BC);
    let sp_after_bc = cpu.sp;
    
    // Stack should be: [BC_low] at sp_after_bc, [BC_high] at sp_after_bc+1,
    // [HL_low] at sp_after_hl, [HL_high] at sp_after_hl+1
    let bc_low = cpu.bus.read_byte(cpu.sp);
    let bc_high = cpu.bus.read_byte(cpu.sp + 1);
    let bc_read = ((bc_high as u16) << 8) | (bc_low as u16);
    
    assert_eq!(bc_read, 0x5678, "BC value on stack");
    
    // Pop BC - should restore original value
    cpu.pop(StackTarget::BC);
    assert_eq!(cpu.registers.get_bc(), 0x5678);
    assert_eq!(cpu.sp, sp_after_hl);
    
    // Pop HL - should restore original value
    cpu.pop(StackTarget::HL);
    assert_eq!(cpu.registers.get_hl(), 0x1234);
    assert_eq!(cpu.sp, 0xFFFE);
}

// ============================================================================
// Carry flag propagation through multiple operations
// ============================================================================

#[test]
fn test_carry_chain() {
    let mut cpu = make_cpu();
    
    // Start with carry set
    cpu.registers.f.carry = true;
    
    // ADD A, 0 should keep carry based on overflow
    cpu.registers.a = 0xFF;
    cpu.add(0x01);
    assert!(cpu.registers.f.carry, "0xFF + 0x01 should set carry");
    
    // Reset carry
    cpu.registers.f.carry = false;
    cpu.registers.a = 0xFF;
    
    // ADD A, 0 should not set carry
    cpu.add(0x00);
    assert!(!cpu.registers.f.carry, "0xFF + 0x00 should not set carry");
}

// ============================================================================
// Flag independence: one operation doesn't affect unrelated flags
// ============================================================================

#[test]
fn test_flag_independence() {
    let mut cpu = make_cpu();
    
    // Set Z flag
    cpu.registers.a = 0x00;
    cpu.registers.f.zero = true;
    cpu.registers.f.subtract = false;
    cpu.registers.f.half_carry = false;
    cpu.registers.f.carry = false;
    
    // ADD should only modify Z, S, H, C - not affect other state
    let pc_before = cpu.pc;
    let sp_before = cpu.sp;
    let ime_before = cpu.ime;
    
    cpu.registers.a = 0x01;
    cpu.add(0x01);
    
    assert_eq!(cpu.pc, pc_before, "ADD shouldn't change PC");
    assert_eq!(cpu.sp, sp_before, "ADD shouldn't change SP");
    assert_eq!(cpu.ime, ime_before, "ADD shouldn't change IME");
}
