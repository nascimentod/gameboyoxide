//! CPU timing changes needed by the APU are verified independently of the ROM.
use dmg_01::cpu::CPU;

fn cpu(code: &[u8]) -> CPU {
    let mut cpu = CPU::new();
    let mut rom = vec![0; 0x8000];
    rom[0x100..0x100 + code.len()].copy_from_slice(code);
    cpu.bus.load_cartridge(rom);
    cpu.bus.boot_rom_enabled = false;
    cpu.pc = 0x100;
    cpu
}
fn step_cycles(cpu: &mut CPU) -> u64 {
    let before = cpu.bus.elapsed_cycles;
    cpu.step();
    cpu.bus.elapsed_cycles - before
}

#[test]
fn instruction_durations_are_t_cycles_not_instruction_counts() {
    let mut c = cpu(&[0, 0x3e, 0x12, 0x01, 0x00, 0xc0, 0x02, 0x03]);
    for expected in [4, 8, 12, 8, 8] {
        assert_eq!(step_cycles(&mut c), expected);
    }
    assert_eq!(c.bus.timer.div_counter, 40);
}

#[test]
fn conditional_branch_call_and_return_durations() {
    let mut c = cpu(&[0x20, 0, 0x28, 0, 0xc4, 0x0a, 1, 0, 0, 0, 0xc0]);
    c.registers.f.zero = false;
    assert_eq!(step_cycles(&mut c), 12); // JR NZ taken
    assert_eq!(step_cycles(&mut c), 8); // JR Z not taken
    assert_eq!(step_cycles(&mut c), 24); // CALL NZ taken
    assert_eq!(step_cycles(&mut c), 20); // RET NZ taken
    c.pc = 0x104;
    c.registers.f.zero = true;
    assert_eq!(step_cycles(&mut c), 12); // CALL NZ not taken
    c.pc = 0x10a;
    assert_eq!(step_cycles(&mut c), 8); // RET NZ not taken
}

#[test]
fn cb_register_bit_memory_and_modify_memory_durations() {
    let mut c = cpu(&[0xcb, 0x00, 0xcb, 0x46, 0xcb, 0x86]);
    c.registers.h = 0xc0;
    c.registers.l = 0;
    for expected in [8, 12, 16] {
        assert_eq!(step_cycles(&mut c), expected);
    }
}

#[test]
fn ldh_div_write_occurs_in_final_machine_cycle() {
    let mut c = cpu(&[0xe0, 0x04]);
    c.bus.step(4090);
    c.bus.write_byte(0xff26, 0x80);
    c.bus.write_byte(0xff12, 0xf0);
    c.bus.write_byte(0xff11, 0x3f);
    c.bus.write_byte(0xff14, 0xc0);
    // DIV bit 12 becomes high during LDH; reset must clock the length counter.
    assert_eq!(step_cycles(&mut c), 12);
    assert_eq!(c.bus.read_byte(0xff26) & 1, 0);
    assert_eq!(c.bus.timer.div_counter, 4);
}

#[test]
fn halt_and_interrupt_entry_consume_time() {
    let mut c = cpu(&[0x76]);
    assert_eq!(step_cycles(&mut c), 4);
    assert_eq!(step_cycles(&mut c), 4);
    c.ime = true;
    c.bus.ie_register = 1;
    c.bus.if_register = 1;
    assert_eq!(step_cycles(&mut c), 20);
    assert_eq!(c.pc, 0x40);
}
