# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based Game Boy emulator project called "gameboyoxide". The main emulator code is in the `dmg-01/` directory (named after the original Game Boy model). The `rust_book/` directory contains learning exercises from the Rust Programming Language book.

## Common Commands

### Build and Development
```bash
# Build the project
cd dmg-01 && cargo build

# Build with optimizations
cd dmg-01 && cargo build --release

# Run the emulator
cd dmg-01 && cargo run

# Check code without building
cd dmg-01 && cargo check
```

### Testing
```bash
# Run all tests
cd dmg-01 && cargo test

# Run specific test
cd dmg-01 && cargo test test_name
```

### Code Quality
```bash
# Format code
cd dmg-01 && cargo fmt

# Lint code
cd dmg-01 && cargo clippy

# Lint with all targets
cd dmg-01 && cargo clippy --all-targets
```

## Architecture

The emulator is structured around these core components:

- **CPU**: Game Boy's Sharp LR35902 CPU emulation with registers and instruction execution
- **Registers**: 8-bit registers (A, B, C, D, E, H, L) with 16-bit register pairs (BC, DE, HL) 
- **FlagsRegister**: CPU flags (Zero, Subtract, Half Carry, Carry) with bit manipulation
- **MemoryBus**: Memory management system for the Game Boy's memory map
- **Instructions**: Instruction set architecture with enums for different operation types

### Key Files
- `dmg-01/src/main.rs`: Main emulator implementation with CPU, memory, and instruction handling

### Current State
The project is in early development with basic CPU register operations and the beginning of instruction execution. The architecture follows a traditional emulator pattern with separate CPU, memory, and instruction components.

## Working Directory
Always work in the `dmg-01/` directory when building or running the emulator, as this contains the actual emulator code. The root directory contains multiple Rust projects.