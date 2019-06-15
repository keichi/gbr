use io_device::IODevice;

pub struct Timer {
    /// Timer counter
    tima: u8,
    /// Timer modulo
    tma: u8,
    /// Timer control
    tac: u8,
    /// Internal 16-bit counter
    counter: u16,
    /// Interrupt request
    pub irq: bool,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            tima: 0,
            tma: 0,
            tac: 0,
            counter: 0,
            irq: false,
        }
    }
}

impl IODevice for Timer {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // DIV
            0xff04 => self.counter = 0,
            // TIMA
            0xff05 => self.tima = val,
            // TMA
            0xff06 => self.tma = val,
            // TAC
            0xff07 => self.tac = val & 0x7,
            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            // DIV
            0xff04 => (self.counter >> 8) as u8,
            // TIMA
            0xff05 => self.tima,
            // TMA
            0xff06 => self.tma,
            // TAC
            0xff07 => self.tac,
            _ => unreachable!("Unexpected address: 0x{:04x}", addr),
        }
    }

    fn update(&mut self, tick: u8) {
        let counter_prev = self.counter;

        self.counter = self.counter.wrapping_add(tick as u16);

        if self.tac & 4 > 0 {
            let divider = match self.tac & 3 {
                0 => 10,
                1 => 4,
                2 => 6,
                3 | _ => 8,
            };

            let x = self.counter >> divider;
            let y = counter_prev >> divider;
            let mask = (1 << (16 - divider)) - 1;
            let diff = x.wrapping_sub(y) & mask;

            if diff > 0 {
                let (res, overflow) = self.tima.overflowing_add(diff as u8);

                if overflow {
                    self.tima = self.tma + (diff as u8 - 1);
                    self.irq = true;
                } else {
                    self.tima = res;
                }
            }
        }
    }
}
