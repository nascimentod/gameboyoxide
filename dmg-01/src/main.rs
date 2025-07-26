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
use crate::ppu::Color;
use std::env;
use std::fs;
use std::process;
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
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

    println!("\nStarting emulation with display...");
    println!("Controls: ESC to exit, SPACE to pause/resume");

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
    let target_fps = 60.0;
    let frame_duration = Duration::from_secs_f64(1.0 / target_fps);
    let mut paused = false;

    event_loop.run(move |event, _, control_flow| {
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
                        ..
                    },
                    .. 
                } => {
                    paused = !paused;
                    println!("Emulation {}", if paused { "paused" } else { "resumed" });
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                let now = Instant::now();
                if now.duration_since(last_frame_time) >= frame_duration {
                    if !paused {
                        // Run emulation for approximately one frame worth of cycles
                        // Game Boy runs at ~4.194 MHz, 60 FPS = ~69905 cycles per frame
                        for _ in 0..69905 {
                            gameboy.step();
                            
                            // Continue emulation even when CPU is halted - PPU and timers still need to run
                            // This allows interrupts (like V-Blank) to wake the halted CPU
                        }
                        
                        // Debug: Print boot ROM status
                        if gameboy.cpu.bus.boot_rom_enabled {
                            static mut COUNTER: u32 = 0;
                            unsafe {
                                COUNTER += 1;
                                if COUNTER % 10000 == 0 {
                                    println!("Boot ROM still enabled - PC: 0x{:04X}", gameboy.cpu.pc);
                                }
                            }
                        }
                    }
                    
                    window.request_redraw();
                    last_frame_time = now;
                }
            }
            Event::RedrawRequested(_) => {
                update_display(&gameboy, pixels.get_frame_mut());
                
                if let Err(err) = pixels.render() {
                    eprintln!("pixels.render() failed: {}", err);
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
        
        *control_flow = ControlFlow::Poll;
    });
}

fn update_display(gameboy: &GameBoy, frame: &mut [u8]) {
    let framebuffer = gameboy.get_framebuffer();
    
    // Debug: Check if framebuffer has any non-white pixels
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
        if FRAME_COUNT % 60 == 0 { // Every ~1 second at 60fps
            let non_white_count = framebuffer.iter()
                .filter(|&&pixel| pixel != Color::White)
                .count();
            if non_white_count > 0 {
                println!("Framebuffer has {} non-white pixels!", non_white_count);
                
                // Sample some pixel values
                for (i, &pixel) in framebuffer.iter().enumerate().take(10) {
                    if pixel != Color::White {
                        println!("Pixel {}: {:?}", i, pixel);
                    }
                }
            } else {
                println!("Framebuffer is all white (no graphics rendered)");
            }
        }
    }
    
    for (i, pixel) in framebuffer.iter().enumerate() {
        let rgb = pixel.to_rgb();
        let rgba = [rgb.0, rgb.1, rgb.2, 255];
        
        frame[i * 4..i * 4 + 4].copy_from_slice(&rgba);
    }
}