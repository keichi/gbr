pub trait IODevice {
    fn write(&mut self, addr: u16, val: u8);
    fn read(&self, addr: u16) -> u8;
    fn update(&mut self, ticks: u8) -> bool;
}
