mod cartridge;
mod cpu;
mod gameboy;
mod instructions;
mod memory;
mod registers;

use gameboy::GameBoy;

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