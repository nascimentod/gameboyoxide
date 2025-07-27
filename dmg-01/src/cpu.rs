use crate::instructions::*;
use crate::memory::MemoryBus;
use crate::registers::Registers;

// Interrupt bit positions
const VBLANK_BIT: u8 = 0;   // V-Blank interrupt (0x40 vector)
const STAT_BIT: u8 = 1;     // LCDC Status interrupt (0x48 vector)
const TIMER_BIT: u8 = 2;    // Timer interrupt (0x50 vector)
const SERIAL_BIT: u8 = 3;   // Serial interrupt (0x58 vector)
const JOYPAD_BIT: u8 = 4;   // Joypad interrupt (0x60 vector)

// Interrupt vectors
const VBLANK_VECTOR: u16 = 0x40;
const STAT_VECTOR: u16 = 0x48;
const TIMER_VECTOR: u16 = 0x50;
const SERIAL_VECTOR: u16 = 0x58;
const JOYPAD_VECTOR: u16 = 0x60;

pub struct CPU {
    pub registers: Registers,
    pub pc: u16,
    pub sp: u16,
    pub bus: MemoryBus,
    pub halted: bool,
    
    // Interrupt system
    pub ime: bool,           // Interrupt Master Enable flag
    pub ie_register: u8,     // Interrupt Enable register (0xFFFF)
    pub if_register: u8,     // Interrupt Flag register (0xFF0F)
    pub ei_delay: bool,      // EI instruction has 1-cycle delay
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: Registers::new(),
            pc: 0x0000,  // Start from boot ROM
            sp: 0xFFFE,
            bus: MemoryBus::new(),
            halted: false,
            
            // Initialize interrupt system
            ime: false,
            ie_register: 0x00,
            if_register: 0x00,
            ei_delay: false,
        }
    }

    pub fn step(&mut self) {
        // Debug: Track Pokemon Red execution progress
        static mut EXECUTION_DEBUG_COUNT: u32 = 0;
        unsafe {
            EXECUTION_DEBUG_COUNT += 1;
            if EXECUTION_DEBUG_COUNT % 50000 == 0 { // Every 50k instructions
                println!("🔄 Pokemon Red executing: PC=0x{:04X}, SP=0x{:04X}, cycles={}", 
                    self.pc, self.sp, EXECUTION_DEBUG_COUNT);
                
                // Check if PC is stuck in a loop  
                static mut LAST_PC: u16 = 0;
                static mut STUCK_COUNT: u32 = 0;
                if LAST_PC == self.pc {
                    STUCK_COUNT += 1;
                    if STUCK_COUNT == 1 {
                        println!("⚠️ CPU might be stuck at PC=0x{:04X}", self.pc);
                        // Show the instruction that's causing the loop
                        let opcode = self.bus.read_byte(self.pc);
                        println!("  Instruction at PC=0x{:04X}: 0x{:02X}", self.pc, opcode);
                        
                        // Show interrupt state
                        println!("  IME={}, IE=0x{:02X}, IF=0x{:02X}, halted={}", 
                            self.ime, self.ie_register, self.if_register, self.halted);
                        
                        // Show if VBLANK interrupt is enabled and pending
                        let vblank_enabled = (self.ie_register & (1 << VBLANK_BIT)) != 0;
                        let vblank_pending = (self.if_register & (1 << VBLANK_BIT)) != 0;
                        println!("  VBLANK: enabled={}, pending={}", vblank_enabled, vblank_pending);
                        
                        // DEBUG: Show memory content around PC to understand the issue
                        println!("  Memory around PC:");
                        for i in 0..8 {
                            let addr = self.pc.wrapping_add(i);
                            let byte = self.bus.read_byte(addr);
                            println!("    0x{:04X}: 0x{:02X}", addr, byte);
                        }
                        
                        // Check if we're reading from boot ROM or cartridge
                        println!("  Boot ROM enabled: {}", self.bus.boot_rom_enabled);
                        if self.pc < 0x0100 && !self.bus.boot_rom_enabled {
                            println!("  WARNING: Reading from cartridge ROM in boot ROM address range!");
                        }
                        
                        // BOOT ROM VBLANK DEBUG: If stuck at 0x0068, check LY register
                        if self.pc == 0x0068 && self.bus.boot_rom_enabled {
                            let ly_value = self.bus.read_byte(0xFF44);
                            println!("  🎮 BOOT ROM VBLANK WAIT: LY={} (waiting for LY=144 to exit loop)", ly_value);
                            println!("  🎮 PPU State: ly={}, mode={:?}, clock={}", 
                                self.bus.ppu.ly, self.bus.ppu.stat.mode, self.bus.ppu.mode_clock);
                        }
                    }
                } else {
                    STUCK_COUNT = 0;
                }
                LAST_PC = self.pc;
            }
        }
        
        // Handle EI delay (EI enables interrupts on the next instruction)
        if self.ei_delay {
            self.ime = true;
            self.ei_delay = false;
        }
        
        // Check for interrupts first
        if self.check_and_handle_interrupts() {
            return; // Interrupt was handled, skip instruction execution
        }
        
        if self.halted {
            // Step PPU even when halted to keep timers running
            self.update_interrupt_flags();
            return;
        }

        let instruction = self.fetch_decode();
        self.execute(instruction);
        
        // Update interrupt flags from peripherals
        self.update_interrupt_flags();
    }

    pub fn fetch_decode(&mut self) -> Instruction {
        let opcode = self.bus.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);

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
            0x0F => Instruction::RRC(BitTarget::A), // RRCA
            0x07 => Instruction::RLC(BitTarget::A), // RLCA
            0x17 => Instruction::RL(BitTarget::A),  // RLA
            0x1F => Instruction::RR(BitTarget::A),  // RRA
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
            0x22 => Instruction::LD(LoadTarget::HLIndirectFromAInc),
            0x23 => Instruction::INC(IncDecTarget::HL),
            0x24 => Instruction::INC(IncDecTarget::H),
            0x25 => Instruction::DEC(IncDecTarget::H),
            0x26 => Instruction::LD(LoadTarget::HFromD8),
            0x28 => Instruction::JR(JumpCondition::Zero),
            0x2A => Instruction::LD(LoadTarget::AFromHLIndirectInc),
            0x2B => Instruction::DEC(IncDecTarget::HL),
            0x2C => Instruction::INC(IncDecTarget::L),
            0x2D => Instruction::DEC(IncDecTarget::L),
            0x2E => Instruction::LD(LoadTarget::LFromD8),
            0x30 => Instruction::JR(JumpCondition::NotCarry),
            0x31 => Instruction::LD(LoadTarget::SPFromD16),
            0x32 => Instruction::LD(LoadTarget::HLIndirectFromADec),
            0x33 => Instruction::INC(IncDecTarget::SP),
            0x34 => Instruction::INC(IncDecTarget::HLIndirect),
            0x35 => Instruction::DEC(IncDecTarget::HLIndirect),
            0x36 => Instruction::LD(LoadTarget::HLIndirectFromD8),
            0x38 => Instruction::JR(JumpCondition::Carry),
            0x3A => Instruction::LD(LoadTarget::AFromHLIndirectDec),
            0x3B => Instruction::DEC(IncDecTarget::SP),
            0x3C => Instruction::INC(IncDecTarget::A),
            0x3D => Instruction::DEC(IncDecTarget::A),
            0x3E => Instruction::LD(LoadTarget::AFromD8),
            
            // More register-to-register loads (0x40-0x7F range)
            0x40 => Instruction::LD(LoadTarget::BFromB), // B <- B (NOP equivalent)
            0x41 => Instruction::LD(LoadTarget::BFromC),
            0x42 => Instruction::LD(LoadTarget::BFromD),
            0x43 => Instruction::LD(LoadTarget::BFromE),
            0x44 => Instruction::LD(LoadTarget::BFromH),
            0x45 => Instruction::LD(LoadTarget::BFromL),
            0x46 => Instruction::LD(LoadTarget::BFromHLIndirect),
            0x47 => Instruction::LD(LoadTarget::BFromA),
            
            0x48 => Instruction::LD(LoadTarget::CFromB),
            0x49 => Instruction::LD(LoadTarget::CFromC), // C <- C (NOP equivalent)
            0x4A => Instruction::LD(LoadTarget::CFromD),
            0x4B => Instruction::LD(LoadTarget::CFromE),
            0x4C => Instruction::LD(LoadTarget::CFromH),
            0x4D => Instruction::LD(LoadTarget::CFromL),
            0x4E => Instruction::LD(LoadTarget::CFromHLIndirect),
            0x4F => Instruction::LD(LoadTarget::CFromA),
            
            0x50 => Instruction::LD(LoadTarget::DFromB),
            0x51 => Instruction::LD(LoadTarget::DFromC),
            0x52 => Instruction::LD(LoadTarget::DFromD), // D <- D (NOP equivalent)
            0x53 => Instruction::LD(LoadTarget::DFromE),
            0x54 => Instruction::LD(LoadTarget::DFromH),
            0x55 => Instruction::LD(LoadTarget::DFromL),
            0x56 => Instruction::LD(LoadTarget::DFromHLIndirect),
            0x57 => Instruction::LD(LoadTarget::DFromA),
            
            0x58 => Instruction::LD(LoadTarget::EFromB),
            0x59 => Instruction::LD(LoadTarget::EFromC),
            0x5A => Instruction::LD(LoadTarget::EFromD),
            0x5B => Instruction::LD(LoadTarget::EFromE), // E <- E (NOP equivalent)
            0x5C => Instruction::LD(LoadTarget::EFromH),
            0x5D => Instruction::LD(LoadTarget::EFromL),
            0x5E => Instruction::LD(LoadTarget::EFromHLIndirect),
            0x5F => Instruction::LD(LoadTarget::EFromA),
            
            0x60 => Instruction::LD(LoadTarget::HFromB),
            0x61 => Instruction::LD(LoadTarget::HFromC),
            0x62 => Instruction::LD(LoadTarget::HFromD),
            0x63 => Instruction::LD(LoadTarget::HFromE),
            0x64 => Instruction::LD(LoadTarget::HFromH), // H <- H (NOP equivalent)
            0x65 => Instruction::LD(LoadTarget::HFromL),
            0x66 => Instruction::LD(LoadTarget::HFromHLIndirect),
            0x67 => Instruction::LD(LoadTarget::HFromA),
            
            0x68 => Instruction::LD(LoadTarget::LFromB),
            0x69 => Instruction::LD(LoadTarget::LFromC),
            0x6A => Instruction::LD(LoadTarget::LFromD),
            0x6B => Instruction::LD(LoadTarget::LFromE),
            0x6C => Instruction::LD(LoadTarget::LFromH),
            0x6D => Instruction::LD(LoadTarget::LFromL), // L <- L (NOP equivalent)
            0x6E => Instruction::LD(LoadTarget::LFromHLIndirect),
            0x6F => Instruction::LD(LoadTarget::LFromA),
            
            0x70 => Instruction::LD(LoadTarget::HLIndirectFromB),
            0x71 => Instruction::LD(LoadTarget::HLIndirectFromC),
            0x72 => Instruction::LD(LoadTarget::HLIndirectFromD),
            0x73 => Instruction::LD(LoadTarget::HLIndirectFromE),
            0x74 => Instruction::LD(LoadTarget::HLIndirectFromH),
            0x75 => Instruction::LD(LoadTarget::HLIndirectFromL),
            0x77 => Instruction::LD(LoadTarget::HLIndirectFromA),
            0x78 => Instruction::LD(LoadTarget::AFromB),
            0x79 => Instruction::LD(LoadTarget::AFromC),
            0x7A => Instruction::LD(LoadTarget::AFromD),
            0x7B => Instruction::LD(LoadTarget::AFromE),
            0x7C => Instruction::LD(LoadTarget::AFromH),
            0x7D => Instruction::LD(LoadTarget::AFromL),
            0x7E => Instruction::LD(LoadTarget::AFromHLIndirect),
            0x7F => Instruction::LD(LoadTarget::AFromA), // A <- A (NOP equivalent)
            0x76 => Instruction::HALT,
            0xCB => {
                let cb_opcode = self.bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.decode_cb_instruction(cb_opcode)
            }
            // ADD A, r8
            0x80 => Instruction::ADD(ArithmeticTarget::B),
            0x81 => Instruction::ADD(ArithmeticTarget::C),
            0x82 => Instruction::ADD(ArithmeticTarget::D),
            0x83 => Instruction::ADD(ArithmeticTarget::E),
            0x84 => Instruction::ADD(ArithmeticTarget::H),
            0x85 => Instruction::ADD(ArithmeticTarget::L),
            0x86 => Instruction::ADD(ArithmeticTarget::HLIndirect),
            0x87 => Instruction::ADD(ArithmeticTarget::A),
            
            // SUB A, r8
            0x90 => Instruction::SUB(ArithmeticTarget::B),
            0x91 => Instruction::SUB(ArithmeticTarget::C),
            0x92 => Instruction::SUB(ArithmeticTarget::D),
            0x93 => Instruction::SUB(ArithmeticTarget::E),
            0x94 => Instruction::SUB(ArithmeticTarget::H),
            0x95 => Instruction::SUB(ArithmeticTarget::L),
            0x96 => Instruction::SUB(ArithmeticTarget::HLIndirect),
            0x97 => Instruction::SUB(ArithmeticTarget::A),
            
            // AND A, r8
            0xA0 => Instruction::AND(ArithmeticTarget::B),
            0xA1 => Instruction::AND(ArithmeticTarget::C),
            0xA2 => Instruction::AND(ArithmeticTarget::D),
            0xA3 => Instruction::AND(ArithmeticTarget::E),
            0xA4 => Instruction::AND(ArithmeticTarget::H),
            0xA5 => Instruction::AND(ArithmeticTarget::L),
            0xA6 => Instruction::AND(ArithmeticTarget::HLIndirect),
            0xA7 => Instruction::AND(ArithmeticTarget::A),
            
            // XOR A, r8
            0xA8 => Instruction::XOR(ArithmeticTarget::B),
            0xA9 => Instruction::XOR(ArithmeticTarget::C),
            0xAA => Instruction::XOR(ArithmeticTarget::D),
            0xAB => Instruction::XOR(ArithmeticTarget::E),
            0xAC => Instruction::XOR(ArithmeticTarget::H),
            0xAD => Instruction::XOR(ArithmeticTarget::L),
            0xAE => Instruction::XOR(ArithmeticTarget::HLIndirect),
            0xAF => Instruction::XOR(ArithmeticTarget::A),
            
            // OR A, r8
            0xB0 => Instruction::OR(ArithmeticTarget::B),
            0xB1 => Instruction::OR(ArithmeticTarget::C),
            0xB2 => Instruction::OR(ArithmeticTarget::D),
            0xB3 => Instruction::OR(ArithmeticTarget::E),
            0xB4 => Instruction::OR(ArithmeticTarget::H),
            0xB5 => Instruction::OR(ArithmeticTarget::L),
            0xB6 => Instruction::OR(ArithmeticTarget::HLIndirect),
            0xB7 => Instruction::OR(ArithmeticTarget::A),
            
            // CP A, r8
            0xB8 => Instruction::CP(ArithmeticTarget::B),
            0xB9 => Instruction::CP(ArithmeticTarget::C),
            0xBA => Instruction::CP(ArithmeticTarget::D),
            0xBB => Instruction::CP(ArithmeticTarget::E),
            0xBC => Instruction::CP(ArithmeticTarget::H),
            0xBD => Instruction::CP(ArithmeticTarget::L),
            0xBE => Instruction::CP(ArithmeticTarget::HLIndirect),
            0xBF => Instruction::CP(ArithmeticTarget::A),
            
            // Return conditions
            0xC0 => Instruction::RET(JumpCondition::NotZero),
            0xC8 => Instruction::RET(JumpCondition::Zero),
            0xC9 => Instruction::RET(JumpCondition::Always),
            0xD0 => Instruction::RET(JumpCondition::NotCarry),
            0xD8 => Instruction::RET(JumpCondition::Carry),
            
            // Stack operations
            0xC1 => Instruction::POP(StackTarget::BC),
            0xC5 => Instruction::PUSH(StackTarget::BC),
            0xD1 => Instruction::POP(StackTarget::DE),
            0xD5 => Instruction::PUSH(StackTarget::DE),
            0xE1 => Instruction::POP(StackTarget::HL),
            0xE5 => Instruction::PUSH(StackTarget::HL),
            0xF1 => Instruction::POP(StackTarget::AF),
            0xF5 => Instruction::PUSH(StackTarget::AF),
            
            // Jump conditions
            0xC2 => Instruction::JP(JumpCondition::NotZero),
            0xC3 => Instruction::JP(JumpCondition::Always),
            0xCA => Instruction::JP(JumpCondition::Zero),
            0xD2 => Instruction::JP(JumpCondition::NotCarry),
            0xDA => Instruction::JP(JumpCondition::Carry),
            
            // Call conditions
            0xC4 => Instruction::CALL(JumpCondition::NotZero),
            0xCC => Instruction::CALL(JumpCondition::Zero),
            0xCD => Instruction::CALL(JumpCondition::Always),
            0xD4 => Instruction::CALL(JumpCondition::NotCarry),
            0xDC => Instruction::CALL(JumpCondition::Carry),
            
            // Arithmetic with immediate
            0xC6 => Instruction::ADD(ArithmeticTarget::D8),
            0xD6 => Instruction::SUB(ArithmeticTarget::D8),
            0xE6 => Instruction::AND(ArithmeticTarget::D8),
            0xEE => Instruction::XOR(ArithmeticTarget::D8),
            0xF6 => Instruction::OR(ArithmeticTarget::D8),
            0xFE => Instruction::CP(ArithmeticTarget::D8),
            
            // LDH instructions
            0xE0 => Instruction::LDH(LDHTarget::A8FromA),
            0xE2 => Instruction::LDH(LDHTarget::CFromA),
            0xF0 => Instruction::LDH(LDHTarget::AFromA8),
            
            // Memory access
            0xEA => Instruction::LD(LoadTarget::A16FromA),
            0xFA => Instruction::LD(LoadTarget::AFromA16),
            
            // ADC instructions (Add with Carry)
            0x88 => Instruction::ADC(ArithmeticTarget::B),
            0x89 => Instruction::ADC(ArithmeticTarget::C),
            0x8A => Instruction::ADC(ArithmeticTarget::D),
            0x8B => Instruction::ADC(ArithmeticTarget::E),
            0x8C => Instruction::ADC(ArithmeticTarget::H),
            0x8D => Instruction::ADC(ArithmeticTarget::L),
            0x8E => Instruction::ADC(ArithmeticTarget::HLIndirect),
            0x8F => Instruction::ADC(ArithmeticTarget::A),
            0xCE => Instruction::ADC(ArithmeticTarget::D8),
            
            // SBC instructions (Subtract with Carry)
            0x98 => Instruction::SBC(ArithmeticTarget::B),
            0x99 => Instruction::SBC(ArithmeticTarget::C),
            0x9A => Instruction::SBC(ArithmeticTarget::D),
            0x9B => Instruction::SBC(ArithmeticTarget::E),
            0x9C => Instruction::SBC(ArithmeticTarget::H),
            0x9D => Instruction::SBC(ArithmeticTarget::L),
            0x9E => Instruction::SBC(ArithmeticTarget::HLIndirect),
            0x9F => Instruction::SBC(ArithmeticTarget::A),
            0xDE => Instruction::SBC(ArithmeticTarget::D8),
            
            // Control flow
            0x18 => Instruction::JR(JumpCondition::Always),
            0x10 => Instruction::NOP, // STOP instruction (treat as NOP for now)
            
            // Misc critical instructions
            0x27 => Instruction::DAA,
            0x2F => Instruction::CPL,
            0x37 => Instruction::SCF,
            0x3F => Instruction::CCF,
            0xF3 => Instruction::DI,
            0xFB => Instruction::EI,
            0xD9 => Instruction::RETI,
            
            // RST instructions (Reset - call to fixed addresses)
            0xC7 => Instruction::RST(0x00),
            0xCF => Instruction::RST(0x08),
            0xD7 => Instruction::RST(0x10),
            0xDF => Instruction::RST(0x18),
            0xE7 => Instruction::RST(0x20),
            0xEF => Instruction::RST(0x28),
            0xF7 => Instruction::RST(0x30),
            0xFF => Instruction::RST(0x38),
            _ => Instruction::NOP,
        }
    }

    fn decode_cb_instruction(&self, cb_opcode: u8) -> Instruction {
        // CB instructions follow a clear pattern:
        // - Lower 3 bits (0-2) determine the target register
        // - Upper bits determine the operation and bit number
        
        let target = match cb_opcode & 0x07 {
            0 => BitTarget::B,
            1 => BitTarget::C,
            2 => BitTarget::D,
            3 => BitTarget::E,
            4 => BitTarget::H,
            5 => BitTarget::L,
            6 => BitTarget::HLIndirect,
            7 => BitTarget::A,
            _ => unreachable!(),
        };

        match cb_opcode {
            // RLC (Rotate Left Circular) - 0x00-0x07
            0x00..=0x07 => Instruction::RLC(target),
            
            // RRC (Rotate Right Circular) - 0x08-0x0F
            0x08..=0x0F => Instruction::RRC(target),
            
            // RL (Rotate Left through Carry) - 0x10-0x17
            0x10..=0x17 => Instruction::RL(target),
            
            // RR (Rotate Right through Carry) - 0x18-0x1F
            0x18..=0x1F => Instruction::RR(target),
            
            // SLA (Shift Left Arithmetic) - 0x20-0x27
            0x20..=0x27 => Instruction::SLA(target),
            
            // SRA (Shift Right Arithmetic) - 0x28-0x2F
            0x28..=0x2F => Instruction::SRA(target),
            
            // SWAP (Swap nibbles) - 0x30-0x37
            0x30..=0x37 => Instruction::SWAP(target),
            
            // SRL (Shift Right Logical) - 0x38-0x3F
            0x38..=0x3F => Instruction::SRL(target),
            
            // BIT instructions (Test bit) - 0x40-0x7F
            0x40..=0x7F => {
                let bit = (cb_opcode - 0x40) / 8;
                Instruction::BIT(bit, target)
            },
            
            // RES instructions (Reset bit) - 0x80-0xBF
            0x80..=0xBF => {
                let bit = (cb_opcode - 0x80) / 8;
                Instruction::RES(bit, target)
            },
            
            // SET instructions (Set bit) - 0xC0-0xFF
            0xC0..=0xFF => {
                let bit = (cb_opcode - 0xC0) / 8;
                Instruction::SET(bit, target)
            },
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
                let result = self.registers.a.wrapping_sub(value);
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (self.registers.a & 0x0F) < (value & 0x0F);
                self.registers.f.carry = self.registers.a < value;
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
            Instruction::LDH(target) => {
                match target {
                    LDHTarget::A8FromA => {
                        let offset = self.bus.read_byte(self.pc);
                        self.pc = self.pc.wrapping_add(1);
                        self.bus.write_byte(0xFF00 + offset as u16, self.registers.a);
                    }
                    LDHTarget::AFromA8 => {
                        let offset = self.bus.read_byte(self.pc);
                        self.pc = self.pc.wrapping_add(1);
                        self.registers.a = self.bus.read_byte(0xFF00 + offset as u16);
                    }
                    LDHTarget::CFromA => {
                        self.bus.write_byte(0xFF00 + self.registers.c as u16, self.registers.a);
                    }
                    LDHTarget::AFromC => {
                        self.registers.a = self.bus.read_byte(0xFF00 + self.registers.c as u16);
                    }
                }
            },
            Instruction::BIT(bit, target) => {
                let value = match target {
                    BitTarget::A => self.registers.a,
                    BitTarget::B => self.registers.b,
                    BitTarget::C => self.registers.c,
                    BitTarget::D => self.registers.d,
                    BitTarget::E => self.registers.e,
                    BitTarget::H => self.registers.h,
                    BitTarget::L => self.registers.l,
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        self.bus.read_byte(address)
                    }
                };
                
                let result = value & (1 << bit);
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = true;
            },
            Instruction::RES(bit, target) => {
                let mask = !(1 << bit);
                match target {
                    BitTarget::A => self.registers.a &= mask,
                    BitTarget::B => self.registers.b &= mask,
                    BitTarget::C => self.registers.c &= mask,
                    BitTarget::D => self.registers.d &= mask,
                    BitTarget::E => self.registers.e &= mask,
                    BitTarget::H => self.registers.h &= mask,
                    BitTarget::L => self.registers.l &= mask,
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        self.bus.write_byte(address, value & mask);
                    }
                }
            },
            Instruction::SET(bit, target) => {
                let mask = 1 << bit;
                match target {
                    BitTarget::A => self.registers.a |= mask,
                    BitTarget::B => self.registers.b |= mask,
                    BitTarget::C => self.registers.c |= mask,
                    BitTarget::D => self.registers.d |= mask,
                    BitTarget::E => self.registers.e |= mask,
                    BitTarget::H => self.registers.h |= mask,
                    BitTarget::L => self.registers.l |= mask,
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        self.bus.write_byte(address, value | mask);
                    }
                }
            },
            Instruction::RL(target) => {
                let (value, result) = match target {
                    BitTarget::A => {
                        let old_carry = if self.registers.f.carry { 1 } else { 0 };
                        let new_carry = (self.registers.a & 0x80) != 0;
                        let result = (self.registers.a << 1) | old_carry;
                        self.registers.a = result;
                        self.registers.f.carry = new_carry;
                        (self.registers.a, result)
                    }
                    BitTarget::C => {
                        let old_carry = if self.registers.f.carry { 1 } else { 0 };
                        let new_carry = (self.registers.c & 0x80) != 0;
                        let result = (self.registers.c << 1) | old_carry;
                        self.registers.c = result;
                        self.registers.f.carry = new_carry;
                        (self.registers.c, result)
                    }
                    _ => {
                        // TODO: Implement other targets
                        (0, 0)
                    }
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
            },
            Instruction::RLC(target) => {
                let (result, carry) = match target {
                    BitTarget::A => {
                        let carry = (self.registers.a & 0x80) != 0;
                        let result = (self.registers.a << 1) | if carry { 1 } else { 0 };
                        self.registers.a = result;
                        (result, carry)
                    }
                    BitTarget::B => {
                        let carry = (self.registers.b & 0x80) != 0;
                        let result = (self.registers.b << 1) | if carry { 1 } else { 0 };
                        self.registers.b = result;
                        (result, carry)
                    }
                    BitTarget::C => {
                        let carry = (self.registers.c & 0x80) != 0;
                        let result = (self.registers.c << 1) | if carry { 1 } else { 0 };
                        self.registers.c = result;
                        (result, carry)
                    }
                    BitTarget::D => {
                        let carry = (self.registers.d & 0x80) != 0;
                        let result = (self.registers.d << 1) | if carry { 1 } else { 0 };
                        self.registers.d = result;
                        (result, carry)
                    }
                    BitTarget::E => {
                        let carry = (self.registers.e & 0x80) != 0;
                        let result = (self.registers.e << 1) | if carry { 1 } else { 0 };
                        self.registers.e = result;
                        (result, carry)
                    }
                    BitTarget::H => {
                        let carry = (self.registers.h & 0x80) != 0;
                        let result = (self.registers.h << 1) | if carry { 1 } else { 0 };
                        self.registers.h = result;
                        (result, carry)
                    }
                    BitTarget::L => {
                        let carry = (self.registers.l & 0x80) != 0;
                        let result = (self.registers.l << 1) | if carry { 1 } else { 0 };
                        self.registers.l = result;
                        (result, carry)
                    }
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        let carry = (value & 0x80) != 0;
                        let result = (value << 1) | if carry { 1 } else { 0 };
                        self.bus.write_byte(address, result);
                        (result, carry)
                    }
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = carry;
            },
            Instruction::RR(target) => {
                let (result, carry) = match target {
                    BitTarget::A => {
                        let carry = (self.registers.a & 0x01) != 0;
                        let old_carry = if self.registers.f.carry { 0x80 } else { 0 };
                        let result = (self.registers.a >> 1) | old_carry;
                        self.registers.a = result;
                        (result, carry)
                    }
                    BitTarget::B => {
                        let carry = (self.registers.b & 0x01) != 0;
                        let old_carry = if self.registers.f.carry { 0x80 } else { 0 };
                        let result = (self.registers.b >> 1) | old_carry;
                        self.registers.b = result;
                        (result, carry)
                    }
                    BitTarget::C => {
                        let carry = (self.registers.c & 0x01) != 0;
                        let old_carry = if self.registers.f.carry { 0x80 } else { 0 };
                        let result = (self.registers.c >> 1) | old_carry;
                        self.registers.c = result;
                        (result, carry)
                    }
                    BitTarget::D => {
                        let carry = (self.registers.d & 0x01) != 0;
                        let old_carry = if self.registers.f.carry { 0x80 } else { 0 };
                        let result = (self.registers.d >> 1) | old_carry;
                        self.registers.d = result;
                        (result, carry)
                    }
                    BitTarget::E => {
                        let carry = (self.registers.e & 0x01) != 0;
                        let old_carry = if self.registers.f.carry { 0x80 } else { 0 };
                        let result = (self.registers.e >> 1) | old_carry;
                        self.registers.e = result;
                        (result, carry)
                    }
                    BitTarget::H => {
                        let carry = (self.registers.h & 0x01) != 0;
                        let old_carry = if self.registers.f.carry { 0x80 } else { 0 };
                        let result = (self.registers.h >> 1) | old_carry;
                        self.registers.h = result;
                        (result, carry)
                    }
                    BitTarget::L => {
                        let carry = (self.registers.l & 0x01) != 0;
                        let old_carry = if self.registers.f.carry { 0x80 } else { 0 };
                        let result = (self.registers.l >> 1) | old_carry;
                        self.registers.l = result;
                        (result, carry)
                    }
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        let carry = (value & 0x01) != 0;
                        let old_carry = if self.registers.f.carry { 0x80 } else { 0 };
                        let result = (value >> 1) | old_carry;
                        self.bus.write_byte(address, result);
                        (result, carry)
                    }
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = carry;
            },
            Instruction::RRC(target) => {
                let (result, carry) = match target {
                    BitTarget::A => {
                        let carry = (self.registers.a & 0x01) != 0;
                        let result = (self.registers.a >> 1) | if carry { 0x80 } else { 0 };
                        self.registers.a = result;
                        (result, carry)
                    }
                    BitTarget::B => {
                        let carry = (self.registers.b & 0x01) != 0;
                        let result = (self.registers.b >> 1) | if carry { 0x80 } else { 0 };
                        self.registers.b = result;
                        (result, carry)
                    }
                    BitTarget::C => {
                        let carry = (self.registers.c & 0x01) != 0;
                        let result = (self.registers.c >> 1) | if carry { 0x80 } else { 0 };
                        self.registers.c = result;
                        (result, carry)
                    }
                    BitTarget::D => {
                        let carry = (self.registers.d & 0x01) != 0;
                        let result = (self.registers.d >> 1) | if carry { 0x80 } else { 0 };
                        self.registers.d = result;
                        (result, carry)
                    }
                    BitTarget::E => {
                        let carry = (self.registers.e & 0x01) != 0;
                        let result = (self.registers.e >> 1) | if carry { 0x80 } else { 0 };
                        self.registers.e = result;
                        (result, carry)
                    }
                    BitTarget::H => {
                        let carry = (self.registers.h & 0x01) != 0;
                        let result = (self.registers.h >> 1) | if carry { 0x80 } else { 0 };
                        self.registers.h = result;
                        (result, carry)
                    }
                    BitTarget::L => {
                        let carry = (self.registers.l & 0x01) != 0;
                        let result = (self.registers.l >> 1) | if carry { 0x80 } else { 0 };
                        self.registers.l = result;
                        (result, carry)
                    }
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        let carry = (value & 0x01) != 0;
                        let result = (value >> 1) | if carry { 0x80 } else { 0 };
                        self.bus.write_byte(address, result);
                        (result, carry)
                    }
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = carry;
            },
            Instruction::SLA(target) => {
                // Shift Left Arithmetic - shift left, insert 0, MSB to carry
                let (result, carry) = match target {
                    BitTarget::A => {
                        let carry = (self.registers.a & 0x80) != 0;
                        let result = self.registers.a << 1;
                        self.registers.a = result;
                        (result, carry)
                    }
                    BitTarget::B => {
                        let carry = (self.registers.b & 0x80) != 0;
                        let result = self.registers.b << 1;
                        self.registers.b = result;
                        (result, carry)
                    }
                    BitTarget::C => {
                        let carry = (self.registers.c & 0x80) != 0;
                        let result = self.registers.c << 1;
                        self.registers.c = result;
                        (result, carry)
                    }
                    BitTarget::D => {
                        let carry = (self.registers.d & 0x80) != 0;
                        let result = self.registers.d << 1;
                        self.registers.d = result;
                        (result, carry)
                    }
                    BitTarget::E => {
                        let carry = (self.registers.e & 0x80) != 0;
                        let result = self.registers.e << 1;
                        self.registers.e = result;
                        (result, carry)
                    }
                    BitTarget::H => {
                        let carry = (self.registers.h & 0x80) != 0;
                        let result = self.registers.h << 1;
                        self.registers.h = result;
                        (result, carry)
                    }
                    BitTarget::L => {
                        let carry = (self.registers.l & 0x80) != 0;
                        let result = self.registers.l << 1;
                        self.registers.l = result;
                        (result, carry)
                    }
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        let carry = (value & 0x80) != 0;
                        let result = value << 1;
                        self.bus.write_byte(address, result);
                        (result, carry)
                    }
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = carry;
            },
            Instruction::SRA(target) => {
                // Shift Right Arithmetic - shift right, preserve MSB, LSB to carry
                let (result, carry) = match target {
                    BitTarget::A => {
                        let carry = (self.registers.a & 0x01) != 0;
                        let result = (self.registers.a >> 1) | (self.registers.a & 0x80);
                        self.registers.a = result;
                        (result, carry)
                    }
                    BitTarget::B => {
                        let carry = (self.registers.b & 0x01) != 0;
                        let result = (self.registers.b >> 1) | (self.registers.b & 0x80);
                        self.registers.b = result;
                        (result, carry)
                    }
                    BitTarget::C => {
                        let carry = (self.registers.c & 0x01) != 0;
                        let result = (self.registers.c >> 1) | (self.registers.c & 0x80);
                        self.registers.c = result;
                        (result, carry)
                    }
                    BitTarget::D => {
                        let carry = (self.registers.d & 0x01) != 0;
                        let result = (self.registers.d >> 1) | (self.registers.d & 0x80);
                        self.registers.d = result;
                        (result, carry)
                    }
                    BitTarget::E => {
                        let carry = (self.registers.e & 0x01) != 0;
                        let result = (self.registers.e >> 1) | (self.registers.e & 0x80);
                        self.registers.e = result;
                        (result, carry)
                    }
                    BitTarget::H => {
                        let carry = (self.registers.h & 0x01) != 0;
                        let result = (self.registers.h >> 1) | (self.registers.h & 0x80);
                        self.registers.h = result;
                        (result, carry)
                    }
                    BitTarget::L => {
                        let carry = (self.registers.l & 0x01) != 0;
                        let result = (self.registers.l >> 1) | (self.registers.l & 0x80);
                        self.registers.l = result;
                        (result, carry)
                    }
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        let carry = (value & 0x01) != 0;
                        let result = (value >> 1) | (value & 0x80);
                        self.bus.write_byte(address, result);
                        (result, carry)
                    }
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = carry;
            },
            Instruction::SRL(target) => {
                // Shift Right Logical - shift right, insert 0, LSB to carry
                let (result, carry) = match target {
                    BitTarget::A => {
                        let carry = (self.registers.a & 0x01) != 0;
                        let result = self.registers.a >> 1;
                        self.registers.a = result;
                        (result, carry)
                    }
                    BitTarget::B => {
                        let carry = (self.registers.b & 0x01) != 0;
                        let result = self.registers.b >> 1;
                        self.registers.b = result;
                        (result, carry)
                    }
                    BitTarget::C => {
                        let carry = (self.registers.c & 0x01) != 0;
                        let result = self.registers.c >> 1;
                        self.registers.c = result;
                        (result, carry)
                    }
                    BitTarget::D => {
                        let carry = (self.registers.d & 0x01) != 0;
                        let result = self.registers.d >> 1;
                        self.registers.d = result;
                        (result, carry)
                    }
                    BitTarget::E => {
                        let carry = (self.registers.e & 0x01) != 0;
                        let result = self.registers.e >> 1;
                        self.registers.e = result;
                        (result, carry)
                    }
                    BitTarget::H => {
                        let carry = (self.registers.h & 0x01) != 0;
                        let result = self.registers.h >> 1;
                        self.registers.h = result;
                        (result, carry)
                    }
                    BitTarget::L => {
                        let carry = (self.registers.l & 0x01) != 0;
                        let result = self.registers.l >> 1;
                        self.registers.l = result;
                        (result, carry)
                    }
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        let carry = (value & 0x01) != 0;
                        let result = value >> 1;
                        self.bus.write_byte(address, result);
                        (result, carry)
                    }
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = carry;
            },
            Instruction::SWAP(target) => {
                let result = match target {
                    BitTarget::A => {
                        let result = (self.registers.a << 4) | (self.registers.a >> 4);
                        self.registers.a = result;
                        result
                    }
                    BitTarget::B => {
                        let result = (self.registers.b << 4) | (self.registers.b >> 4);
                        self.registers.b = result;
                        result
                    }
                    BitTarget::C => {
                        let result = (self.registers.c << 4) | (self.registers.c >> 4);
                        self.registers.c = result;
                        result
                    }
                    BitTarget::D => {
                        let result = (self.registers.d << 4) | (self.registers.d >> 4);
                        self.registers.d = result;
                        result
                    }
                    BitTarget::E => {
                        let result = (self.registers.e << 4) | (self.registers.e >> 4);
                        self.registers.e = result;
                        result
                    }
                    BitTarget::H => {
                        let result = (self.registers.h << 4) | (self.registers.h >> 4);
                        self.registers.h = result;
                        result
                    }
                    BitTarget::L => {
                        let result = (self.registers.l << 4) | (self.registers.l >> 4);
                        self.registers.l = result;
                        result
                    }
                    BitTarget::HLIndirect => {
                        let address = self.registers.get_hl();
                        let value = self.bus.read_byte(address);
                        let result = (value << 4) | (value >> 4);
                        self.bus.write_byte(address, result);
                        result
                    }
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = false;
            },
            Instruction::ADC(target) => {
                let value = self.get_arithmetic_target_value(target);
                let carry = if self.registers.f.carry { 1 } else { 0 };
                let (temp_result, carry1) = self.registers.a.overflowing_add(value);
                let (final_result, carry2) = temp_result.overflowing_add(carry);
                
                self.registers.f.zero = final_result == 0;
                self.registers.f.subtract = false;
                self.registers.f.carry = carry1 || carry2;
                self.registers.f.half_carry = (self.registers.a & 0xF) + (value & 0xF) + carry > 0xF;
                self.registers.a = final_result;
            },
            Instruction::SBC(target) => {
                let value = self.get_arithmetic_target_value(target);
                let carry = if self.registers.f.carry { 1 } else { 0 };
                let (temp_result, borrow1) = self.registers.a.overflowing_sub(value);
                let (final_result, borrow2) = temp_result.overflowing_sub(carry);
                
                self.registers.f.zero = final_result == 0;
                self.registers.f.subtract = true;
                self.registers.f.carry = borrow1 || borrow2;
                self.registers.f.half_carry = (self.registers.a & 0xF) < (value & 0xF) + carry;
                self.registers.a = final_result;
            },
            Instruction::DAA => {
                // Decimal Adjust Accumulator - converts binary result to BCD
                let mut result = self.registers.a;
                let mut adjust = 0;
                
                if self.registers.f.half_carry || (!self.registers.f.subtract && (result & 0x0F) > 9) {
                    adjust |= 0x06;
                }
                
                if self.registers.f.carry || (!self.registers.f.subtract && result > 0x99) {
                    adjust |= 0x60;
                    self.registers.f.carry = true;
                }
                
                result = if self.registers.f.subtract {
                    result.wrapping_sub(adjust)
                } else {
                    result.wrapping_add(adjust)
                };
                
                self.registers.f.zero = result == 0;
                self.registers.f.half_carry = false;
                self.registers.a = result;
            },
            Instruction::CPL => {
                // Complement A (flip all bits)
                self.registers.a = !self.registers.a;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = true;
            },
            Instruction::SCF => {
                // Set Carry Flag
                self.registers.f.carry = true;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
            },
            Instruction::CCF => {
                // Complement Carry Flag
                self.registers.f.carry = !self.registers.f.carry;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
            },
            Instruction::DI => {
                // Disable Interrupts
                self.ime = false;
            },
            Instruction::EI => {
                // Enable Interrupts (takes effect after next instruction)
                self.ei_delay = true;
            },
            Instruction::RETI => {
                // Return from Interrupt - pop PC and enable interrupts immediately
                let low = self.bus.read_byte(self.sp) as u16;
                self.sp = self.sp.wrapping_add(1);
                let high = self.bus.read_byte(self.sp) as u16;
                self.sp = self.sp.wrapping_add(1);
                self.pc = (high << 8) | low;
                self.ime = true; // RETI enables interrupts immediately
            },
            Instruction::RST(address) => {
                // Reset - push PC and jump to fixed address
                self.sp = self.sp.wrapping_sub(1);
                self.bus.write_byte(self.sp, ((self.pc & 0xFF00) >> 8) as u8);
                self.sp = self.sp.wrapping_sub(1);
                self.bus.write_byte(self.sp, (self.pc & 0xFF) as u8);
                self.pc = address as u16;
            },
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
                self.pc = self.pc.wrapping_add(1);
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
            LoadTarget::AFromHLIndirectDec => {
                let address = self.registers.get_hl();
                self.registers.a = self.bus.read_byte(address);
                self.registers.set_hl(address.wrapping_sub(1));
            }
            LoadTarget::AFromHLIndirectInc => {
                let address = self.registers.get_hl();
                self.registers.a = self.bus.read_byte(address);
                self.registers.set_hl(address.wrapping_add(1));
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
            LoadTarget::HLIndirectFromADec => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.a);
                self.registers.set_hl(address.wrapping_sub(1));
            }
            LoadTarget::HLIndirectFromAInc => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.a);
                self.registers.set_hl(address.wrapping_add(1));
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
                self.pc = self.pc.wrapping_add(1);
                self.registers.a = value;
            }
            LoadTarget::BFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.registers.b = value;
            }
            LoadTarget::CFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.registers.c = value;
            }
            LoadTarget::DFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.registers.d = value;
            }
            LoadTarget::EFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.registers.e = value;
            }
            LoadTarget::HFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.registers.h = value;
            }
            LoadTarget::LFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                self.registers.l = value;
            }
            LoadTarget::HLIndirectFromD8 => {
                let value = self.bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                let address = self.registers.get_hl();
                self.bus.write_byte(address, value);
            }
            LoadTarget::BCFromD16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.registers.set_bc((high << 8) | low);
            }
            LoadTarget::DEFromD16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.registers.set_de((high << 8) | low);
            }
            LoadTarget::HLFromD16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.registers.set_hl((high << 8) | low);
            }
            LoadTarget::SPFromD16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                self.sp = (high << 8) | low;
            }
            LoadTarget::AFromA16 => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                let address = (high << 8) | low;
                self.registers.a = self.bus.read_byte(address);
            }
            LoadTarget::A16FromA => {
                let low = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                let high = self.bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                let address = (high << 8) | low;
                self.bus.write_byte(address, self.registers.a);
            }
            LoadTarget::AFromB => {
                self.registers.a = self.registers.b;
            }
            LoadTarget::AFromC => {
                self.registers.a = self.registers.c;
            }
            LoadTarget::AFromD => {
                self.registers.a = self.registers.d;
            }
            LoadTarget::AFromE => {
                self.registers.a = self.registers.e;
            }
            LoadTarget::AFromH => {
                self.registers.a = self.registers.h;
            }
            LoadTarget::AFromL => {
                self.registers.a = self.registers.l;
            }
            LoadTarget::BFromA => {
                self.registers.b = self.registers.a;
            }
            LoadTarget::CFromA => {
                self.registers.c = self.registers.a;
            }
            LoadTarget::DFromA => {
                self.registers.d = self.registers.a;
            }
            LoadTarget::EFromA => {
                self.registers.e = self.registers.a;
            }
            LoadTarget::HFromA => {
                self.registers.h = self.registers.a;
            }
            LoadTarget::LFromA => {
                self.registers.l = self.registers.a;
            }
            LoadTarget::AFromA => { /* NOP - A is already A */ }
            
            // Register-to-register loads implementation
            LoadTarget::BFromB => { /* NOP - B is already B */ }
            LoadTarget::BFromC => { self.registers.b = self.registers.c; }
            LoadTarget::BFromD => { self.registers.b = self.registers.d; }
            LoadTarget::BFromE => { self.registers.b = self.registers.e; }
            LoadTarget::BFromH => { self.registers.b = self.registers.h; }
            LoadTarget::BFromL => { self.registers.b = self.registers.l; }
            LoadTarget::BFromHLIndirect => {
                let address = self.registers.get_hl();
                self.registers.b = self.bus.read_byte(address);
            }
            
            LoadTarget::CFromB => { self.registers.c = self.registers.b; }
            LoadTarget::CFromC => { /* NOP - C is already C */ }
            LoadTarget::CFromD => { self.registers.c = self.registers.d; }
            LoadTarget::CFromE => { self.registers.c = self.registers.e; }
            LoadTarget::CFromH => { self.registers.c = self.registers.h; }
            LoadTarget::CFromL => { self.registers.c = self.registers.l; }
            LoadTarget::CFromHLIndirect => {
                let address = self.registers.get_hl();
                self.registers.c = self.bus.read_byte(address);
            }
            
            LoadTarget::DFromB => { self.registers.d = self.registers.b; }
            LoadTarget::DFromC => { self.registers.d = self.registers.c; }
            LoadTarget::DFromD => { /* NOP - D is already D */ }
            LoadTarget::DFromE => { self.registers.d = self.registers.e; }
            LoadTarget::DFromH => { self.registers.d = self.registers.h; }
            LoadTarget::DFromL => { self.registers.d = self.registers.l; }
            LoadTarget::DFromHLIndirect => {
                let address = self.registers.get_hl();
                self.registers.d = self.bus.read_byte(address);
            }
            
            LoadTarget::EFromB => { self.registers.e = self.registers.b; }
            LoadTarget::EFromC => { self.registers.e = self.registers.c; }
            LoadTarget::EFromD => { self.registers.e = self.registers.d; }
            LoadTarget::EFromE => { /* NOP - E is already E */ }
            LoadTarget::EFromH => { self.registers.e = self.registers.h; }
            LoadTarget::EFromL => { self.registers.e = self.registers.l; }
            LoadTarget::EFromHLIndirect => {
                let address = self.registers.get_hl();
                self.registers.e = self.bus.read_byte(address);
            }
            
            LoadTarget::HFromB => { self.registers.h = self.registers.b; }
            LoadTarget::HFromC => { self.registers.h = self.registers.c; }
            LoadTarget::HFromD => { self.registers.h = self.registers.d; }
            LoadTarget::HFromE => { self.registers.h = self.registers.e; }
            LoadTarget::HFromH => { /* NOP - H is already H */ }
            LoadTarget::HFromL => { self.registers.h = self.registers.l; }
            LoadTarget::HFromHLIndirect => {
                let address = self.registers.get_hl();
                self.registers.h = self.bus.read_byte(address);
            }
            
            LoadTarget::LFromB => { self.registers.l = self.registers.b; }
            LoadTarget::LFromC => { self.registers.l = self.registers.c; }
            LoadTarget::LFromD => { self.registers.l = self.registers.d; }
            LoadTarget::LFromE => { self.registers.l = self.registers.e; }
            LoadTarget::LFromH => { self.registers.l = self.registers.h; }
            LoadTarget::LFromL => { /* NOP - L is already L */ }
            LoadTarget::LFromHLIndirect => {
                let address = self.registers.get_hl();
                self.registers.l = self.bus.read_byte(address);
            }
            
            LoadTarget::HLIndirectFromB => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.b);
            }
            LoadTarget::HLIndirectFromC => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.c);
            }
            LoadTarget::HLIndirectFromD => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.d);
            }
            LoadTarget::HLIndirectFromE => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.e);
            }
            LoadTarget::HLIndirectFromH => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.h);
            }
            LoadTarget::HLIndirectFromL => {
                let address = self.registers.get_hl();
                self.bus.write_byte(address, self.registers.l);
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
            self.pc = self.pc.wrapping_add(1);
            let high = self.bus.read_byte(self.pc) as u16;
            self.pc = self.pc.wrapping_add(1);
            self.pc = (high << 8) | low;
        } else {
            self.pc = self.pc.wrapping_add(2);
        }
    }

    fn jump_relative(&mut self, condition: JumpCondition) {
        if self.check_condition(condition) {
            let offset = self.bus.read_byte(self.pc) as i8;
            self.pc = self.pc.wrapping_add(1);
            self.pc = ((self.pc as i32) + (offset as i32)) as u16;
        } else {
            self.pc = self.pc.wrapping_add(1);
        }
    }

    fn call(&mut self, condition: JumpCondition) {
        if self.check_condition(condition) {
            let low = self.bus.read_byte(self.pc) as u16;
            self.pc = self.pc.wrapping_add(1);
            let high = self.bus.read_byte(self.pc) as u16;
            self.pc = self.pc.wrapping_add(1);
            let address = (high << 8) | low;

            self.sp = self.sp.wrapping_sub(1);
            self.bus
                .write_byte(self.sp, ((self.pc & 0xFF00) >> 8) as u8);
            self.sp = self.sp.wrapping_sub(1);
            self.bus.write_byte(self.sp, (self.pc & 0xFF) as u8);

            self.pc = address;
        } else {
            self.pc = self.pc.wrapping_add(2);
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

    fn update_interrupt_flags(&mut self) {
        // Sync interrupt registers between CPU and memory bus
        self.bus.ie_register = self.ie_register;
        self.bus.if_register = self.if_register;
        
        // BOOT ROM TIMING FIX: Use more accurate cycle timing during boot ROM VBLANK wait
        // The boot ROM gets stuck because it executes too fast relative to PPU timing
        let cycles = if self.bus.boot_rom_enabled && (self.pc >= 0x0064 && self.pc <= 0x0069) {
            // Boot ROM VBLANK wait loop - use more cycles to slow down CPU relative to PPU
            // This ensures LY=144 window lasts long enough for detection
            8  // Increase from 4 to 8 cycles for proper timing
        } else {
            4  // Normal timing for other code
        };
        
        let (vblank_interrupt, stat_interrupt, timer_interrupt, joypad_interrupt) = self.bus.step(cycles);
        
        // Update interrupt flags based on peripheral events
        if vblank_interrupt {
            self.request_interrupt(VBLANK_BIT);
        }
        if stat_interrupt {
            self.request_interrupt(STAT_BIT);
        }
        if timer_interrupt {
            self.request_interrupt(TIMER_BIT);
        }
        if joypad_interrupt {
            self.request_interrupt(JOYPAD_BIT);
        }
        
        // Sync interrupt registers back to memory bus before reading back
        self.bus.ie_register = self.ie_register;
        self.bus.if_register = self.if_register;
        
        // Note: Don't overwrite CPU interrupt registers with memory bus values
        // since the CPU is the source of truth for interrupt state
        
        // TODO: Add joypad interrupt source when implemented
    }

    fn request_interrupt(&mut self, interrupt_bit: u8) {
        self.if_register |= 1 << interrupt_bit;
    }

    fn check_and_handle_interrupts(&mut self) -> bool {
        // Check if any interrupt is both enabled and requested
        let pending_interrupts = self.ie_register & self.if_register;
        
        if pending_interrupts == 0 {
            return false; // No interrupts pending
        }
        
        // If halted, any pending interrupt wakes up the CPU (even if IME is disabled)
        if self.halted {
            self.halted = false;
            if !self.ime {
                return false; // Wake up but don't handle interrupt if IME disabled
            }
        }
        
        // Only handle interrupts if IME is enabled
        if !self.ime {
            return false;
        }
        
        // Handle interrupts in priority order (lower bits have higher priority)
        for i in 0..5 {
            if (pending_interrupts & (1 << i)) != 0 {
                self.handle_interrupt(i);
                return true;
            }
        }
        
        false
    }

    fn handle_interrupt(&mut self, interrupt_bit: u8) {
        // Disable further interrupts
        self.ime = false;
        
        // Clear the interrupt flag
        self.if_register &= !(1 << interrupt_bit);
        
        // Push current PC to stack
        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, ((self.pc & 0xFF00) >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, (self.pc & 0xFF) as u8);
        
        // Jump to interrupt vector
        self.pc = match interrupt_bit {
            VBLANK_BIT => VBLANK_VECTOR,
            STAT_BIT => STAT_VECTOR,
            TIMER_BIT => TIMER_VECTOR,
            SERIAL_BIT => SERIAL_VECTOR,
            JOYPAD_BIT => JOYPAD_VECTOR,
            _ => panic!("Invalid interrupt bit: {}", interrupt_bit),
        };
    }

    // Public API for accessing interrupt registers
    pub fn read_ie_register(&self) -> u8 {
        self.ie_register
    }

    pub fn write_ie_register(&mut self, value: u8) {
        self.ie_register = value;
    }

    pub fn read_if_register(&self) -> u8 {
        self.if_register | 0xE0 // Upper 3 bits always read as 1
    }

    pub fn write_if_register(&mut self, value: u8) {
        self.if_register = value & 0x1F; // Only lower 5 bits are writable
    }

    // Joypad input methods
    pub fn press_button(&mut self, button: crate::joypad::JoypadButton) {
        self.bus.press_button(button);
    }

    pub fn release_button(&mut self, button: crate::joypad::JoypadButton) {
        self.bus.release_button(button);
    }

    pub fn get_joypad_state(&self) -> crate::joypad::JoypadState {
        self.bus.get_joypad_state()
    }
}