use dmg_01::registers::{Registers, FlagsRegister};

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