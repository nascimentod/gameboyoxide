# Handoff: DMG APU, Pokémon Red sprites, and battery saves

## Next session: clean stale tests/verifiers, then commit

The user has requested that the **next session audit and clean stale tests and verification tooling, then commit the reviewed work**. This session intentionally leaves the changes uncommitted.

1. Read this file first. Inspect `git status` and the complete diff before editing: the working tree contains pre-existing CPU, cartridge, register, test and documentation changes as well as the APU/sprite/save work. Do not discard or overwrite them blindly.
2. Audit `tests/rom_runner.rs`, legacy non-sound helpers/tests in `tests/test_cpu_instrs.rs`, `tests/verify_*.rs`, `decode_rom.py`, `__pycache__/`, `run_test.sh`, and old claims in `SESSION.md` and project docs. Keep useful regressions; remove or consolidate genuinely stale/duplicate scaffolding and debug artifacts. Do not delete a test merely because it exposes a real bug.
3. In particular, legacy density-based screen parsing can incorrectly call unknown output a pass, some ROM tests print failures instead of asserting, and old verifier tests/notes treat Z80-style ED instructions and AF-prime behavior as Game Boy features. The DMG CPU does not have that ED instruction set. Replace misleading tests/claims with hardware-grounded assertions or clearly mark deferred work; do not silently bless the implementation's behavior as a hardware oracle.
4. Preserve the enabled, genuinely asserting sound ROM regressions and focused APU, CPU timing, MBC1 sound-scan, sprite/DMA and battery-save tests. Separate device-dependent playback tests from headless defaults. Ensure the normal test entry point is bounded and fails on real errors rather than reporting success for unknown screens.
5. Review pending implementation changes and known limitations below. Run the focused verification commands, then the cleaned default test suite. Report any failures honestly. Update documentation so it distinguishes hardware-tested facts, unit-test coverage and user-confirmed behavior.
6. Review formatting, generated files, dependency/lockfile changes and `git diff --check`. Stage deliberately and commit coherent changes with descriptive messages (split logical commits if appropriate). Do **not** commit the user's ROMs, `.sav` files, `/tmp` logs/screenshots, build products or Python cache artifacts. Do not amend or reset unrelated commits.

## User-confirmed end-to-end results

- Pokémon Red game audio sounds correct to the user.
- Overworld people are visible after the sprite/DMA fixes.
- The user used the game's SAVE command; the emulator wrote `pokemon-red.sav`.
- After closing and relaunching the emulator, the user selected **CONTINUE and confirmed it works**. Battery-save persistence and reload are verified end-to-end, not only by unit tests.
- Latest launch used `/Users/davidnascimento/Downloads/pokemon-red.gb`; its battery save is `/Users/davidnascimento/Downloads/pokemon-red.sav`. Preserve that file.

## Current verified result

**All 12 subtests in the actual `test-roms/dmg_sound.gb` return 0. The aggregate ROM completes and renders literal `Passed`, with first-failure byte D880 = 0.** Both the subtest-return regression and aggregate completion/screen regression are enabled (not ignored) and assert success.

The APU now generates stereo audio, and the GUI feeds it to a CPAL host output stream. An explicit host smoke test opened a 48,000 Hz stereo device and observed 24,109 nonzero APU frames consumed by its callback. **The user confirmed hearing the sound during that test: audible host output is verified.** This confirms audibility, not subjective accuracy of game audio.

### Verification results

- Focused headless tests: **31 passed** (26 APU register/sequencer/signal tests and 5 CPU timing integration tests).
- Sound ROM tests: **3 passed**, repeated successfully in three consecutive release runs (glyph parser, all subtest return values, aggregate completion and actual passing screen).
- Individual ROM returns:

| Subtest | Description | Return |
| --- | --- | --- |
| 1 | Registers | 0 / pass |
| 2 | Length counter | 0 / pass |
| 3 | Trigger | 0 / pass |
| 4 | Sweep | 0 / pass |
| 5 | Sweep details | 0 / pass |
| 6 | Overflow on trigger | 0 / pass |
| 7 | Length/sweep period sync | 0 / pass |
| 8 | Length counter during power | 0 / pass |
| 9 | Wave read while on | 0 / pass |
| 10 | Wave trigger while on | 0 / pass |
| 11 | Registers after power | 0 / pass |
| 12 | Wave write while on | 0 / pass |

- Host playback smoke test: **passed when explicitly run**, otherwise ignored because it needs a real output device and emits a short tone.
- Nearby non-ROM regression targets listed below: **102 passed**. This is not a claim that the legacy all-ROM runner or the entire emulator is correct.

## Implementation and diagnosis

### Timing and original subtest 4 stall

The baseline partial APU passed its three original unit tests but timed out after 100 million CPU steps at PC C091. The loop at C08D–C091 is `LDH A,(NR52); AND 1; JR NZ,...`. It waits for CH1 sweep overflow to disable the channel. Implementing CH1 sweep removed that stall, but initially most sound subtests still failed.

The next blocker was `CPU::update_interrupt_flags`: almost every instruction advanced peripherals by only four T-cycles, including instructions that actually take 8–24. The CPU now uses opcode durations, conditional branch/call/return durations, and CB register/memory durations. HALT clocks four T-cycles; interrupt entry clocks twenty. The boot-only timing multiplier was removed. The final data access is placed at the start of the instruction's last machine cycle, which gives LDH/DIV/APU accesses their required timing. This made subtests 1–8 and 11 pass before wave access was implemented.

**The CPU is still not a complete bus-cycle-accurate model.** Fetches, multi-access instructions, stack accesses and read/modify/write accesses are not individually scheduled. The timing work is deliberately limited to the integration needed by sound; it is not a general CPU rewrite.

### APU

- `src/apu.rs`: register masks, DAC/status/trigger behavior, DMG power transitions and writable off-state length fields; length preservation; DIV-bit-12 falling-edge frame sequencer; length enable/retrigger extra clocks; CH1 sweep shadow, timer, overflow and negate behavior.
- `src/memory.rs`: waveforms advance between sequencer edges in chronological order, including large steps. Resetting DIV clocks the sequencer only when internal bit 12 was high. Power changes do not reset DIV. `elapsed_cycles` supplies host pacing.
- `src/timer.rs`: clarified T-cycle units; TIMA implementation was not otherwise rewritten.
- Wave channel: period timer, initial trigger delay, retained sample buffer, nibble progression, digital output shifts; two-T-cycle DMG wave RAM access window and address redirection; retrigger corruption of byte 0 or the first aligned four-byte block. Real ROM subtests 9, 10 and 12 verify the resulting CRCs.
- `src/apu_signal.rs`: independent pulse duty generators, phase-preserving retriggers, noise LFSR/width/clock divisors, envelopes on sequencer step 7, DAC conversion/polarity, NR51 stereo routing and NR50 gain, box-integrated sample conversion and a DMG-style high-pass filter. Headless sample storage is bounded to 250 ms.
- `src/audio.rs`: CPAL device selection, device-rate sampling, f32/i16/u16 conversion, stereo/mono output, bounded playback queue (100 ms), silence on underrun, pause clearing, callback diagnostic count. An unavailable device produces a warning; emulation continues silently.
- `src/main.rs`: advances 70,224 T-cycles per DMG frame at approximately 59.7275 Hz, rather than 69,905 instructions at 60 Hz. It submits the generated samples and keeps the host stream alive.

No ROM results are forced or special-cased in the emulator. The implementation is original code informed by hardware documentation and test ROM source, not copied from another emulator.

## Verification commands

Run from `dmg-01/`:

```sh
# Headless register, power/DIV, length, sweep, waveform, envelope, mixer,
# sample-rate/pitch/filter and CPU timing regressions: 31 tests.
cargo test --test test_apu_registers --test test_apu_sweep \
  --test test_apu_frame_sequencer --test test_apu_waveforms \
  --test test_apu_cpu_timing

# Actual ROM: prints every subtest return, asserts all twelve are zero,
# checks completion and an actual passing screen; no --ignored flag.
cargo test --release --test test_cpu_instrs dmg_sound -- --nocapture

# Optional, device-dependent: emits a quiet half-second ~440 Hz pulse.
# Checks that the real host callback consumes nonzero APU samples.
cargo test --release --test test_apu_host_audio -- --ignored --nocapture

# GUI playback/listening check (ESC exits, SPACE pauses).
cargo run --release -- test-roms/dmg_sound.gb

# Nearby non-ROM regressions: 102 tests across these targets.
cargo test --lib --test test_cpu --test test_memory --test test_gameboy \
  --test verify_interrupts --test test_mbc1_dmg_sound --test test_cartridge \
  --test test_mbc1 --test test_ppu --test test_registers --test test_joypad \
  --test verify_cpu_prop --test verify_memory --test verify_roundtrip

cargo check
```

CPAL uses platform audio libraries (CoreAudio on the verified host; Linux builds may require ALSA development packages). Ordinary headless tests do not open a device.

### Remaining limitations / next verification

1. Audible host output during the smoke test is confirmed by the user. A longer game/GUI listening check is still useful for evaluating sound quality, sustained playback, and synchronization.
2. CPU instruction totals and common APU I/O placement are corrected, but individual CPU bus accesses are not fully cycle-scheduled. General TIMA edge/reload accuracy and STOP behavior remain outside this work.
3. The sample converter uses T-cycle box integration, not a band-limited resampler; high-frequency aliasing is still possible. HPF/analog DAC transitions are approximate. VIN is not connected/emulated.
4. The documented portable DMG zombie-envelope increment is implemented. Other active envelope writes and obscure model/revision-specific analog/digital behaviors remain approximate. No CGB/double-speed APU support is claimed.
5. Passing this version of dmg_sound is strong evidence for its register/length/sweep/wave cases, not proof of every waveform/envelope behavior or every DMG hardware revision. Broader hardware suites and game listening tests would add coverage.
6. Audio queues intentionally discard old frames on overload; there is no adaptive host clock synchronization, save-state audio support, or host-device hotplug recovery.

## Follow-up: Pokémon Red sprites and battery saves

The user confirmed Pokémon Red's game audio sounded correct, then reported that overworld people were missing. Two direct causes were found and reproduced: FF46 only stored the DMA register without copying OAM, and OBJ rendering incorrectly followed LCDC bit 4's signed background tile addressing.

Implemented:

- `src/memory.rs`: timed OAM DMA, with one startup M-cycle followed by 160 bytes at one byte per M-cycle. Reads use mapped ROM/RAM/VRAM, not the raw backing array. CPU access is restricted to HRAM during the transfer; a new FF46 write can restart it. Video and DMA advance together during transfer.
- `src/ppu.rs`: OBJ tiles always use unsigned 8000 addressing; signed screen coordinates correctly clip offscreen sprites and avoid overflow. Select the first ten Y-matching OAM entries before stable X-priority sorting. Resolve OBJ-vs-OBJ priority before OBJ-vs-BG priority. Preserve 8x16 tile-pair/flip behavior and clear old pixels when BG is disabled.
- `tests/test_oam_dma.rs`: 6 tests, including the CPU's standard HRAM DMA wait routine and shadow-OAM-to-visible-sprite integration.
- `tests/test_ppu_sprites.rs`: 9 tests covering addressing, clipping, 8x16 sprites, flips, palettes, selection and priority.
- `src/battery_save.rs`: load raw `.sav` RAM before execution, preserve all banks, reject invalid-size saves without overwriting them, write only changed RAM via a synced same-directory temporary file and atomic rename, retry failed writes. Supported battery-backed MBC1/MBC3 cartridges only; RTC state is not persisted.
- GUI: flush changed battery RAM every second and on normal shutdown. Do not overwrite an existing save if loading failed. Preserve the event loop's exit request instead of replacing it with Poll afterward.
- `tests/test_battery_save.rs`: 6 tests covering loading, saving, all-bank round trips, invalid files, battery detection and failed-write retries.

**Save files are battery RAM, not full CPU save states.** The player must use the game's SAVE command before closing to preserve current progress. Changed RAM is persisted automatically, but this does not automatically save the current map/location in the game's save format. Force-killing the process can lose up to the last flush interval. There was no disk save support in the earlier build, and the user had already closed its window; its in-memory save could not be recovered.

For the currently used ROM, the save path is `/Users/davidnascimento/Downloads/pokemon-red.sav`. No existing Pokémon Red save was found in the ROM directory or repository. No unrelated saves were modified.

Verification:

```sh
# From dmg-01/
cargo test --test test_oam_dma --test test_ppu_sprites --test test_battery_save
cargo test --test test_ppu --test test_memory --test verify_memory
cargo test --test test_apu_registers --test test_apu_sweep --test test_apu_waveforms \
  --test test_apu_cpu_timing --test test_apu_frame_sequencer \
  --test test_cpu --test verify_interrupts
cargo test --release --test test_cpu_instrs dmg_sound -- --nocapture
cargo build --release
cargo run --release -- /Users/davidnascimento/Downloads/pokemon-red.gb
```

All listed regression commands passed after these changes. A headless real-ROM probe observed DMA from C300 and 35 visible OAM entries on the title screen, and captured a correctly rendered title screen. The updated GUI was relaunched, and the user **confirmed that overworld people are now visible**. The user also performed an in-game save; the runtime log confirms battery-save writes to `/Users/davidnascimento/Downloads/pokemon-red.sav`. The user then restarted the emulator and **confirmed CONTINUE loads the saved game successfully**. No APU behavior was changed in this follow-up.

Remaining PPU/DMA limits: PPU mode timing still discards transition overshoot for some step sizes, OAM scan is not a per-dot pipeline, and mid-render DMA/OAM contention is not modeled. Undocumented DMA source ranges above DF00 are not hardware-verified. These changes do not claim general PPU cycle accuracy.

## Existing MBC1 finding (preserved)

`test-roms/dmg_sound.gb` is a 64 KiB, four-bank MBC1 ROM. The aggregate runner at 231A scans four subtests per bank, probing 4000, 5000, 6000 and 7000 for opcode C3. There are twelve entries in banks 1–3. Entry 13 selects bank 4; disconnected ROM address lines map this to bank 0, whose first byte is 00, terminating the scan at 2344.

Previously MBC1 masked the bank first and then substituted one for zero. Selecting bank 4 therefore incorrectly selected bank 1 and repeated the scan indefinitely. `src/cartridge.rs` substitutes one only when the five-bit register itself is zero, before masking. Upper bank bits apply to the switchable window in both MBC1 modes. `tests/test_mbc1_dmg_sound.rs` verifies those cases and all thirteen real-ROM probes. This session did not change cartridge banking.

## Screen reporting and legacy tests

The earlier claim that dense stripes represented a passing bar graph was wrong. ROM routine 23BD stores the first failing subtest at D880; 25F2 selects `Passed` for zero and 26F4 prints `Failed`/`Failed #` otherwise. Routine 281A expands the font at ROM offset 28CD into VRAM; 28B5 writes ASCII tile indices to the 9800 map, with vertical scrolling.

The sound parser matches literal glyphs from that ROM font. Unrecognized screens stay Unknown. Its synthetic screen test checks pass/fail and rejects dense blocks and intermediate `ok` output. The completion regression now **requires an actual pass**, not merely consistency with a failure byte.

`test_roms_available` still prints rather than asserts failures; the parser for non-sound ROMs still has unreliable density heuristics. Do not use those legacy results as proof of compatibility.

## References

- https://gbdev.io/pandocs/Audio_Registers.html
- https://gbdev.io/pandocs/Audio_details.html
- https://gbdev.io/pandocs/CPU_Instruction_Set.html
- https://github.com/retrio/gb-test-roms/tree/master/dmg_sound/source (test specification/assembly, not emulator code)
