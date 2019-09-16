/// An IO device connected to the bus.
pub trait IODevice {
    /// Writes a byte to an address.
    fn write(&mut self, addr: u16, val: u8);

    /// Reads a byte from an address.
    fn read(&self, addr: u16) -> u8;

    /// Progresses the clock for a given number of ticks.
    fn update(&mut self, tick: u8);
}
