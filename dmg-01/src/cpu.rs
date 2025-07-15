use crate::instructions::*;
use crate::memory::MemoryBus;
use crate::registers::Registers;

pub struct CPU {
    pub registers: Registers,
    pub pc: u16,
    pub sp: u16,
    pub bus: MemoryBus,
    pub halted: bool,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers::new(),
            pc: 0x0100,
            sp: 0xFFFE,
            bus: MemoryBus::new(),
            halted: false,
        }
    }

    pub fn step(&mut self) {
        if self.halted {
            return;
        }

        let instruction = self.fetch_decode();
        self.execute(instruction);
    }

    pub fn fetch_decode(&mut self) -> Instruction {
        let opcode = self.bus.read_byte(self.pc);
        self.pc += 1;

        match opcode {
            0x00 => Instruction::NOP,
            0x01 => Instruction::LD(LoadTarget::BCFromD16),
            0x02 => Instruction::LD(LoadTarget::BCIndirectFromA),
            0x03 => Instruction::INC(IncDecTarget::BC),
            0x04 => Instruction::INC(IncDecTarget::B),
            0x05 => Instruction::DEC(IncDecTarget::B),
            0x06 => Instruction::LD(LoadTarget::BFromD8),
            0x0A => Instruction::LD(LoadTarget::AFromBCIndirect),
            0x0B => Instruction::DEC(IncDecTarget::BC),
            0x0C => Instruction::INC(IncDecTarget::C),
            0x0D => Instruction::DEC(IncDecTarget::C),
            0x0E => Instruction::LD(LoadTarget::CFromD8),
            0x11 => Instruction::LD(LoadTarget::DEFromD16),
            0x12 => Instruction::LD(LoadTarget::DEIndirectFromA),
            0x13 => Instruction::INC(IncDecTarget::DE),
            0x14 => Instruction::INC(IncDecTarget::D),
            0x15 => Instruction::DEC(IncDecTarget::D),
            0x16 => Instruction::LD(LoadTarget::DFromD8),
            0x1A => Instruction::LD(LoadTarget::AFromDEIndirect),
            0x1B => Instruction::DEC(IncDecTarget::DE),
            0x1C => Instruction::INC(IncDecTarget::E),
            0x1D => Instruction::DEC(IncDecTarget::E),
            0x1E => Instruction::LD(LoadTarget::EFromD8),
            0x20 => Instruction::JR(JumpCondition::NotZero),
            0x21 => Instruction::LD(LoadTarget::HLFromD16),
            0x22 => Instruction::LD(LoadTarget::HLIndirectFromA),
            0x23 => Instruction::INC(IncDecTarget::HL),
            0x24 => Instruction::INC(IncDecTarget::H),
            0x25 => Instruction::DEC(IncDecTarget::H),
            0x26 => Instruction::LD(LoadTarget::HFromD8),
            0x28 => Instruction::JR(JumpCondition::Zero),
            0x2A => Instruction::LD(LoadTarget::AFromHLIndirect),
            0x2B => Instruction::DEC(IncDecTarget::HL),
            0x2C => Instruction::INC(IncDecTarget::L),
            0x2D => Instruction::DEC(IncDecTarget::L),
            0x2E => Instruction::LD(LoadTarget::LFromD8),
            0x30 => Instruction::JR(JumpCondition::NotCarry),
            0x31 => Instruction::LD(LoadTarget::SPFromD16),
            0x32 => Instruction::LD(LoadTarget::HLIndirectFromA),
            0x33 => Instruction::INC(IncDecTarget::SP),
            0x34 => Instruction::INC(IncDecTarget::HLIndirect),
            0x35 => Instruction::DEC(IncDecTarget::HLIndirect),
            0x36 => Instruction::LD(LoadTarget::HLIndirectFromD8),
            0x38 => Instruction::JR(JumpCondition::Carry),
            0x3A => Instruction::LD(LoadTarget::AFromHLIndirect),
            0x3B => Instruction::DEC(IncDecTarget::SP),
            0x3C => Instruction::INC(IncDecTarget::A),
            0x3D => Instruction::DEC(IncDecTarget::A),
            0x3E => Instruction::LD(LoadTarget::AFromD8),
            0x76 => Instruction::HALT,
            0x80 => Instruction::ADD(ArithmeticTarget::B),
            0x81 => Instruction::ADD(ArithmeticTarget::C),
            0x82 => Instruction::ADD(ArithmeticTarget::D),
            0x83 => Instruction::ADD(ArithmeticTarget::E),
            0x84 => Instruction::ADD(ArithmeticTarget::H),
            0x85 => Instruction::ADD(ArithmeticTarget::L),
            0x86 => Instruction::ADD(ArithmeticTarget::HLIndirect),
            0x87 => Instruction::ADD(ArithmeticTarget::A),
            0x90 => Instruction::SUB(ArithmeticTarget::B),
            0x91 => Instruction::SUB(ArithmeticTarget::C),
            0x92 => Instruction::SUB(ArithmeticTarget::D),
            0x93 => Instruction::SUB(ArithmeticTarget::E),
            0x94 => Instruction::SUB(ArithmeticTarget::H),
            0x95 => Instruction::SUB(ArithmeticTarget::L),
            0x96 => Instruction::SUB(ArithmeticTarget::HLIndirect),
            0x97 => Instruction::SUB(ArithmeticTarget::A),
            0xA0 => Instruction::AND(ArithmeticTarget::B),
            0xA1 => Instruction::AND(ArithmeticTarget::C),
            0xA2 => Instruction::AND(ArithmeticTarget::D),
            0xA3 => Instruction::AND(ArithmeticTarget::E),
            0xA4 => Instruction::AND(ArithmeticTarget::H),
            0xA5 => Instruction::AND(ArithmeticTarget::L),
            0xA6 => Instruction::AND(ArithmeticTarget::HLIndirect),
            0xA7 => Instruction::AND(ArithmeticTarget::A),
            0xB0 => Instruction::OR(ArithmeticTarget::B),
            0xB1 => Instruction::OR(ArithmeticTarget::C),
            0xB2 => Instruction::OR(ArithmeticTarget::D),
            0xB3 => Instruction::OR(ArithmeticTarget::E),
            0xB4 => Instruction::OR(ArithmeticTarget::H),
            0xB5 => Instruction::OR(ArithmeticTarget::L),
            0xB6 => Instruction::OR(ArithmeticTarget::HLIndirect),
            0xB7 => Instruction::OR(ArithmeticTarget::A),
            0xA8 => Instruction::XOR(ArithmeticTarget::B),
            0xA9 => Instruction::XOR(ArithmeticTarget::C),
            0xAA => Instruction::XOR(ArithmeticTarget::D),
            0xAB => Instruction::XOR(ArithmeticTarget::E),
            0xAC => Instruction::XOR(ArithmeticTarget::H),
            0xAD => Instruction::XOR(ArithmeticTarget::L),
            0xAE => Instruction::XOR(ArithmeticTarget::HLIndirect),
            0xAF => Instruction::XOR(ArithmeticTarget::A),
            0xB8 => Instruction::CP(ArithmeticTarget::B),
            0xB9 => Instruction::CP(ArithmeticTarget::C),
            0xBA => Instruction::CP(ArithmeticTarget::D),
            0xBB => Instruction::CP(ArithmeticTarget::E),
            0xBC => Instruction::CP(ArithmeticTarget::H),
            0xBD => Instruction::CP(ArithmeticTarget::L),
            0xBE => Instruction::CP(ArithmeticTarget::HLIndirect),
            0xBF => Instruction::CP(ArithmeticTarget::A),
            0xC0 => Instruction::RET(JumpCondition::NotZero),
            0xC1 => Instruction::POP(StackTarget::BC),
            0xC2 => Instruction::JP(JumpCondition::NotZero),
            0xC3 => Instruction::JP(JumpCondition::Always),
            0xC4 => Instruction::CALL(JumpCondition::NotZero),
            0xC5 => Instruction::PUSH(StackTarget::BC),
            0xC6 => Instruction::ADD(ArithmeticTarget::D8),
            0xC8 => Instruction::RET(JumpCondition::Zero),
            0xC9 => Instruction::RET(JumpCondition::Always),
            0xCA => Instruction::JP(JumpCondition::Zero),
            0xCC => Instruction::CALL(JumpCondition::Zero),
            0xCD => Instruction::CALL(JumpCondition::Always),
            0xD0 => Instruction::RET(JumpCondition::NotCarry),
            0xD1 => Instruction::POP(StackTarget::DE),
            0xD2 => Instruction::JP(JumpCondition::NotCarry),
            0xD4 => Instruction::CALL(JumpCondition::NotCarry),
            0xD5 => Instruction::PUSH(StackTarget::DE),
            0xD6 => Instruction::SUB(ArithmeticTarget::D8),
            0xD8 => Instruction::RET(JumpCondition::Carry),
            0xDA => Instruction::JP(JumpCondition::Carry),
            0xDC => Instruction::CALL(JumpCondition::Carry),
            0xE0 => Instruction::RET(JumpCondition::NotCarry),
            0xE1 => Instruction::POP(StackTarget::HL),
            0xE2 => Instruction::JP(JumpCondition::NotCarry),
            0xE5 => Instruction::PUSH(StackTarget::HL),
            0xE6 => Instruction::AND(ArithmeticTarget::D8),
            0xEE => Instruction::XOR(ArithmeticTarget::D8),
            0xF0 => Instruction::RET(JumpCondition::NotCarry),
            0xF1 => Instruction::POP(StackTarget::AF),
            0xF3 => Instruction::JP(JumpCondition::NotCarry),
            0xF5 => Instruction::PUSH(StackTarget::AF),
            0xF6 => Instruction::OR(ArithmeticTarget::D8),
            0xFE => Instruction::CP(ArithmeticTarget::D8),
            _ => Instruction::NOP,
        }
    }

    pub fn execute(&mut self, instruction: Instruction) {
        match instruction {
            Instruction::NOP => {}
            Instruction::HALT => self.halted = true,
            Instruction::ADD(target) => {
                let value = self.get_arithmetic_target_value(target);
                self.add(value);
            }
            Instruction::SUB(target) => {
                let value = self.get_arithmetic_target_value(target);
                self.sub(value);
            }
            Instruction::AND(target) => {
                let value = self.get_arithmetic_target_value(target);
                self.and(value);
            }
            Instruction::OR(target) => {
                let value = self.get_arithmetic_target_value(target);
                self.or(value);
            }
            Instruction::XOR(target) => {
                let value = self.get_arithmetic_target_value(target);
                self.xor(value);
            }
            Instruction::CP(target) => {
                let value = self.get_arithmetic_target_value(target);
                self.cp(value);
            }
            Instruction::INC(target) => {
                self.inc(target);
            }
            Instruction::DEC(target) => {
                self.dec(target);
            }
            Instruction::LD(target) => {
                self.load(target);
            }
            Instruction::PUSH(target) => {
                self.push(target);
            }
            Instruction::POP(target) => {
                self.pop(target);
            }
            Instruction::JP(condition) => {
                self.jump(condition);
            }
            Instruction::JR(condition) => {
                self.jump_relative(condition);
            }
            Instruction::CALL(condition) => {
                self.call(condition);
            }
            Instruction::RET(condition) => {
                self.ret(condition);
            }
        }
    }

    fn get_arithmetic_target_value(&mut self, target: ArithmeticTarget) -> u8 {
        match target {
            ArithmeticTarget::A => self.registers.a,
            ArithmeticTarget::B => self.registers.b,
            ArithmeticTarget::C => self.registers.c,
            ArithmeticTarget::D => self.registers.d,
            ArithmeticTarget::E => self.registers.e,
            ArithmeticTarget::H => self.registers.h,
            ArithmeticTarget::L => self.registers.l,
            ArithmeticTarget::HLIndirect => {
                let address = self.registers.get_hl();
                self.bus.read_byte(address)
            }
            ArithmeticTarget::D8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                value
            }
        }
    }

    pub fn add(&mut self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_add(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) + (value & 0xF) > 0xF;
        self.registers.a = new_value;
    }

    pub fn sub(&mut self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) < (value & 0xF);
        self.registers.a = new_value;
    }

    pub fn and(&mut self, value: u8) {
        let new_value = self.registers.a & value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = false;
        self.registers.f.half_carry = true;
        self.registers.a = new_value;
    }

    pub fn or(&mut self, value: u8) {
        let new_value = self.registers.a | value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = false;
        self.registers.f.half_carry = false;
        self.registers.a = new_value;
    }

    pub fn xor(&mut self, value: u8) {
        let new_value = self.registers.a ^ value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = false;
        self.registers.f.half_carry = false;
        self.registers.a = new_value;
    }

    fn cp(&mut self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) < (value & 0xF);
    }

    pub fn inc(&mut self, target: IncDecTarget) {
        match target {
            IncDecTarget::A => {
                let (new_value, _) = self.registers.a.overflowing_add(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.registers.a & 0xF) == 0xF;
                self.registers.a = new_value;
            }
            IncDecTarget::B => {
                let (new_value, _) = self.registers.b.overflowing_add(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.registers.b & 0xF) == 0xF;
                self.registers.b = new_value;
            }
            IncDecTarget::C => {
                let (new_value, _) = self.registers.c.overflowing_add(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.registers.c & 0xF) == 0xF;
                self.registers.c = new_value;
            }
            IncDecTarget::D => {
                let (new_value, _) = self.registers.d.overflowing_add(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.registers.d & 0xF) == 0xF;
                self.registers.d = new_value;
            }
            IncDecTarget::E => {
                let (new_value, _) = self.registers.e.overflowing_add(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.registers.e & 0xF) == 0xF;
                self.registers.e = new_value;
            }
            IncDecTarget::H => {
                let (new_value, _) = self.registers.h.overflowing_add(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.registers.h & 0xF) == 0xF;
                self.registers.h = new_value;
            }
            IncDecTarget::L => {
                let (new_value, _) = self.registers.l.overflowing_add(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.registers.l & 0xF) == 0xF;
                self.registers.l = new_value;
            }
            IncDecTarget::HLIndirect => {
                let address = self.registers.get_hl();
                let value = self.bus.read_byte(address);
                let (new_value, _) = value.overflowing_add(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (value & 0xF) == 0xF;
                self.bus.write_byte(address, new_value);
            }
            IncDecTarget::BC => {
                let value = self.registers.get_bc().wrapping_add(1);
                self.registers.set_bc(value);
            }
            IncDecTarget::DE => {
                let value = self.registers.get_de().wrapping_add(1);
                self.registers.set_de(value);
            }
            IncDecTarget::HL => {
                let value = self.registers.get_hl().wrapping_add(1);
                self.registers.set_hl(value);
            }
            IncDecTarget::SP => {
                self.sp = self.sp.wrapping_add(1);
            }
        }
    }

    pub fn dec(&mut self, target: IncDecTarget) {
        match target {
            IncDecTarget::A => {
                let (new_value, _) = self.registers.a.overflowing_sub(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (self.registers.a & 0xF) == 0;
                self.registers.a = new_value;
            }
            IncDecTarget::B => {
                let (new_value, _) = self.registers.b.overflowing_sub(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (self.registers.b & 0xF) == 0;
                self.registers.b = new_value;
            }
            IncDecTarget::C => {
                let (new_value, _) = self.registers.c.overflowing_sub(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (self.registers.c & 0xF) == 0;
                self.registers.c = new_value;
            }
            IncDecTarget::D => {
                let (new_value, _) = self.registers.d.overflowing_sub(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (self.registers.d & 0xF) == 0;
                self.registers.d = new_value;
            }
            IncDecTarget::E => {
                let (new_value, _) = self.registers.e.overflowing_sub(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (self.registers.e & 0xF) == 0;
                self.registers.e = new_value;
            }
            IncDecTarget::H => {
                let (new_value, _) = self.registers.h.overflowing_sub(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (self.registers.h & 0xF) == 0;
                self.registers.h = new_value;
            }
            IncDecTarget::L => {
                let (new_value, _) = self.registers.l.overflowing_sub(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (self.registers.l & 0xF) == 0;
                self.registers.l = new_value;
            }
            IncDecTarget::HLIndirect => {
                let address = self.registers.get_hl();
                let value = self.bus.read_byte(address);
                let (new_value, _) = value.overflowing_sub(1);
                self.registers.f.zero = new_value == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (value & 0xF) == 0;
                self.bus.write_byte(address, new_value);
            }
            IncDecTarget::BC => {
                let value = self.registers.get_bc().wrapping_sub(1);
                self.registers.set_bc(value);
            }
            IncDecTarget::DE => {
                let value = self.registers.get_de().wrapping_sub(1);
                self.registers.set_de(value);
            }
            IncDecTarget::HL => {
                let value = self.registers.get_hl().wrapping_sub(1);
                self.registers.set_hl(value);
            }
            IncDecTarget::SP => {
                self.sp = self.sp.wrapping_sub(1);
            }
        }
    }

    fn load(&mut self, target: LoadTarget) {
        match target {
            LoadTarget::AFromHLIndirect => {
                let address = self.registers.get_hl();
                self.registers.a = self.bus.read_byte(address);
            }
            LoadTarget::AFromBCIndirect => {
                let address = self.registers.get_bc();
                self.registers.a = self.bus.read_byte(address);
            }
            LoadTarget::AFromDEIndirect => {
                let address = self.registers.get_de();
                self.registers.a = self.bus.read_byte(address);
            }
            LoadTarget::HLIndirectFromA => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.a);
            }
            LoadTarget::BCIndirectFromA => {
                let address = self.registers.get_bc();
                self.bus.write_byte(address, self.registers.a);
            }
            LoadTarget::DEIndirectFromA => {
                let address = self.registers.get_de();
                self.bus.write_byte(address, self.registers.a);
            }
            LoadTarget::AFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                self.registers.a = value;
            }
            LoadTarget::BFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                self.registers.b = value;
            }
            LoadTarget::CFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                self.registers.c = value;
            }
            LoadTarget::DFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                self.registers.d = value;
            }
            LoadTarget::EFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                self.registers.e = value;
            }
            LoadTarget::HFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                self.registers.h = value;
            }
            LoadTarget::LFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                self.registers.l = value;
            }
            LoadTarget::HLIndirectFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc += 1;
                let address = self.registers.get_hl();
                self.bus.write_byte(address, value);
            }
            LoadTarget::BCFromD16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc += 1;
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc += 1;
                self.registers.set_bc((high << 8) | low);
            }
            LoadTarget::DEFromD16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc += 1;
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc += 1;
                self.registers.set_de((high << 8) | low);
            }
            LoadTarget::HLFromD16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc += 1;
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc += 1;
                self.registers.set_hl((high << 8) | low);
            }
            LoadTarget::SPFromD16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc += 1;
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc += 1;
                self.sp = (high << 8) | low;
            }
        }
    }

    pub fn push(&mut self, target: StackTarget) {
        let value = match target {
            StackTarget::AF => self.registers.get_af(),
            StackTarget::BC => self.registers.get_bc(),
            StackTarget::DE => self.registers.get_de(),
            StackTarget::HL => self.registers.get_hl(),
        };

        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, ((value & 0xFF00) >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, (value & 0xFF) as u8);
    }

    pub fn pop(&mut self, target: StackTarget) {
        let low = self.bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let high = self.bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let value = (high << 8) | low;

        match target {
            StackTarget::AF => self.registers.set_af(value),
            StackTarget::BC => self.registers.set_bc(value),
            StackTarget::DE => self.registers.set_de(value),
            StackTarget::HL => self.registers.set_hl(value),
        }
    }

    fn jump(&mut self, condition: JumpCondition) {
        if self.check_condition(condition) {
            let low = self.bus.read_byte(self.pc) as u16;
            self.pc += 1;
            let high = self.bus.read_byte(self.pc) as u16;
            self.pc += 1;
            self.pc = (high << 8) | low;
        } else {
            self.pc += 2;
        }
    }

    fn jump_relative(&mut self, condition: JumpCondition) {
        if self.check_condition(condition) {
            let offset = self.bus.read_byte(self.pc) as i8;
            self.pc += 1;
            self.pc = ((self.pc as i32) + (offset as i32)) as u16;
        } else {
            self.pc += 1;
        }
    }

    fn call(&mut self, condition: JumpCondition) {
        if self.check_condition(condition) {
            let low = self.bus.read_byte(self.pc) as u16;
            self.pc += 1;
            let high = self.bus.read_byte(self.pc) as u16;
            self.pc += 1;
            let address = (high << 8) | low;

            self.sp = self.sp.wrapping_sub(1);
            self.bus
                .write_byte(self.sp, ((self.pc & 0xFF00) >> 8) as u8);
            self.sp = self.sp.wrapping_sub(1);
            self.bus.write_byte(self.sp, (self.pc & 0xFF) as u8);

            self.pc = address;
        } else {
            self.pc += 2;
        }
    }

    fn ret(&mut self, condition: JumpCondition) {
        if self.check_condition(condition) {
            let low = self.bus.read_byte(self.sp) as u16;
            self.sp = self.sp.wrapping_add(1);
            let high = self.bus.read_byte(self.sp) as u16;
            self.sp = self.sp.wrapping_add(1);
            self.pc = (high << 8) | low;
        }
    }

    pub fn check_condition(&self, condition: JumpCondition) -> bool {
        match condition {
            JumpCondition::Always => true,
            JumpCondition::Zero => self.registers.f.zero,
            JumpCondition::NotZero => !self.registers.f.zero,
            JumpCondition::Carry => self.registers.f.carry,
            JumpCondition::NotCarry => !self.registers.f.carry,
        }
    }
}