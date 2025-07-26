use dmg_01::gameboy::GameBoy;
use dmg_01::joypad::JoypadButton;

#[test]
fn test_joypad_basic_functionality() {
    let mut gameboy = GameBoy::new();
    
    // Initially all buttons should be unpressed
    let state = gameboy.get_joypad_state();
    assert!(!state.up);
    assert!(!state.down);
    assert!(!state.left);
    assert!(!state.right);
    assert!(!state.a);
    assert!(!state.b);
    assert!(!state.select);
    assert!(!state.start);
    
    // Press the A button
    gameboy.press_button(JoypadButton::A);
    let state = gameboy.get_joypad_state();
    assert!(state.a);
    assert!(!state.b); // Other buttons should still be unpressed
    
    // Press the Up button as well
    gameboy.press_button(JoypadButton::Up);
    let state = gameboy.get_joypad_state();
    assert!(state.a);
    assert!(state.up);
    assert!(!state.down);
    
    // Release the A button
    gameboy.release_button(JoypadButton::A);
    let state = gameboy.get_joypad_state();
    assert!(!state.a);
    assert!(state.up); // Up should still be pressed
}

#[test]
fn test_joypad_register_read_write() {
    let mut gameboy = GameBoy::new();
    
    // Test reading the joypad register when no buttons are pressed
    // Default state: no selection, no buttons pressed
    let register_value = gameboy.cpu.bus.read_byte(0xFF00);
    assert_eq!(register_value, 0xFF); // All bits should be 1 when nothing is selected/pressed
    
    // Select action buttons (write 0xDF to clear bit 5)
    gameboy.cpu.bus.write_byte(0xFF00, 0xDF);
    let register_value = gameboy.cpu.bus.read_byte(0xFF00);
    assert_eq!(register_value & 0x20, 0x00); // Bit 5 should be cleared (action buttons selected)
    assert_eq!(register_value & 0x0F, 0x0F); // Lower 4 bits should be 1 (no buttons pressed)
    
    // Press A button and read
    gameboy.press_button(JoypadButton::A);
    let register_value = gameboy.cpu.bus.read_byte(0xFF00);
    assert_eq!(register_value & 0x01, 0x00); // Bit 0 should be cleared (A button pressed)
    
    // Select direction buttons (write 0xEF to clear bit 4)
    gameboy.cpu.bus.write_byte(0xFF00, 0xEF);
    gameboy.press_button(JoypadButton::Up);
    let register_value = gameboy.cpu.bus.read_byte(0xFF00);
    assert_eq!(register_value & 0x10, 0x00); // Bit 4 should be cleared (direction buttons selected)
    assert_eq!(register_value & 0x04, 0x00); // Bit 2 should be cleared (Up button pressed)
}

#[test]
fn test_joypad_interrupt_generation() {
    let mut gameboy = GameBoy::new();
    
    // Initially no joypad interrupt should be pending
    let if_register = gameboy.cpu.read_if_register();
    assert_eq!(if_register & 0x10, 0x00); // Joypad interrupt bit should be clear
    
    // Press a button - this should generate an interrupt
    gameboy.press_button(JoypadButton::Start);
    
    // Step the CPU to process the interrupt request  
    gameboy.step();
    
    // Check if joypad interrupt bit is set
    let if_register = gameboy.cpu.read_if_register();
    assert_eq!(if_register & 0x10, 0x10); // Joypad interrupt bit should be set
}