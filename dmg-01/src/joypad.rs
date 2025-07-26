// Game Boy Joypad Input System
// Handles the joypad register (0xFF00) and button state management

#[derive(Debug, Clone, Copy)]
pub struct JoypadState {
    // Direction pad
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    
    // Action buttons
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
}

impl JoypadState {
    pub fn new() -> Self {
        JoypadState {
            up: false,
            down: false,
            left: false,
            right: false,
            a: false,
            b: false,
            select: false,
            start: false,
        }
    }
}

pub struct Joypad {
    pub state: JoypadState,
    pub select_buttons: bool,  // Bit 5: 0=Select action buttons (A,B,Select,Start)
    pub select_dpad: bool,     // Bit 4: 0=Select direction buttons (Right,Left,Up,Down)
    interrupt_requested: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            state: JoypadState::new(),
            select_buttons: true,  // Default: not selecting action buttons
            select_dpad: true,     // Default: not selecting direction buttons
            interrupt_requested: false,
        }
    }

    // Update button state and check for interrupt
    pub fn update_button(&mut self, button: JoypadButton, pressed: bool) {
        let was_pressed = self.get_button_state(button);
        
        // Update the button state
        match button {
            JoypadButton::Up => self.state.up = pressed,
            JoypadButton::Down => self.state.down = pressed,
            JoypadButton::Left => self.state.left = pressed,
            JoypadButton::Right => self.state.right = pressed,
            JoypadButton::A => self.state.a = pressed,
            JoypadButton::B => self.state.b = pressed,
            JoypadButton::Select => self.state.select = pressed,
            JoypadButton::Start => self.state.start = pressed,
        }
        
        // Generate interrupt on button press (not release)
        if !was_pressed && pressed {
            self.interrupt_requested = true;
        }
    }

    fn get_button_state(&self, button: JoypadButton) -> bool {
        match button {
            JoypadButton::Up => self.state.up,
            JoypadButton::Down => self.state.down,
            JoypadButton::Left => self.state.left,
            JoypadButton::Right => self.state.right,
            JoypadButton::A => self.state.a,
            JoypadButton::B => self.state.b,
            JoypadButton::Select => self.state.select,
            JoypadButton::Start => self.state.start,
        }
    }

    // Read the joypad register (0xFF00)
    pub fn read_register(&self) -> u8 {
        let mut result = 0xFF; // Start with all bits set (default state)
        
        // Set selection bits (inverted: 0 = selected, 1 = not selected)
        if self.select_buttons {
            result |= 0x20; // Set bit 5 (action buttons not selected)
        } else {
            result &= !0x20; // Clear bit 5 (action buttons selected)
        }
        if self.select_dpad {
            result |= 0x10; // Set bit 4 (direction buttons not selected)
        } else {
            result &= !0x10; // Clear bit 4 (direction buttons selected)
        }
        
        // Only set button states if the corresponding group is selected
        if !self.select_buttons {
            // Action buttons selected - clear bits for pressed buttons
            if self.state.a { result &= !0x01; }      // Bit 0: A button
            if self.state.b { result &= !0x02; }      // Bit 1: B button
            if self.state.select { result &= !0x04; } // Bit 2: Select button  
            if self.state.start { result &= !0x08; }  // Bit 3: Start button
        } else if !self.select_dpad {
            // Direction buttons selected - clear bits for pressed buttons
            if self.state.right { result &= !0x01; }  // Bit 0: Right
            if self.state.left { result &= !0x02; }   // Bit 1: Left
            if self.state.up { result &= !0x04; }     // Bit 2: Up
            if self.state.down { result &= !0x08; }   // Bit 3: Down
        }
        // If neither group is selected, lower 4 bits remain 1 (no buttons reported)
        
        // Upper 2 bits (6-7) are always 1
        result | 0xC0
    }

    // Write to the joypad register (0xFF00)
    pub fn write_register(&mut self, value: u8) {
        // Only bits 4 and 5 are writable (selection bits)
        self.select_buttons = (value & 0x20) != 0;
        self.select_dpad = (value & 0x10) != 0;
    }

    // Check and clear joypad interrupt
    pub fn check_and_clear_interrupt(&mut self) -> bool {
        if self.interrupt_requested {
            self.interrupt_requested = false;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum JoypadButton {
    // Direction pad
    Up,
    Down,
    Left,
    Right,
    
    // Action buttons
    A,
    B,
    Select,
    Start,
}