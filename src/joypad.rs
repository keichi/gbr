use io_device::IODevice;

pub struct Joypad {
    joyp: u8,
    key_state: u8,
    pub irq: bool,
}

#[derive(Hash, Eq, PartialEq)]
pub enum Key {
    Down,
    Up,
    Left,
    Right,
    Start,
    Select,
    B,
    A,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            joyp: 0xff,
            key_state: 0xff,
            irq: false,
        }
    }

    pub fn keydown(&mut self, key: Key) {
        match key {
            Key::Down => self.key_state &= !0x80,
            Key::Up => self.key_state &= !0x40,
            Key::Left => self.key_state &= !0x20,
            Key::Right => self.key_state &= !0x10,
            Key::Start => self.key_state &= !0x08,
            Key::Select => self.key_state &= !0x04,
            Key::B => self.key_state &= !0x02,
            Key::A => self.key_state &= !0x01,
        }

        self.irq = true;
    }

    pub fn keyup(&mut self, key: Key) {
        match key {
            Key::Down => self.key_state |= 0x80,
            Key::Up => self.key_state |= 0x40,
            Key::Left => self.key_state |= 0x20,
            Key::Right => self.key_state |= 0x10,
            Key::Start => self.key_state |= 0x08,
            Key::Select => self.key_state |= 0x04,
            Key::B => self.key_state |= 0x02,
            Key::A => self.key_state |= 0x01,
        }
    }
}

impl IODevice for Joypad {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xff00 => self.joyp = (self.joyp & 0xcf) | (val & 0x30),
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0xff00 => {
                // Direction keys selected
                if self.joyp & 0x10 == 0 {
                    (self.joyp & 0xf0) | (self.key_state >> 4) & 0x0f
                // Button keys selected
                } else if self.joyp & 0x20 == 0 {
                    (self.joyp & 0xf0) | self.key_state & 0x0f
                } else {
                    self.joyp
                }
            }
            _ => panic!("Invalid address: 0x{:04x}", addr),
        }
    }

    fn update(&mut self, tick: u8) {}
}
