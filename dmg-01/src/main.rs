fn main() {
    let mut gameboy = GameBoy::new();

    let test_rom = vec![
        0x3E, 0x42, // LD A, 0x42
        0x06, 0x10, // LD B, 0x10
        0x80, // ADD A, B
        0x3C, // INC A
        0x76, // HALT
    ];

    gameboy.load_cartridge(test_rom);

    println!("Game Boy Emulator - DMG-01");
    println!("Running test ROM...");

    for i in 0..10 {
        println!(
            "Step {}: PC=0x{:04X}, A=0x{:02X}, B=0x{:02X}",
            i, gameboy.cpu.pc, gameboy.cpu.registers.a, gameboy.cpu.registers.b
        );

        if gameboy.cpu.halted {
            break;
        }

        gameboy.step();
    }

    println!("Emulation complete!");
    println!(
        "Final state: A=0x{:02X}, B=0x{:02X}",
        gameboy.cpu.registers.a, gameboy.cpu.registers.b
    );
}

const ZERO_FLAG_BYTE_POSITION: u8 = 7;
const SUBTRACT_FLAG_BYTE_POSITION: u8 = 6;
const HALF_CARRY_FLAG_BYTE_POSITION: u8 = 5;
const CARRY_FLAG_BYTE_POSITION: u8 = 4;

struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: FlagsRegister,
    h: u8,
    l: u8,
}

impl Registers {
    fn get_bc(&self) -> u16 {
        (self.b as u16) << 8 | (self.c as u16)
    }

    fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    fn get_de(&self) -> u16 {
        (self.d as u16) << 8 | (self.e as u16)
    }

    fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    fn get_hl(&self) -> u16 {
        (self.h as u16) << 8 | (self.l as u16)
    }

    fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }

    fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | (u8::from(self.f) as u16)
    }

    fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;
        self.f = FlagsRegister::from((value & 0xFF) as u8);
    }
}

#[derive(Copy, Clone)]
struct FlagsRegister {
    zero: bool,
    subtract: bool,
    half_carry: bool,
    carry: bool,
}

impl std::convert::From<FlagsRegister> for u8 {
    fn from(flag: FlagsRegister) -> u8 {
        (if flag.zero { 1 } else { 0 }) << ZERO_FLAG_BYTE_POSITION
            | (if flag.subtract { 1 } else { 0 }) << SUBTRACT_FLAG_BYTE_POSITION
            | (if flag.half_carry { 1 } else { 0 }) << HALF_CARRY_FLAG_BYTE_POSITION
            | (if flag.carry { 1 } else { 0 }) << CARRY_FLAG_BYTE_POSITION
    }
}

impl std::convert::From<u8> for FlagsRegister {
    fn from(byte: u8) -> Self {
        let zero = ((byte >> ZERO_FLAG_BYTE_POSITION) & 0b1) != 0;
        let subtract = ((byte >> SUBTRACT_FLAG_BYTE_POSITION) & 0b1) != 0;
        let half_carry = ((byte >> HALF_CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;
        let carry = ((byte >> CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;

        FlagsRegister {
            zero,
            subtract,
            half_carry,
            carry,
        }
    }
}

enum Instruction {
    ADD(ArithmeticTarget),
    SUB(ArithmeticTarget),
    AND(ArithmeticTarget),
    OR(ArithmeticTarget),
    XOR(ArithmeticTarget),
    CP(ArithmeticTarget),
    INC(IncDecTarget),
    DEC(IncDecTarget),
    LD(LoadTarget),
    PUSH(StackTarget),
    POP(StackTarget),
    JP(JumpCondition),
    JR(JumpCondition),
    CALL(JumpCondition),
    RET(JumpCondition),
    NOP,
    HALT,
}

enum ArithmeticTarget {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    HLIndirect,
    D8,
}

enum IncDecTarget {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    HLIndirect,
    BC,
    DE,
    HL,
    SP,
}

enum LoadTarget {
    AFromHLIndirect,
    AFromBCIndirect,
    AFromDEIndirect,
    HLIndirectFromA,
    BCIndirectFromA,
    DEIndirectFromA,
    AFromD8,
    BFromD8,
    CFromD8,
    DFromD8,
    EFromD8,
    HFromD8,
    LFromD8,
    HLIndirectFromD8,
    BCFromD16,
    DEFromD16,
    HLFromD16,
    SPFromD16,
}

enum StackTarget {
    AF,
    BC,
    DE,
    HL,
}

enum JumpCondition {
    Always,
    Zero,
    NotZero,
    Carry,
    NotCarry,
}

struct CPU {
    registers: Registers,
    pc: u16,
    sp: u16,
    bus: MemoryBus,
    halted: bool,
}

struct MemoryBus {
    memory: [u8; 0x10000],
    cartridge: Option<Cartridge>,
}

struct Cartridge {
    rom: Vec<u8>,
    rom_size: usize,
    ram: Vec<u8>,
    ram_size: usize,
    mbc_type: u8,
    current_rom_bank: usize,
    current_ram_bank: usize,
}

impl CPU {
    fn new() -> Self {
        CPU {
            registers: Registers {
                a: 0x01,
                b: 0x00,
                c: 0x13,
                d: 0x00,
                e: 0xD8,
                f: FlagsRegister {
                    zero: true,
                    subtract: false,
                    half_carry: true,
                    carry: true,
                },
                h: 0x01,
                l: 0x4D,
            },
            pc: 0x0100,
            sp: 0xFFFE,
            bus: MemoryBus::new(),
            halted: false,
        }
    }

    fn step(&mut self) {
        if self.halted {
            return;
        }

        let instruction = self.fetch_decode();
        self.execute(instruction);
    }

    fn fetch_decode(&mut self) -> Instruction {
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

    fn execute(&mut self, instruction: Instruction) {
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

    fn add(&mut self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_add(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) + (value & 0xF) > 0xF;
        self.registers.a = new_value;
    }

    fn sub(&mut self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) < (value & 0xF);
        self.registers.a = new_value;
    }

    fn and(&mut self, value: u8) {
        let new_value = self.registers.a & value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = false;
        self.registers.f.half_carry = true;
        self.registers.a = new_value;
    }

    fn or(&mut self, value: u8) {
        let new_value = self.registers.a | value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = false;
        self.registers.f.half_carry = false;
        self.registers.a = new_value;
    }

    fn xor(&mut self, value: u8) {
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

    fn inc(&mut self, target: IncDecTarget) {
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

    fn dec(&mut self, target: IncDecTarget) {
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

    fn push(&mut self, target: StackTarget) {
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

    fn pop(&mut self, target: StackTarget) {
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

    fn check_condition(&self, condition: JumpCondition) -> bool {
        match condition {
            JumpCondition::Always => true,
            JumpCondition::Zero => self.registers.f.zero,
            JumpCondition::NotZero => !self.registers.f.zero,
            JumpCondition::Carry => self.registers.f.carry,
            JumpCondition::NotCarry => !self.registers.f.carry,
        }
    }
}

impl MemoryBus {
    fn new() -> Self {
        MemoryBus {
            memory: [0; 0x10000],
            cartridge: None,
        }
    }

    fn load_cartridge(&mut self, rom_data: Vec<u8>) {
        let cartridge = Cartridge::new(rom_data);
        self.cartridge = Some(cartridge);

        // Copy ROM data to memory starting at 0x0100 (Game Boy boot address)
        if let Some(cartridge) = &self.cartridge {
            for (i, &byte) in cartridge.rom.iter().enumerate() {
                if i + 0x0100 < 0x8000 {
                    self.memory[i + 0x0100] = byte;
                }
            }
        }
    }

    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => {
                // For addresses 0x0100-0x7FFF, read from memory (loaded ROM)
                // For addresses 0x0000-0x00FF, read from cartridge if available
                if address >= 0x0100 {
                    self.memory[address as usize]
                } else if let Some(cartridge) = &self.cartridge {
                    cartridge.read_rom(address)
                } else {
                    0xFF
                }
            }
            0x8000..=0x9FFF => self.memory[address as usize],
            0xA000..=0xBFFF => {
                if let Some(cartridge) = &self.cartridge {
                    cartridge.read_ram(address - 0xA000)
                } else {
                    0xFF
                }
            }
            0xC000..=0xFDFF => self.memory[address as usize],
            0xFE00..=0xFE9F => self.memory[address as usize],
            0xFEA0..=0xFEFF => 0x00,
            0xFF00..=0xFF7F => self.memory[address as usize],
            0xFF80..=0xFFFE => self.memory[address as usize],
            0xFFFF => self.memory[address as usize],
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x7FFF => {
                if let Some(cartridge) = &mut self.cartridge {
                    cartridge.write_rom(address, value);
                }
            }
            0x8000..=0x9FFF => self.memory[address as usize] = value,
            0xA000..=0xBFFF => {
                if let Some(cartridge) = &mut self.cartridge {
                    cartridge.write_ram(address - 0xA000, value);
                }
            }
            0xC000..=0xFDFF => self.memory[address as usize] = value,
            0xFE00..=0xFE9F => self.memory[address as usize] = value,
            0xFEA0..=0xFEFF => {}
            0xFF00..=0xFF7F => self.memory[address as usize] = value,
            0xFF80..=0xFFFE => self.memory[address as usize] = value,
            0xFFFF => self.memory[address as usize] = value,
        }
    }
}

impl Cartridge {
    fn new(rom_data: Vec<u8>) -> Self {
        let rom_size = rom_data.len();
        let mbc_type = if rom_size > 0x8000 {
            rom_data[0x0147]
        } else {
            0
        };
        let ram_size = match mbc_type {
            0x00 => 0,
            _ => 0x2000,
        };

        Cartridge {
            rom: rom_data,
            rom_size,
            ram: vec![0; ram_size],
            ram_size,
            mbc_type,
            current_rom_bank: 1,
            current_ram_bank: 0,
        }
    }

    fn read_rom(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                if (address as usize) < self.rom.len() {
                    self.rom[address as usize]
                } else {
                    0xFF
                }
            }
            0x4000..=0x7FFF => {
                let bank_offset = self.current_rom_bank * 0x4000;
                let rom_address = bank_offset + ((address - 0x4000) as usize);
                if rom_address < self.rom.len() {
                    self.rom[rom_address]
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    fn write_rom(&mut self, address: u16, value: u8) {
        match self.mbc_type {
            0x00 => {}
            _ => match address {
                0x2000..=0x3FFF => {
                    let bank = (value & 0x1F) as usize;
                    self.current_rom_bank = if bank == 0 { 1 } else { bank };
                }
                0x4000..=0x5FFF => {
                    self.current_ram_bank = (value & 0x03) as usize;
                }
                _ => {}
            },
        }
    }

    fn read_ram(&self, address: u16) -> u8 {
        let ram_address = (self.current_ram_bank * 0x2000) + (address as usize);
        if ram_address < self.ram.len() {
            self.ram[ram_address]
        } else {
            0xFF
        }
    }

    fn write_ram(&mut self, address: u16, value: u8) {
        let ram_address = (self.current_ram_bank * 0x2000) + (address as usize);
        if ram_address < self.ram.len() {
            self.ram[ram_address] = value;
        }
    }
}

struct GameBoy {
    cpu: CPU,
}

impl GameBoy {
    fn new() -> Self {
        GameBoy { cpu: CPU::new() }
    }

    fn load_cartridge(&mut self, rom_data: Vec<u8>) {
        self.cpu.bus.load_cartridge(rom_data);
    }

    fn run(&mut self) {
        loop {
            self.cpu.step();
            if self.cpu.halted {
                break;
            }
        }
    }

    fn step(&mut self) {
        self.cpu.step();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registers_16bit_operations() {
        let mut registers = Registers {
            a: 0x12,
            b: 0x34,
            c: 0x56,
            d: 0x78,
            e: 0x9A,
            f: FlagsRegister::from(0xB0),
            h: 0xBC,
            l: 0xDE,
        };

        assert_eq!(registers.get_bc(), 0x3456);
        assert_eq!(registers.get_de(), 0x789A);
        assert_eq!(registers.get_hl(), 0xBCDE);
        assert_eq!(registers.get_af(), 0x12B0);

        registers.set_bc(0x1234);
        assert_eq!(registers.b, 0x12);
        assert_eq!(registers.c, 0x34);

        registers.set_de(0x5678);
        assert_eq!(registers.d, 0x56);
        assert_eq!(registers.e, 0x78);

        registers.set_hl(0x9ABC);
        assert_eq!(registers.h, 0x9A);
        assert_eq!(registers.l, 0xBC);

        registers.set_af(0xDEF0);
        assert_eq!(registers.a, 0xDE);
        assert_eq!(u8::from(registers.f), 0xF0);
    }

    #[test]
    fn test_flags_register_conversion() {
        let flags = FlagsRegister {
            zero: true,
            subtract: false,
            half_carry: true,
            carry: false,
        };

        let byte_value = u8::from(flags);
        assert_eq!(byte_value, 0xA0);

        let flags_from_byte = FlagsRegister::from(0xA0);
        assert!(flags_from_byte.zero);
        assert!(!flags_from_byte.subtract);
        assert!(flags_from_byte.half_carry);
        assert!(!flags_from_byte.carry);
    }

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
    fn test_memory_bus_cartridge_loading() {
        let mut bus = MemoryBus::new();
        let test_rom = vec![0x12, 0x34, 0x56, 0x78];
        bus.load_cartridge(test_rom);

        assert_eq!(bus.read_byte(0x0000), 0x12);
        assert_eq!(bus.read_byte(0x0001), 0x34);
        assert_eq!(bus.read_byte(0x0002), 0x56);
        assert_eq!(bus.read_byte(0x0003), 0x78);
    }

    #[test]
    fn test_memory_bus_write_read() {
        let mut bus = MemoryBus::new();

        bus.write_byte(0x8000, 0x42);
        assert_eq!(bus.read_byte(0x8000), 0x42);

        bus.write_byte(0xC000, 0x55);
        assert_eq!(bus.read_byte(0xC000), 0x55);

        bus.write_byte(0xFF80, 0xAA);
        assert_eq!(bus.read_byte(0xFF80), 0xAA);
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

    #[test]
    fn test_gameboy_integration() {
        let mut gameboy = GameBoy::new();
        let test_rom = vec![
            0x3E, 0x42, // LD A, 0x42
            0x06, 0x10, // LD B, 0x10
            0x80, // ADD A, B
            0x76, // HALT
        ];

        gameboy.load_cartridge(test_rom);

        assert_eq!(gameboy.cpu.pc, 0x0100);

        // Check that memory was loaded correctly
        assert_eq!(gameboy.cpu.bus.read_byte(0x0100), 0x3E);
        assert_eq!(gameboy.cpu.bus.read_byte(0x0101), 0x42);

        gameboy.step();
        assert_eq!(gameboy.cpu.registers.a, 0x42);
        assert_eq!(gameboy.cpu.pc, 0x0102);

        gameboy.step();
        assert_eq!(gameboy.cpu.registers.b, 0x10);
        assert_eq!(gameboy.cpu.pc, 0x0104);

        gameboy.step();
        assert_eq!(gameboy.cpu.registers.a, 0x52);
        assert_eq!(gameboy.cpu.pc, 0x0105);

        gameboy.step();
        assert!(gameboy.cpu.halted);
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
}
