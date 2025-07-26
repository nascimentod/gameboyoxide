# Game Boy Emulator (DMG-01)

A Game Boy emulator written in Rust, implementing the original Game Boy (DMG-01) hardware.

## Features

- ✅ **Complete CPU Implementation**: Sharp LR35902 CPU with all instructions
- ✅ **Memory Management**: Full 64KB address space with cartridge support
- ✅ **PPU (Picture Processing Unit)**: Complete graphics rendering system
  - Background and window rendering
  - Sprite rendering with 8x8 and 8x16 modes
  - Proper LCD timing and modes
  - 160x144 framebuffer output
- ✅ **Cartridge Support**: ROM loading with MBC support
- ✅ **Accurate Timing**: Proper CPU/PPU cycle timing
- ⚠️ **Headless Operation**: Currently no display output (framebuffer available)

## Missing Features

- ❌ **Display Output**: No visual output to screen yet
- ❌ **Audio (APU)**: No sound system
- ❌ **Input Handling**: No controller/keyboard input
- ❌ **Interrupt Controller**: Partial interrupt support
- ❌ **Timer System**: No timer registers
- ❌ **Serial Communication**: No link cable support

## Building and Running

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Game Boy ROM files (.gb)

### Building

```bash
git clone <repository-url>
cd dmg-01
cargo build --release
```

### Running

```bash
# Run with a ROM file
cargo run <rom_file.gb>

# Examples
cargo run tetris.gb
cargo run super_mario_land.gb
cargo run test_rom.gb
```

### Usage

```
Usage: dmg-01 <rom_file.gb>
Example: dmg-01 tetris.gb
```

## ROM File Requirements

- **Format**: Game Boy ROM files (.gb)
- **Size**: Minimum 32KB
- **Header**: Must contain valid Game Boy header at 0x0100-0x014F
- **Supported MBC Types**: 
  - ROM Only (Type 0x00)
  - MBC1 (Type 0x01-0x03) - Basic support

## Example Output

```
Game Boy Emulator - DMG-01
Loading ROM: tetris.gb
Game Title: TETRIS
Cartridge Type: 0x00
ROM Size: 32 KB
RAM Size: 0 KB

Starting emulation...
Note: This is a headless emulator - no display output yet
Press Ctrl+C to stop

Cycle 10000: PC=0x01D2, A=0x01, B=0x00, C=0x13, D=0x00, E=0xD8, H=0x01, L=0x4D
Cycle 20000: PC=0x01F2, A=0x01, B=0x00, C=0x13, D=0x00, E=0xD8, H=0x01, L=0x4D
...
```

## Testing

Run the full test suite:

```bash
cargo test
```

Run specific test categories:

```bash
cargo test test_cpu      # CPU tests
cargo test test_ppu      # PPU tests
cargo test test_memory   # Memory tests
cargo test test_gameboy  # Integration tests
```

## Project Structure

```
src/
├── main.rs          # Entry point and ROM loading
├── lib.rs           # Library exports
├── cpu.rs           # CPU implementation
├── ppu.rs           # Picture Processing Unit
├── memory.rs        # Memory bus and mapping
├── cartridge.rs     # Cartridge/ROM handling
├── registers.rs     # CPU registers
├── instructions.rs  # Instruction definitions
└── gameboy.rs       # Main emulator struct

tests/
├── test_cpu.rs      # CPU unit tests
├── test_ppu.rs      # PPU unit tests
├── test_memory.rs   # Memory unit tests
├── test_cartridge.rs # Cartridge unit tests
├── test_registers.rs # Register unit tests
└── test_gameboy.rs  # Integration tests
```

## Technical Details

### CPU (Sharp LR35902)
- 8-bit CPU similar to Z80
- 16-bit address space (64KB)
- 8-bit registers: A, B, C, D, E, H, L, F
- 16-bit registers: BC, DE, HL, AF, SP, PC
- Complete instruction set implementation

### PPU (Picture Processing Unit)
- 160x144 pixel LCD display
- 4 display modes: OAM Search, Drawing, H-Blank, V-Blank
- Background and window rendering
- Sprite system with 40 sprites max
- Tile-based graphics (8x8 pixel tiles)
- 4-color grayscale palette

### Memory Map
- 0x0000-0x3FFF: ROM Bank 0
- 0x4000-0x7FFF: ROM Bank 1-N (switchable)
- 0x8000-0x9FFF: Video RAM (VRAM)
- 0xA000-0xBFFF: External RAM
- 0xC000-0xFDFF: Work RAM
- 0xFE00-0xFE9F: Object Attribute Memory (OAM)
- 0xFF00-0xFF7F: I/O Registers
- 0xFF80-0xFFFE: High RAM
- 0xFFFF: Interrupt Enable Register

## Getting ROM Files

You can use homebrew Game Boy ROMs or create your own:

1. **Homebrew ROMs**: Available from sites like [itch.io](https://itch.io/games/tag-gameboy)
2. **Test ROMs**: Various test ROMs available for emulator testing
3. **Create Your Own**: Use tools like [RGBDS](https://github.com/gbdev/rgbds)

## Next Steps

To make this a complete emulator, the following features need to be added:

1. **Display Output**: SDL2 or similar for visual output
2. **Audio System**: APU implementation for sound
3. **Input Handling**: Keyboard/controller support
4. **Better MBC Support**: More cartridge types
5. **Interrupt Controller**: Complete interrupt system
6. **Timer System**: Proper timer implementation
7. **Save States**: Save/load game states
8. **Debugging Tools**: Memory viewer, disassembler

## Contributing

This is a learning project demonstrating Game Boy emulation concepts. The code is well-structured and documented for educational purposes.

## License

This project is for educational purposes. Game Boy is a trademark of Nintendo.