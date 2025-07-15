use dmg_01::cpu::CPU;
use dmg_01::instructions::*;

#[test]
fn test_cpu_arithmetic_operations() {
    let mut cpu = CPU::new();
    cpu.registers.a = 0x10;

    cpu.add(0x05);
    assert_eq!(cpu.registers.a, 0x15);
    assert!(!cpu.registers.f.zero);
    assert!(!cpu.registers.f.subtract);
    assert!(!cpu.registers.f.carry);
    assert!(!cpu.registers.f.half_carry);

    cpu.registers.a = 0x0F;
    cpu.add(0x01);
    assert_eq!(cpu.registers.a, 0x10);
    assert!(cpu.registers.f.half_carry);

    cpu.registers.a = 0xFF;
    cpu.add(0x01);
    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.f.zero);
    assert!(cpu.registers.f.carry);
}

#[test]
fn test_cpu_sub_operation() {
    let mut cpu = CPU::new();
    cpu.registers.a = 0x10;

    cpu.sub(0x05);
    assert_eq!(cpu.registers.a, 0x0B);
    assert!(!cpu.registers.f.zero);
    assert!(cpu.registers.f.subtract);
    assert!(!cpu.registers.f.carry);

    cpu.registers.a = 0x00;
    cpu.sub(0x01);
    assert_eq!(cpu.registers.a, 0xFF);
    assert!(cpu.registers.f.carry);
}

#[test]
fn test_cpu_logical_operations() {
    let mut cpu = CPU::new();
    cpu.registers.a = 0xF0;

    cpu.and(0x0F);
    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.f.zero);
    assert!(cpu.registers.f.half_carry);

    cpu.registers.a = 0xF0;
    cpu.or(0x0F);
    assert_eq!(cpu.registers.a, 0xFF);
    assert!(!cpu.registers.f.zero);

    cpu.registers.a = 0xFF;
    cpu.xor(0xFF);
    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.f.zero);
}

#[test]
fn test_cpu_inc_dec_operations() {
    let mut cpu = CPU::new();
    cpu.registers.a = 0x0F;

    cpu.inc(IncDecTarget::A);
    assert_eq!(cpu.registers.a, 0x10);
    assert!(cpu.registers.f.half_carry);

    cpu.registers.a = 0xFF;
    cpu.inc(IncDecTarget::A);
    assert_eq!(cpu.registers.a, 0x00);
    assert!(cpu.registers.f.zero);

    cpu.registers.a = 0x10;
    cpu.dec(IncDecTarget::A);
    assert_eq!(cpu.registers.a, 0x0F);
    assert!(cpu.registers.f.half_carry);
}

#[test]
fn test_cpu_16bit_inc_dec() {
    let mut cpu = CPU::new();
    cpu.registers.set_bc(0x1234);

    cpu.inc(IncDecTarget::BC);
    assert_eq!(cpu.registers.get_bc(), 0x1235);

    cpu.dec(IncDecTarget::BC);
    assert_eq!(cpu.registers.get_bc(), 0x1234);

    cpu.registers.set_bc(0xFFFF);
    cpu.inc(IncDecTarget::BC);
    assert_eq!(cpu.registers.get_bc(), 0x0000);
}

#[test]
fn test_jump_conditions() {
    let mut cpu = CPU::new();
    cpu.registers.f.zero = true;
    cpu.registers.f.carry = false;

    assert!(cpu.check_condition(JumpCondition::Always));
    assert!(cpu.check_condition(JumpCondition::Zero));
    assert!(!cpu.check_condition(JumpCondition::NotZero));
    assert!(!cpu.check_condition(JumpCondition::Carry));
    assert!(cpu.check_condition(JumpCondition::NotCarry));
}

#[test]
fn test_instruction_execution() {
    let mut cpu = CPU::new();
    cpu.registers.a = 0x10;
    cpu.registers.b = 0x05;

    cpu.execute(Instruction::ADD(ArithmeticTarget::B));
    assert_eq!(cpu.registers.a, 0x15);

    cpu.execute(Instruction::SUB(ArithmeticTarget::B));
    assert_eq!(cpu.registers.a, 0x10);

    cpu.execute(Instruction::INC(IncDecTarget::A));
    assert_eq!(cpu.registers.a, 0x11);

    cpu.execute(Instruction::DEC(IncDecTarget::A));
    assert_eq!(cpu.registers.a, 0x10);
}

#[test]
fn test_load_operations() {
    let mut cpu = CPU::new();
    cpu.bus.write_byte(0xC000, 0x42);
    cpu.registers.set_hl(0xC000);

    cpu.execute(Instruction::LD(LoadTarget::AFromHLIndirect));
    assert_eq!(cpu.registers.a, 0x42);

    cpu.registers.a = 0x55;
    cpu.execute(Instruction::LD(LoadTarget::HLIndirectFromA));
    assert_eq!(cpu.bus.read_byte(0xC000), 0x55);
}

#[test]
fn test_stack_operations() {
    let mut cpu = CPU::new();
    cpu.sp = 0xFFFE;
    cpu.registers.set_bc(0x1234);

    cpu.push(StackTarget::BC);
    assert_eq!(cpu.sp, 0xFFFC);
    assert_eq!(cpu.bus.read_byte(0xFFFC), 0x34);
    assert_eq!(cpu.bus.read_byte(0xFFFD), 0x12);

    cpu.registers.set_bc(0x0000);
    cpu.pop(StackTarget::BC);
    assert_eq!(cpu.sp, 0xFFFE);
    assert_eq!(cpu.registers.get_bc(), 0x1234);
}

#[test]
fn test_fetch_decode_cycle() {
    let mut cpu = CPU::new();
    cpu.bus.memory[0x0100] = 0x00;
    cpu.bus.memory[0x0101] = 0x76;
    cpu.bus.memory[0x0102] = 0x3E;
    cpu.bus.memory[0x0103] = 0x42;

    let instruction = cpu.fetch_decode();
    assert_eq!(cpu.pc, 0x0101);
    match instruction {
        Instruction::NOP => {}
        _ => panic!("Expected NOP instruction"),
    }

    let instruction = cpu.fetch_decode();
    assert_eq!(cpu.pc, 0x0102);
    match instruction {
        Instruction::HALT => {}
        _ => panic!("Expected HALT instruction"),
    }

    let instruction = cpu.fetch_decode();
    assert_eq!(cpu.pc, 0x0103);
    match instruction {
        Instruction::LD(LoadTarget::AFromD8) => {}
        _ => panic!("Expected LD A, d8 instruction"),
    }
}