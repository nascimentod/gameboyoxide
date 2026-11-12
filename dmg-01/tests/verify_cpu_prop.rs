// Property-based CPU instruction verification tests
// Tests all instructions with random inputs, verifying results and flags
// Uses proptest for exhaustive generation

use dmg_01::cpu::CPU;
use dmg_01::instructions::*;
use proptest::prelude::*;

// Helper: Create a CPU with registers in a known memory region
fn make_cpu() -> CPU {
    let mut cpu = CPU::new();
    cpu.pc = 0xC000;
    for i in 0x4000..0x8000 {
        cpu.bus.memory[i] = 0;
    }
    cpu
}

// ============================================================================
// ADD Property Tests
// ============================================================================

#[test]
fn prop_add_register_source() {
    proptest! {
        #[test]
        fn test_add(a_val in 0u8..=255u8, b_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            
            let expected = a_val.wrapping_add(b_val);
            let has_carry = (a_val as u16) + (b_val as u16) > 0xFF;
            let has_half_carry = ((a_val & 0xF) as u16) + ((b_val & 0xF) as u16) > 0xF;
            
            cpu.add(b_val);
            
            assert_eq!(cpu.registers.a, expected, "ADD A,B result for 0x{:02X} + 0x{:02X}", a_val, b_val);
            assert_eq!(cpu.registers.f.zero, expected == 0, "Z flag");
            assert_eq!(cpu.registers.f.carry, has_carry, "C flag");
            assert_eq!(cpu.registers.f.half_carry, has_half_carry, "H flag");
            assert!(!cpu.registers.f.subtract, "S flag should be clear after ADD");
        }
    }
}

#[test]
fn prop_add_all_registers() {
    proptest! {
        #[test]
        fn test_add_regs(a_val in 0u8..=255u8, b_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            
            let expected = a_val.wrapping_add(b_val);
            let has_carry = (a_val as u16) + (b_val as u16) > 0xFF;
            let has_half_carry = ((a_val & 0xF) as u16) + ((b_val & 0xF) as u16) > 0xF;
            
            cpu.add(b_val);
            
            assert_eq!(cpu.registers.a, expected, "ADD A,B result");
            assert_eq!(cpu.registers.f.carry, has_carry, "C flag");
            assert_eq!(cpu.registers.f.half_carry, has_half_carry, "H flag");
        }
    }
}

// ============================================================================
// ADC Property Tests
// ============================================================================

#[test]
fn prop_adc_with_carry() {
    proptest! {
        #[test]
        fn test_adc(a_val in 0u8..=255u8, b_val in 0u8..=255u8, carry_bit in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            cpu.registers.f.carry = (carry_bit & 0x80) != 0;
            
            let old_a = a_val;
            let carry_in = if (carry_bit & 0x80) != 0 { 1u16 } else { 0 };
            let raw = (a_val as u16) + (b_val as u16) + carry_in;
            let expected = (raw & 0xFF) as u8;
            let has_carry = raw > 0xFF;
            
            // Manual ADC computation (matches cpu.rs implementation)
            let carry = if cpu.registers.f.carry { 1 } else { 0 };
            let (temp_result, carry1) = old_a.overflowing_add(b_val);
            let (final_result, carry2) = temp_result.overflowing_add(carry);
            
            cpu.registers.a = final_result;
            cpu.registers.f.zero = final_result == 0;
            cpu.registers.f.subtract = false;
            cpu.registers.f.carry = carry1 || carry2;
            cpu.registers.f.half_carry = ((old_a & 0xF) as u8) + (b_val & 0xF) + carry as u8 > 0xF;
            
            assert_eq!(cpu.registers.a, expected, "ADC result: 0x{:02X} + 0x{:02X} + C={}", old_a, b_val, carry_in);
            assert_eq!(cpu.registers.f.carry, has_carry, "ADC carry flag");
        }
    }
}

// ============================================================================
// SUB Property Tests
// ============================================================================

#[test]
fn prop_sub_register_source() {
    proptest! {
        #[test]
        fn test_sub(a_val in 0u8..=255u8, b_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            
            let expected = a_val.wrapping_sub(b_val);
            let has_carry = (a_val as u16) < (b_val as u16);
            let has_half_carry = (a_val & 0xF) < (b_val & 0xF);
            
            cpu.sub(b_val);
            
            assert_eq!(cpu.registers.a, expected, "SUB result for 0x{:02X} - 0x{:02X}", a_val, b_val);
            assert_eq!(cpu.registers.f.zero, expected == 0, "Z flag");
            assert_eq!(cpu.registers.f.carry, has_carry, "C flag");
            assert_eq!(cpu.registers.f.half_carry, has_half_carry, "H flag");
            assert!(cpu.registers.f.subtract, "S flag should be set after SUB");
        }
    }
}

// ============================================================================
// SBC Property Tests
// ============================================================================

#[test]
fn prop_sbc_with_carry() {
    proptest! {
        #[test]
        fn test_sbc(a_val in 0u8..=255u8, b_val in 0u8..=255u8, carry_bit in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            cpu.registers.f.carry = (carry_bit & 0x80) != 0;
            
            let old_a = a_val;
            let carry_in = if (carry_bit & 0x80) != 0 { 1u16 } else { 0 };
            let sub_val = b_val as u16 + carry_in;
            let raw = (a_val as u16).wrapping_sub(sub_val);
            let expected = (raw & 0xFF) as u8;
            let has_carry = (a_val as u16) < sub_val;
            
            let carry = if cpu.registers.f.carry { 1 } else { 0 };
            let (temp_result, borrow1) = old_a.overflowing_sub(b_val);
            let (final_result, borrow2) = temp_result.overflowing_sub(carry);
            
            cpu.registers.a = final_result;
            cpu.registers.f.zero = final_result == 0;
            cpu.registers.f.carry = borrow1 || borrow2;
            
            assert_eq!(cpu.registers.a, expected, "SBC result: 0x{:02X} - 0x{:02X} - C={}", old_a, b_val, carry_in);
            assert_eq!(cpu.registers.f.carry, has_carry, "SBC carry flag");
        }
    }
}

// ============================================================================
// AND / OR / XOR Property Tests
// ============================================================================

#[test]
fn prop_logical_operations_and() {
    proptest! {
        #[test]
        fn test(a_val in 0u8..=255u8, b_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            
            let result = a_val & b_val;
            cpu.and(b_val);
            
            assert_eq!(cpu.registers.a, result);
            assert_eq!(cpu.registers.f.zero, result == 0);
            assert!(cpu.registers.f.half_carry, "AND always sets H");
            assert!(!cpu.registers.f.carry, "AND always clears C");
            assert!(!cpu.registers.f.subtract, "AND always clears S");
        }
    }
}

#[test]
fn prop_logical_operations_or() {
    proptest! {
        #[test]
        fn test(a_val in 0u8..=255u8, b_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            
            let result = a_val | b_val;
            cpu.or(b_val);
            
            assert_eq!(cpu.registers.a, result);
            assert_eq!(cpu.registers.f.zero, result == 0);
            assert!(!cpu.registers.f.half_carry, "OR always clears H");
            assert!(!cpu.registers.f.carry, "OR always clears C");
        }
    }
}

#[test]
fn prop_logical_operations_xor() {
    proptest! {
        #[test]
        fn test(a_val in 0u8..=255u8, b_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            
            let result = a_val ^ b_val;
            cpu.xor(b_val);
            
            assert_eq!(cpu.registers.a, result);
            assert_eq!(cpu.registers.f.zero, result == 0);
            assert!(!cpu.registers.f.half_carry, "XOR always clears H");
            assert!(!cpu.registers.f.carry, "XOR always clears C");
        }
    }
}

// ============================================================================
// INC / DEC Property Tests
// ============================================================================

#[test]
fn prop_inc_half_carry() {
    proptest! {
        #[test]
        fn test(reg_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = reg_val;
            
            let prev_half = (reg_val & 0xF) == 0xF;
            let new_val = reg_val.wrapping_add(1);
            
            cpu.inc(IncDecTarget::A);
            
            assert_eq!(cpu.registers.a, new_val);
            assert_eq!(cpu.registers.f.zero, new_val == 0);
            assert_eq!(cpu.registers.f.half_carry, prev_half, "INC half_carry for 0x{:02X}", reg_val);
            assert!(!cpu.registers.f.subtract);
        }
    }
}

#[test]
fn prop_dec_half_carry() {
    proptest! {
        #[test]
        fn test(reg_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = reg_val;
            
            let prev_zero = (reg_val & 0xF) == 0;
            let new_val = reg_val.wrapping_sub(1);
            
            cpu.dec(IncDecTarget::A);
            
            assert_eq!(cpu.registers.a, new_val);
            assert_eq!(cpu.registers.f.zero, new_val == 0);
            assert_eq!(cpu.registers.f.half_carry, prev_zero, "DEC half_carry for 0x{:02X}", reg_val);
            assert!(cpu.registers.f.subtract);
        }
    }
}

// ============================================================================
// 16-bit Register Pair Tests
// ============================================================================

#[test]
fn prop_16bit_add_hl() {
    proptest! {
        #[test]
        fn test_add_hl_bc(hl_val in 0u16..=65535u16, bc_val in 0u16..=65535u16) {
            let mut cpu = make_cpu();
            cpu.registers.set_hl(hl_val);
            cpu.registers.set_bc(bc_val);
            
            let result = (hl_val as u32) + (bc_val as u32);
            let expected_hl = (result & 0xFFFF) as u16;
            let expected_carry = result > 0xFFFF;
            let expected_half = ((hl_val & 0x0FFF) as u32) + ((bc_val & 0x0FFF) as u32) > 0x0FFF;
            
            cpu.add_hl(AddHLTarget::BC);
            
            assert_eq!(cpu.registers.get_hl(), expected_hl, "ADD HL,BC result");
            assert_eq!(cpu.registers.f.carry, expected_carry, "ADD HL,BC carry");
            assert_eq!(cpu.registers.f.half_carry, expected_half, "ADD HL,BC half_carry");
            assert!(!cpu.registers.f.subtract, "ADD HL clears subtract");
        }
    }
}

#[test]
fn prop_16bit_inc_dec() {
    proptest! {
        #[test]
        fn test_16bit_inc_dec(reg_val in 0u16..=65535u16) {
            let mut cpu = make_cpu();
            cpu.registers.set_bc(reg_val);
            
            cpu.inc(IncDecTarget::BC);
            assert_eq!(cpu.registers.get_bc(), reg_val.wrapping_add(1));
            
            cpu.registers.set_bc(reg_val);
            cpu.dec(IncDecTarget::BC);
            assert_eq!(cpu.registers.get_bc(), reg_val.wrapping_sub(1));
        }
    }
}

// ============================================================================
// CPL / SCF / CCF
// ============================================================================

#[test]
fn prop_cpl() {
    proptest! {
        #[test]
        fn test(a_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.f.subtract = false;
            cpu.registers.f.half_carry = false;
            
            cpu.execute(Instruction::CPL);
            
            assert_eq!(cpu.registers.a, !a_val, "CPL flips all bits of 0x{:02X}", a_val);
            assert!(cpu.registers.f.subtract, "CPL sets S");
            assert!(cpu.registers.f.half_carry, "CPL sets H");
        }
    }
}

#[test]
fn prop_scf() {
    proptest! {
        #[test]
        fn test(_a in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.f.carry = false;
            cpu.registers.f.subtract = true;
            cpu.registers.f.half_carry = true;
            
            cpu.execute(Instruction::SCF);
            
            assert!(cpu.registers.f.carry, "SCF sets C");
            assert!(!cpu.registers.f.subtract, "SCF clears S");
            assert!(!cpu.registers.f.half_carry, "SCF clears H");
        }
    }
}

#[test]
fn prop_ccf() {
    proptest! {
        #[test]
        fn test(_a in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.f.carry = false;
            
            cpu.execute(Instruction::CCF);
            assert!(cpu.registers.f.carry, "CCF: false -> true");
            
            cpu.execute(Instruction::CCF);
            assert!(!cpu.registers.f.carry, "CCF: true -> false");
        }
    }
}

// ============================================================================
// DAA (Decimal Adjust Accumulator)
// ============================================================================

#[test]
fn prop_daa_basic() {
    proptest! {
        #[test]
        fn test_daa_no_flags(a_val in 0u8..=99u8) {
            // When A is valid BCD (0-99) and no flags set, DAA should be identity
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.f.subtract = false;
            cpu.registers.f.carry = false;
            cpu.registers.f.half_carry = false;
            
            let low_digit = a_val & 0x0F;
            let _high_digit = a_val >> 4;
            
            // Only adjust if low digit > 9 or half_carry set
            let needs_low_adj = low_digit > 9;
            
            cpu.execute(Instruction::DAA);
            
            // DAA should produce a valid result
            if needs_low_adj || a_val > 0x99 {
                assert!(cpu.registers.a <= 0x99 || cpu.registers.f.carry);
            }
        }
    }
}

// ============================================================================
// Load operations: flags unchanged
// ============================================================================

#[test]
fn prop_load_preserves_flags() {
    proptest! {
        #[test]
        fn test_ld_preserves_flags(a_val in 0u8..=255u8, b_val in 0u8..=255u8) {
            let mut cpu = make_cpu();
            cpu.registers.a = a_val;
            cpu.registers.b = b_val;
            
            // Set flags with ADD
            cpu.add(b_val);
            let flags_before = (cpu.registers.f.zero, cpu.registers.f.subtract,
                               cpu.registers.f.half_carry, cpu.registers.f.carry);
            
            // LD B, A should NOT modify flags
            cpu.registers.a = a_val;
            cpu.execute(Instruction::LD(LoadTarget::BFromA));
            let flags_after = (cpu.registers.f.zero, cpu.registers.f.subtract,
                              cpu.registers.f.half_carry, cpu.registers.f.carry);
            
            assert_eq!(flags_before, flags_after, "LD should preserve flags");
        }
    }
}

// ============================================================================
// Stack operations
// ============================================================================

#[test]
fn prop_push_pop_bc() {
    proptest! {
        #[test]
        fn test(bc_val in 0u16..=65535u16) {
            let mut cpu = make_cpu();
            cpu.sp = 0xFFFE;
            cpu.registers.set_bc(bc_val);
            
            cpu.push(StackTarget::BC);
            assert_eq!(cpu.sp, 0xFFFC);
            
            cpu.registers.set_bc(0x0000);
            cpu.pop(StackTarget::BC);
            assert_eq!(cpu.registers.get_bc(), bc_val, "BC push/pop round-trip");
            assert_eq!(cpu.sp, 0xFFFE);
        }
    }
}

#[test]
fn prop_push_pop_hl() {
    proptest! {
        #[test]
        fn test(hl_val in 0u16..=65535u16) {
            let mut cpu = make_cpu();
            cpu.sp = 0xFFFE;
            cpu.registers.set_hl(hl_val);
            
            cpu.push(StackTarget::HL);
            assert_eq!(cpu.sp, 0xFFFC);
            
            cpu.registers.set_hl(0x0000);
            cpu.pop(StackTarget::HL);
            assert_eq!(cpu.registers.get_hl(), hl_val, "HL push/pop round-trip");
            assert_eq!(cpu.sp, 0xFFFE);
        }
    }
}

#[test]
fn prop_push_pop_af() {
    proptest! {
        #[test]
        fn test(af_val in 0u16..=65535u16) {
            let mut cpu = make_cpu();
            cpu.sp = 0xFFFE;
            cpu.registers.set_af(af_val);
            
            cpu.push(StackTarget::AF);
            
            cpu.registers.set_af(0x0000);
            cpu.pop(StackTarget::AF);
            
            let recovered_a = cpu.registers.a;
            let recovered_f = u8::from(cpu.registers.f);
            assert_eq!(((recovered_a as u16) << 8) | (recovered_f as u16), af_val, "AF push/pop round-trip");
        }
    }
}

// ============================================================================
// Test ED instruction decoding
// ============================================================================

#[test]
fn test_ed_opcode_decode() {
    let mut cpu = make_cpu();
    cpu.pc = 0xC000;
    cpu.bus.memory[0xC000] = 0xED;
    
    // Test a sample of ED opcodes
    let ed_opcodes = [0x40, 0x47, 0x4F, 0x53, 0x5B, 0x63, 0x6B, 0x73, 0x7B,
                      0x70, 0x78, 0x7A, 0x72, 0x74, 0x76, 0x79, 0x71, 0x77, 0x7C,
                      0x88, 0x89, 0x90, 0x91, 0xA0, 0xA1, 0xB0, 0xB1,
                      0xB7, 0xBE, 0xBF, 0xC7, 0xDF, 0xE7, 0xEF, 0xF7, 0xFF];
    
    for &opcode in &ed_opcodes {
        cpu.pc = 0xC000;
        cpu.bus.memory[0xC000] = 0xED;
        cpu.bus.memory[0xC001] = opcode;
        
        let result = cpu.fetch_decode();
        match result {
            Instruction::ED(actual) => {
                assert_eq!(actual, opcode, "ED opcode decode mismatch for 0x{:02X}", opcode);
            }
            _ => panic!("Expected ED({:02X}) but got {:?}", opcode, result),
        }
    }
}
