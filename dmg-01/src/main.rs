mod apu;
mod audio;
mod battery_save;
mod cartridge;
mod cpu;
mod gameboy;
mod instructions;
mod joypad;
mod memory;
mod ppu;
mod registers;
mod timer;

use gameboy::GameBoy;
use pixels::{Pixels, SurfaceTexture};
use std::env;
use std::fs;
use std::process;
use std::path::Path;
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <rom_file.gb>", args[0]);
        eprintln!("Example: {} tetris.gb", args[0]);
        process::exit(1);
    }

    let rom_path = &args[1];
    
    // Load ROM file
    let rom_data = match fs::read(rom_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading ROM file '{}': {}", rom_path, e);
            process::exit(1);
        }
    };

    // Basic ROM validation
    if rom_data.len() < 0x8000 {
        eprintln!("Error: ROM file too small (minimum 32KB required)");
        process::exit(1);
    }

    // Check for Game Boy header
    if rom_data.len() > 0x0147 {
        let title_bytes = &rom_data[0x0134..0x0144];
        let title = String::from_utf8_lossy(title_bytes);
        let title = title.trim_end_matches('\0').trim();
        
        println!("Game Boy Emulator - DMG-01");
        println!("Loading ROM: {}", rom_path);
        println!("Game Title: {}", title);
        
        let cartridge_type = rom_data[0x0147];
        println!("Cartridge Type: 0x{:02X}", cartridge_type);
        
        let rom_size = rom_data[0x0148];
        println!("ROM Size: {} KB", 32 << rom_size);
        
        let ram_size = rom_data[0x0149];
        let ram_size_kb = match ram_size {
            0 => 0,
            1 => 2,
            2 => 8,
            3 => 32,
            4 => 128,
            5 => 64,
            _ => 0,
        };
        println!("RAM Size: {} KB", ram_size_kb);
    } else {
        println!("Game Boy Emulator - DMG-01");
        println!("Loading ROM: {}", rom_path);
        println!("Warning: ROM header not found or incomplete");
    }

    let mut gameboy = GameBoy::new();
    gameboy.load_cartridge(rom_data);
    let mut battery_save = match battery_save::BatterySave::open(
        Path::new(rom_path), gameboy.cpu.bus.cartridge.as_mut().unwrap(),
    ) {
        Ok(save) => {
            if let Some(save) = &save { println!("Battery save: {}", save.path().display()); }
            save
        }
        Err(err) => {
            eprintln!("Battery save could not be loaded: {err}. Saving disabled to protect the existing file.");
            None
        }
    };
    let audio = match audio::HostAudio::new() {
        Ok(audio) => {
            gameboy.cpu.bus.apu.set_sample_rate(audio.sample_rate());
            Some(audio)
        }
        Err(err) => {
            eprintln!("Audio unavailable; continuing silently: {err}");
            None
        }
    };

    println!("\nStarting emulation with display...");
    println!("Controls: ESC exit, SPACE pause, arrows D-pad, Z=A, X=B, Enter=Start, Right Shift=Select");

    // Set up display
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Game Boy Emulator - DMG-01")
        .with_inner_size(LogicalSize::new(160 * 4, 144 * 4)) // 4x scale
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(160, 144, surface_texture).unwrap()
    };

    let mut last_frame_time = Instant::now();
    // A DMG frame is 70,224 T-cycles (~59.7275 Hz), not 69,905 instructions.
    let frame_duration = Duration::from_secs_f64(70_224.0 / f64::from(apu::CLOCK_RATE));
    let mut next_cycle_target = gameboy.cpu.bus.elapsed_cycles;
    let mut paused = false;
    let mut last_save_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput { 
                    input: winit::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                    .. 
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput { 
                    input: winit::event::KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Space),
                        state: ElementState::Pressed,
                        ..
                    },
                    .. 
                } => {
                    paused = !paused;
                    if let Some(audio) = &audio { audio.clear(); }
                    gameboy.cpu.bus.apu.take_samples();
                    println!("Emulation {}", if paused { "paused" } else { "resumed" });
                }
                WindowEvent::KeyboardInput {
                    input: winit::event::KeyboardInput {
                        virtual_keycode: Some(keycode),
                        state,
                        ..
                    },
                    ..
                } => {
                    if let Some(button) = key_to_joypad_button(keycode) {
                        match state {
                            ElementState::Pressed => gameboy.press_button(button),
                            ElementState::Released => gameboy.release_button(button),
                        }
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                let now = Instant::now();
                if now.duration_since(last_save_time) >= Duration::from_secs(1) {
                    flush_battery_save(&mut battery_save, &gameboy);
                    last_save_time = now;
                }
                if now.duration_since(last_frame_time) >= frame_duration {
                    if !paused {
                        next_cycle_target += 70_224;
                        while gameboy.cpu.bus.elapsed_cycles < next_cycle_target {
                            gameboy.step();
                        }
                        let samples = gameboy.cpu.bus.apu.take_samples();
                        if let Some(audio) = &audio { audio.submit(samples); }
                    }
                    
                    window.request_redraw();
                    // Keep fractional frame duration instead of accumulating
                    // event-loop scheduling latency into the audio pitch.
                    last_frame_time += frame_duration;
                    if now.duration_since(last_frame_time) > frame_duration * 4 {
                        last_frame_time = now;
                    }
                }
            }
            Event::RedrawRequested(_) => {
                update_display(&gameboy, pixels.get_frame_mut());
                
                if let Err(err) = pixels.render() {
                    eprintln!("pixels.render() failed: {}", err);
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::LoopDestroyed => flush_battery_save(&mut battery_save, &gameboy),
            _ => {}
        }
    });
}

fn flush_battery_save(save: &mut Option<battery_save::BatterySave>, gameboy: &GameBoy) {
    if let (Some(save), Some(cartridge)) = (save.as_mut(), gameboy.cpu.bus.cartridge.as_ref()) {
        match save.flush(cartridge) {
            Ok(true) => println!("Battery save written: {}", save.path().display()),
            Ok(false) => {},
            Err(err) => eprintln!("Could not write battery save {}: {err}", save.path().display()),
        }
    }
}

fn key_to_joypad_button(keycode: VirtualKeyCode) -> Option<crate::joypad::JoypadButton> {
    match keycode {
        VirtualKeyCode::Up => Some(crate::joypad::JoypadButton::Up),
        VirtualKeyCode::Down => Some(crate::joypad::JoypadButton::Down),
        VirtualKeyCode::Left => Some(crate::joypad::JoypadButton::Left),
        VirtualKeyCode::Right => Some(crate::joypad::JoypadButton::Right),
        VirtualKeyCode::Z => Some(crate::joypad::JoypadButton::A),
        VirtualKeyCode::X => Some(crate::joypad::JoypadButton::B),
        VirtualKeyCode::Return => Some(crate::joypad::JoypadButton::Start),
        VirtualKeyCode::RShift => Some(crate::joypad::JoypadButton::Select),
        _ => None,
    }
}

fn update_display(gameboy: &GameBoy, frame: &mut [u8]) {
    let framebuffer = gameboy.get_framebuffer();

    for (i, pixel) in framebuffer.iter().enumerate() {
        let rgb = pixel.to_rgb();
        let rgba = [rgb.0, rgb.1, rgb.2, 255];
        
        frame[i * 4..i * 4 + 4].copy_from_slice(&rgba);
    }
}
