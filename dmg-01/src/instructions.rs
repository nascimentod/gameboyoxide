#[derive(Debug)]
pub enum Instruction {
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
    LDH(LDHTarget),
    BIT(u8, BitTarget),
    RES(u8, BitTarget),
    SET(u8, BitTarget),
    RL(BitTarget),
    RLC(BitTarget),
    RR(BitTarget),
    RRC(BitTarget),
    SLA(BitTarget),
    SRA(BitTarget),
    SRL(BitTarget),
    SWAP(BitTarget),
    
    // Additional critical instructions
    ADC(ArithmeticTarget),  // Add with carry
    SBC(ArithmeticTarget),  // Subtract with carry
    SCF,                    // Set Carry Flag
    CCF,                    // Complement Carry Flag
    DAA,                    // Decimal Adjust Accumulator
    CPL,                    // Complement A
    EI,                     // Enable Interrupts
    DI,                     // Disable Interrupts
    RETI,                   // Return from Interrupt
    RST(u8),                // Reset (call to fixed address)
}

#[derive(Debug)]
pub enum ArithmeticTarget {
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

#[derive(Debug)]
pub enum IncDecTarget {
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

#[derive(Debug)]
pub enum LoadTarget {
    AFromHLIndirect,
    AFromHLIndirectDec,
    AFromHLIndirectInc,
    AFromBCIndirect,
    AFromDEIndirect,
    HLIndirectFromA,
    HLIndirectFromADec,
    HLIndirectFromAInc,
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
    AFromA16,
    A16FromA,
    AFromC,
    CFromA,
    AFromB,
    BFromA,
    AFromD,
    DFromA,
    AFromE,
    EFromA,
    AFromH,
    HFromA,
    AFromL,
    LFromA,
    AFromA,
    
    // Register-to-register loads (missing variants)
    BFromB,
    BFromC,
    BFromD,
    BFromE,
    BFromH,
    BFromL,
    BFromHLIndirect,
    
    CFromB,
    CFromC,
    CFromD,
    CFromE,
    CFromH,
    CFromL,
    CFromHLIndirect,
    
    DFromB,
    DFromC,
    DFromD,
    DFromE,
    DFromH,
    DFromL,
    DFromHLIndirect,
    
    EFromB,
    EFromC,
    EFromD,
    EFromE,
    EFromH,
    EFromL,
    EFromHLIndirect,
    
    HFromB,
    HFromC,
    HFromD,
    HFromE,
    HFromH,
    HFromL,
    HFromHLIndirect,
    
    LFromB,
    LFromC,
    LFromD,
    LFromE,
    LFromH,
    LFromL,
    LFromHLIndirect,
    
    HLIndirectFromB,
    HLIndirectFromC,
    HLIndirectFromD,
    HLIndirectFromE,
    HLIndirectFromH,
    HLIndirectFromL,
}

#[derive(Debug)]
pub enum LDHTarget {
    AFromA8,
    A8FromA,
    AFromC,
    CFromA,
}

#[derive(Debug)]
pub enum BitTarget {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    HLIndirect,
}

#[derive(Debug)]
pub enum StackTarget {
    AF,
    BC,
    DE,
    HL,
}

#[derive(Debug)]
pub enum JumpCondition {
    Always,
    Zero,
    NotZero,
    Carry,
    NotCarry,
}