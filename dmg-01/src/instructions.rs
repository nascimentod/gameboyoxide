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
}

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

pub enum LoadTarget {
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

pub enum StackTarget {
    AF,
    BC,
    DE,
    HL,
}

pub enum JumpCondition {
    Always,
    Zero,
    NotZero,
    Carry,
    NotCarry,
}