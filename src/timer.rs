use io_device::IODevice;

#[derive(Debug)]
pub struct Timer {
    /// Divider
    div: u8,
    /// Timer counter
    tima: u8,
    /// Timer modulo
    tma: u8,
    /// Timer control
    tac: u8,
    /// Internal 16-bit counter
    counter: u16,
    /// Previous counter value when TIMA was incremented
    counter_prev: u16,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            counter: 0,
            counter_prev: 0,
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
            _ => panic!("Wrong"),
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
            _ => panic!("Wrong"),
        }
    }

    fn update(&mut self, tick: u8) -> bool {
        let mut irq = false;

        self.counter = self.counter.wrapping_add(tick as u16);

        if self.tac & 4 > 0 {
            let divider = match self.tac & 3 {
                0 => 1024,
                1 => 16,
                2 => 64,
                3 => 256,
                _ => panic!("Wrong"),
            };

            let diff = (self.counter / divider).wrapping_sub(self.counter_prev / divider);
            if diff > 0 {
                self.counter_prev = self.counter;
                let (res, overflow) = self.tima.overflowing_add(diff as u8);

                if overflow {
                    self.tima = self.tma + (diff as u8 - 1);
                    irq = true;
                } else {
                    self.tima = res;
                }
            }
        }

        irq
    }
}
