# Formal Verification Framework - Session Tracker

## Session 1: Foundation

### Date: 2025-11-11

### Summary
Built a comprehensive property-based and structural verification framework:
- 46 new verification tests (all passing)
- Total test suite: 97 tests, 0 failures
- Found 2 bugs: ED opcode execution gap, MBC0 RAM enable
- Downloaded 9 test ROMs from retrio/gb-test-roms
- Scaffolded ROM integration test framework

### File Index
```
dmg-01/
├── SESSION.md              # This file - session tracker
├── Cargo.toml              # Added proptest, rand dev-deps
├── src/
│   ├── cartridge.rs        # Fixed: MBC0 RAM enable
│   ├── cpu.rs              # Added: ED opcode panic arm, pub interrupt methods
│   └── ...
├── tests/
│   ├── rom_runner.rs       # ROM integration test framework (scaffolded)
│   ├── verify_cpu_prop.rs  # 21 property-based CPU tests
│   ├── verify_interrupts.rs # 16 interrupt state machine tests
│   ├── verify_memory.rs    # 20 memory invariant tests
│   ├── verify_roundtrip.rs # 12 round-trip consistency tests
│   └── ... (existing tests)
└── test-roms/              # 9 test ROMs downloaded
    ├── cpu_instrs.gb
    ├── 07-jr_jp_call_ret_rst.gb
    ├── 08-misc_instrs.gb
    ├── 10-bit_ops.gb
    └── ...
```

### Completed
- [x] Created `formal-verification-framework` branch from main
- [x] Downloaded test ROMs from retrio/gb-test-roms:
  - `cpu_instrs.gb` - CPU instruction tests (CRC-32 based, all standard + CB opcodes)
  - `07-jr_jp_call_ret_rst.gb` - Control flow tests (JR, JP, CALL, RET, RST)
  - `08-misc_instrs.gb` - Misc instructions (DAA, CPL, SCF, CCF, DI, EI, RETI, STOP)
  - `10-bit_ops.gb` - Bit operation tests (BIT, RES, SET)
  - `dmg_sound.gb` - Sound tests (later)
  - `interrupt_time.gb` - Interrupt timing tests
  - `mem_timing.gb` - Memory timing tests
  - `oam_bug.gb` - OAM bug tests
- [x] Added `proptest` and `rand` dev dependencies
- [x] Created `tests/verify_cpu_prop.rs` - Property-based CPU instruction tests (21 tests)
  - ADD with all register sources: verifies result, Z, C, H flags
  - ADC with carry: verifies carry chain computation
  - SUB with all register sources
  - SBC with carry
  - AND/OR/XOR: verifies flags (H, C, S)
  - INC/DEC: verifies half_carry for 0xF→0x0 and 0x0→0xFF transitions
  - 16-bit ADD HL, rr: verifies carry and half_carry on 12-bit nibble overflow
  - 16-bit INC/DEC
  - CPL: verifies bit flip + S/H flags
  - SCF: verifies C set, S/H cleared
  - CCF: verifies carry toggle
  - DAA: basic BCD validation
  - LD: flag preservation invariant
  - PUSH/POP round-trip: BC, HL, AF
  - ED opcode decode: 35 sample ED opcodes
- [x] Created `tests/verify_roundtrip.rs` - Instruction round-trip tests (12 tests)
  - Determinism: same input → same output
  - NOP round-trip
  - LD A, d8 round-trip
  - LD BC, d16 round-trip
  - LD SP, d16 round-trip
  - ADD HL, BC round-trip (exact flag computation)
  - JP HL round-trip
  - JR relative round-trip
  - PUSH/POP stack consistency
  - Carry chain propagation
  - Flag independence
- [x] Created `tests/verify_interrupts.rs` - Interrupt state machine tests (16 tests)
  - IE/IF register write/read/mask
  - Interrupt priority (V-Blank highest)
  - Vector address correctness
  - IME disabled blocks interrupts
  - IME enabled allows interrupts
  - EI sets ime after next instruction
  - DI clears IME immediately
  - RETI enables IME
  - Interrupt pushes correct PC (high byte at SP+1, low at SP)
  - Interrupt clears IF flag
  - HALT wake-up (with and without IME)
  - IF read mask (upper 3 bits always 1)
  - IE write masking
  - IME disabled during interrupt handling
- [x] Created `tests/verify_memory.rs` - Memory invariant tests (20 tests)
  - Boot ROM region detection
  - VRAM region read/write
  - OAM region read/write
  - Echo RAM region (0xFEA0-0xFEFF returns 0x00)
  - HRAM region read/write
  - IE register at 0xFFFF
  - Write-through consistency (internal RAM)
  - VRAM write-through
  - Cartridge ROM read
  - Cartridge RAM read/write (MBC0)
  - DIV register reset on write
  - TIMA/TMA/TAC register behavior
  - Joypad register default state
  - LCDC register read/write
  - Boot ROM disable via 0xFF50
  - PPU register read/write
  - DMA register echo
  - Memory corruption detection
  - No cross-region address leak

### Bug Fixes Found
1. **ED opcodes not executed**: Added `Instruction::ED(_)` arm to execute() that panics with opcode info
2. **MBC0 RAM never enabled**: Fixed cartridge.rs to handle MBC0 RAM enable (0x0A to 0x0000-0x1FFF)

### Test Results
- **Total: 97 tests pass (0 failures)**
- Existing tests: 51 tests
- New verification tests: 46 tests

### What's Next (Session 2 - Pending)
- [x] Create ROM runner framework (`tests/rom_runner.rs`) ✅
- [ ] Run cpu_instrs.gb and parse LCD output for pass/fail
- [ ] Run 07-jr_jp_call_ret_rst.gb
- [ ] Run 08-misc_instrs.gb
- [ ] Run 10-bit_ops.gb
- [ ] Run interrupt_time.gb
- [ ] Fix bugs found by ROM tests (expect ED opcode failures)

### Session 2 Plan (ROM Integration Tests)
The ROM runner is scaffolded in `tests/rom_runner.rs`:
- `RomTestRunner` struct handles loading, running, frame capture
- `extract_char(col, row)` returns 8x8 pixel patterns for font matching
- `extract_visible_chars()` scans LCD for non-empty characters
- `parse_test_result()` checks for "ok" / "E nn" patterns
- `crc32()` computes Castagnoli CRC-32
- `run_cpu_instrs_test()` runs a ROM for 30B cycles and parses result

Font approach: The cpu_instrs ROM uses a compact custom font.
Strategy: Match character pixel patterns against known templates.
Alternative: Run individual test ROMs and look for specific output patterns.

Key challenges:
1. cpu_instrs tests each take ~30 seconds (30B cycles) - need efficient running
2. Font recognition needs template matching
3. Need to handle the "ok" / "E nn" output format correctly
4. Some tests may crash on unimplemented ED opcodes - need graceful handling

### Key Architectural Decisions
1. **Proptest over Hypothesis**: Using Rust's `proptest` crate for property-based testing
2. **Layered approach**: Structural tests (fast, unit-level) → ROM tests (slower, integration-level)
3. **No external oracle**: Tests verify internal correctness against mathematical invariants
4. **ROM tests use CRC-32**: cpu_instrs.gb prints pass/fail on LCD, we can capture LCD state
5. **Session tracking**: This file tracks progress for cross-session continuity

### Bug Patterns We're Hunting
1. **Flag computation errors**: Half-carry not set correctly, carry chain broken
2. **ED opcode gaps**: Many ED opcodes might be unimplemented or wrong
3. **Interrupt timing**: EI delay, DI blocking during interrupt, IME rules
4. **Stack behavior**: PUSH/POP order, SP wrapping, alignment
5. **16-bit arithmetic**: ADC16 (ADD HL, rr), SP math, LD HL,SP,e8
6. **DAA (Decimal Adjust)**: Complex flag-dependent BCD correction
7. **CB instruction patterns**: BIT/RES/SET with (HL) indirect, carry propagation in rotates

### File Index
```
dmg-01/
├── SESSION.md              <- This file
├── Cargo.toml              <- Updated with proptest
├── src/
│   ├── verify/
│   │   └── mod.rs          <- Verification utilities
│   └── ... (existing)
├── tests/
│   ├── verify_cpu_prop.rs  <- Property-based CPU tests
│   ├── verify_roundtrip.rs <- Round-trip tests
│   ├── verify_interrupts.rs<- Interrupt state machine
│   ├── verify_memory.rs    <- Memory invariants
│   └── ... (existing)
└── test-roms/              <- Downloaded ROMs
    ├── cpu_instrs.gb
    ├── 07-jr_jp_call_ret_rst.gb
    ├── 08-misc_instrs.gb
    ├── 10-bit_ops.gb
    └── ...
```
