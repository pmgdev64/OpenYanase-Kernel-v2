// src/keyboard.rs

static mut LINE_BUF: [u8; 256] = [0; 256];
static mut LINE_LEN: usize = 0;

// Cờ lưu trạng thái phím Shift đang được giữ hay không
static mut SHIFT_PRESSED: bool = false;

pub fn scancode_to_ascii(scancode: u8) -> Option<char> {
    unsafe {
        match scancode {
            // Hàng phím số (1-0) và các dấu tương ứng
            0x02 => Some(if SHIFT_PRESSED { '!' } else { '1' }),
            0x03 => Some(if SHIFT_PRESSED { '@' } else { '2' }),
            0x04 => Some(if SHIFT_PRESSED { '#' } else { '3' }),
            0x05 => Some(if SHIFT_PRESSED { '$' } else { '4' }),
            0x06 => Some(if SHIFT_PRESSED { '%' } else { '5' }),
            0x07 => Some(if SHIFT_PRESSED { '^' } else { '6' }),
            0x08 => Some(if SHIFT_PRESSED { '&' } else { '7' }),
            0x09 => Some(if SHIFT_PRESSED { '*' } else { '8' }),
            0x0A => Some(if SHIFT_PRESSED { '(' } else { '9' }),
            0x0B => Some(if SHIFT_PRESSED { ')' } else { '0' }),
            0x0C => Some(if SHIFT_PRESSED { '_' } else { '-' }),
            0x0D => Some(if SHIFT_PRESSED { '+' } else { '=' }),
            0x0E => Some('\x08'), // Backspace

            // Hàng chữ QWERTY thứ nhất
            0x10 => Some(if SHIFT_PRESSED { 'Q' } else { 'q' }),
            0x11 => Some(if SHIFT_PRESSED { 'W' } else { 'w' }),
            0x12 => Some(if SHIFT_PRESSED { 'E' } else { 'e' }),
            0x13 => Some(if SHIFT_PRESSED { 'R' } else { 'r' }),
            0x14 => Some(if SHIFT_PRESSED { 'T' } else { 't' }),
            0x15 => Some(if SHIFT_PRESSED { 'Y' } else { 'y' }),
            0x16 => Some(if SHIFT_PRESSED { 'U' } else { 'u' }),
            0x17 => Some(if SHIFT_PRESSED { 'I' } else { 'i' }),
            0x18 => Some(if SHIFT_PRESSED { 'O' } else { 'o' }),
            0x19 => Some(if SHIFT_PRESSED { 'P' } else { 'p' }),
            0x1A => Some(if SHIFT_PRESSED { '{' } else { '[' }),
            0x1B => Some(if SHIFT_PRESSED { '}' } else { ']' }),
            0x1C => Some('\n'), // Enter

            // Hàng chữ ASDF thứ hai
            0x1E => Some(if SHIFT_PRESSED { 'A' } else { 'a' }),
            0x1F => Some(if SHIFT_PRESSED { 'S' } else { 's' }),
            0x20 => Some(if SHIFT_PRESSED { 'D' } else { 'd' }),
            0x21 => Some(if SHIFT_PRESSED { 'F' } else { 'f' }),
            0x22 => Some(if SHIFT_PRESSED { 'G' } else { 'g' }),
            0x23 => Some(if SHIFT_PRESSED { 'H' } else { 'h' }),
            0x24 => Some(if SHIFT_PRESSED { 'J' } else { 'j' }),
            0x25 => Some(if SHIFT_PRESSED { 'K' } else { 'k' }),
            0x26 => Some(if SHIFT_PRESSED { 'L' } else { 'l' }),
            0x27 => Some(if SHIFT_PRESSED { ':' } else { ';' }),
            0x28 => Some(if SHIFT_PRESSED { '"' } else { '\'' }),
            0x29 => Some(if SHIFT_PRESSED { '~' } else { '`' }),

            // Hàng chữ ZXCV thứ ba
            0x2B => Some(if SHIFT_PRESSED { '|' } else { '\\' }),
            0x2C => Some(if SHIFT_PRESSED { 'Z' } else { 'z' }),
            0x2D => Some(if SHIFT_PRESSED { 'X' } else { 'x' }),
            0x2E => Some(if SHIFT_PRESSED { 'C' } else { 'c' }),
            0x2F => Some(if SHIFT_PRESSED { 'V' } else { 'v' }),
            0x30 => Some(if SHIFT_PRESSED { 'B' } else { 'b' }),
            0x31 => Some(if SHIFT_PRESSED { 'N' } else { 'n' }),
            0x32 => Some(if SHIFT_PRESSED { 'M' } else { 'm' }),
            0x33 => Some(if SHIFT_PRESSED { '<' } else { ',' }),
            0x34 => Some(if SHIFT_PRESSED { '>' } else { '.' }),
            0x35 => Some(if SHIFT_PRESSED { '?' } else { '/' }),

            0x39 => Some(' '), // Space
            _ => None,
        }
    }
}

pub fn poll_scancode() -> Option<u8> {
    unsafe {
        // Kiểm tra xem có dữ liệu trong bộ đệm bàn phím không
        if crate::cpu::inb(0x64) & 1 != 0 {
            let scancode = crate::cpu::inb(0x60);
            // Chỉ trả về phím nhấn (không phải release)
            if scancode & 0x80 == 0 {
                return Some(scancode);
            }
        }
        None
    }
}

pub fn handle_scancode(scancode: u8) {
    unsafe {
        // Xử lý sự kiện nhấn phím Shift (Left Shift: 0x2A, Right Shift: 0x36)
        if scancode == 0x2A || scancode == 0x36 {
            SHIFT_PRESSED = true;
            return;
        }
        // Xử lý sự kiện nhả phím Shift (Break code: Make code + 0x80)
        if scancode == 0x2A + 0x80 || scancode == 0x36 + 0x80 {
            SHIFT_PRESSED = false;
            return;
        }

        // Bỏ qua các sự kiện thả phím khác (Break code - Bit 7 = 1)
        if scancode & 0x80 != 0 {
            return;
        }

        if let Some(ch) = scancode_to_ascii(scancode) {
            // Ẩn con trỏ trước khi vẽ ký tự mới
            crate::console::CONSOLE.draw_cursor(false);

            match ch {
                '\n' => {
                    crate::println!(); // Xuống dòng mới[cite: 11]
                    
                    // Gửi câu lệnh trong buffer sang Shell xử lý[cite: 11]
                    if let Ok(input_str) = core::str::from_utf8(&LINE_BUF[..LINE_LEN]) {
                        crate::shell::execute(input_str);
                    }
                    
                    // Reset bộ đệm dòng và in lại dấu nhắc lệnh[cite: 11]
                    LINE_LEN = 0;
                    crate::print!("> ");
                }
                '\x08' => { // Xử lý phím Backspace[cite: 11]
                    if LINE_LEN > 0 {
                        LINE_LEN -= 1;
                        crate::print!("\x08"); // Xóa 1 ký tự trên VTTY Console[cite: 11]
                    }
                }
                _ => { // Ký tự văn bản thông thường[cite: 11]
                    if LINE_LEN < LINE_BUF.len() {
                        LINE_BUF[LINE_LEN] = ch as u8;
                        LINE_LEN += 1;
                        crate::print!("{}", ch);
                    }
                }
            }
        }
    }
}