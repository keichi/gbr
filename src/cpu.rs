use mmu;

#[derive(Debug)]
pub struct CPU {
    mmu: mmu::MMU,
    pc: u16,
    sp: u16,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
}

#[derive(Debug)]
enum Reg8 {
    B,
    C,
    D,
    E,
    H,
    L,
    MEM,
    A,
}

#[derive(Debug)]
enum Reg16 {
    BC,
    DE,
    HL,
    SP,
}

impl CPU {
    pub fn new(mmu: mmu::MMU) -> Self {
        CPU {
            mmu: mmu,
            pc: 0,
            sp: 0,
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
        }
    }

    fn af(&self) -> u16 {
        (self.a as u16) << 8
    }

    fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    /// Read 8-bit immediate from memory
    fn read_d8(&mut self) -> u8 {
        let imm = self.mmu.read(self.pc);
        self.pc = self.pc.wrapping_add(1);

        imm
    }

    /// 16-bit load
    fn ld_reg16_d16(&mut self, reg: Reg16) {
        let lo = self.read_d8();
        let hi = self.read_d8();

        debug!("LD {:?}, 0x{:02x}{:02x}", reg, hi, lo);

        match reg {
            Reg16::BC => {
                self.b = hi;
                self.c = lo
            }
            Reg16::DE => {
                self.d = hi;
                self.e = lo
            }
            Reg16::HL => {
                self.h = hi;
                self.l = lo
            }
            Reg16::SP => self.sp = (hi as u16) << 8 | lo as u16,
        }
    }

    /// 8-bit AND
    fn and_reg8(&mut self, reg: Reg8) {
        debug!("AND {:?}", reg);

        match reg {
            Reg8::B => self.a &= self.b,
            Reg8::C => self.a &= self.c,
            Reg8::D => self.a &= self.d,
            Reg8::E => self.a &= self.e,
            Reg8::H => self.a &= self.h,
            Reg8::L => self.a &= self.l,
            Reg8::MEM => self.a &= self.mmu.read(self.hl()),
            Reg8::A => self.a &= self.a,
        }
    }

    /// 8-bit OR
    fn or_reg8(&mut self, reg: Reg8) {
        debug!("OR {:?}", reg);

        match reg {
            Reg8::B => self.a |= self.b,
            Reg8::C => self.a |= self.c,
            Reg8::D => self.a |= self.d,
            Reg8::E => self.a |= self.e,
            Reg8::H => self.a |= self.h,
            Reg8::L => self.a |= self.l,
            Reg8::MEM => self.a |= self.mmu.read(self.hl()),
            Reg8::A => self.a |= self.a,
        }
    }

    /// 8-bit XOR
    fn xor_reg8(&mut self, reg: Reg8) {
        debug!("XOR {:?}", reg);

        match reg {
            Reg8::B => self.a ^= self.b,
            Reg8::C => self.a ^= self.c,
            Reg8::D => self.a ^= self.d,
            Reg8::E => self.a ^= self.e,
            Reg8::H => self.a ^= self.h,
            Reg8::L => self.a ^= self.l,
            Reg8::MEM => self.a ^= self.mmu.read(self.hl()),
            Reg8::A => self.a ^= self.a,
        }
    }

    fn ldi_hl_a(&mut self) {
        debug!("LD (HL+), A");

        let addr = self.hl();
        self.mmu.write(addr, self.a);
        self.a = self.a.wrapping_add(1);
    }

    fn ldd_hl_a(&mut self) {
        debug!("LD (HL-), A");

        let addr = self.hl();
        self.mmu.write(addr, self.a);
        self.a = self.a.wrapping_sub(1);
    }

    fn ldi_a_hl(&mut self) {
        debug!("LD A, (HL+)");

        let addr = self.hl();
        self.a = self.mmu.read(addr);
        self.a = self.a.wrapping_add(1);
    }

    fn ldd_a_hl(&mut self) {
        debug!("LD A, (HL-)");

        let addr = self.hl();
        self.a = self.mmu.read(addr);
        self.a = self.a.wrapping_sub(1);
    }

    /// Test bit
    fn bit(&mut self, pos: u8, reg: Reg8) {
        debug!("BIT {}, {:?}", pos, reg);
    }

    /// Prefixed instructions
    fn prefix(&mut self) {
        let opcode = self.read_d8();
        let pos = opcode >> 3 & 0x7;
        let reg = opcode & 0x7;

        match opcode {
            0x40...0x7f => match reg {
                0x00 => self.bit(pos, Reg8::B),
                0x01 => self.bit(pos, Reg8::C),
                0x02 => self.bit(pos, Reg8::D),
                0x03 => self.bit(pos, Reg8::E),
                0x04 => self.bit(pos, Reg8::H),
                0x05 => self.bit(pos, Reg8::L),
                0x06 => self.bit(pos, Reg8::MEM),
                0x07 => self.bit(pos, Reg8::A),
                _ => panic!("Should not happen"),
            },
            _ => println!("Unimplemented opcode 0xcb 0x{:x}", opcode),
        }
    }

    pub fn step(&mut self) {
        let opcode = self.read_d8();

        match opcode {
            // LD reg, imm
            0x01 => self.ld_reg16_d16(Reg16::BC),
            0x11 => self.ld_reg16_d16(Reg16::DE),
            0x21 => self.ld_reg16_d16(Reg16::HL),
            0x31 => self.ld_reg16_d16(Reg16::SP),

            // AND reg
            0xa0 => self.and_reg8(Reg8::B),
            0xa1 => self.and_reg8(Reg8::C),
            0xa2 => self.and_reg8(Reg8::D),
            0xa3 => self.and_reg8(Reg8::E),
            0xa4 => self.and_reg8(Reg8::H),
            0xa5 => self.and_reg8(Reg8::L),
            0xa6 => self.and_reg8(Reg8::MEM),
            0xa7 => self.and_reg8(Reg8::A),

            // OR reg
            0xb0 => self.or_reg8(Reg8::B),
            0xb1 => self.or_reg8(Reg8::C),
            0xb2 => self.or_reg8(Reg8::D),
            0xb3 => self.or_reg8(Reg8::E),
            0xb4 => self.or_reg8(Reg8::H),
            0xb5 => self.or_reg8(Reg8::L),
            0xb6 => self.or_reg8(Reg8::MEM),
            0xb7 => self.or_reg8(Reg8::A),

            // XOR reg
            0xa8 => self.xor_reg8(Reg8::B),
            0xa9 => self.xor_reg8(Reg8::C),
            0xaa => self.xor_reg8(Reg8::D),
            0xab => self.xor_reg8(Reg8::E),
            0xac => self.xor_reg8(Reg8::H),
            0xad => self.xor_reg8(Reg8::L),
            0xae => self.xor_reg8(Reg8::MEM),
            0xaf => self.xor_reg8(Reg8::A),

            // LDI, LDD
            0x22 => self.ldi_hl_a(),
            0x32 => self.ldd_hl_a(),
            0x2a => self.ldi_a_hl(),
            0x3a => self.ldd_a_hl(),

            0xcb => self.prefix(),
            _ => println!("Unimplemented opcode 0x{:x}", opcode),
        }
    }

    pub fn dump(&self) {
        println!("CPU State:");
        println!("PC: 0x{:04x}", self.pc);
        println!("SP: 0x{:04x}", self.sp);
        println!("AF: 0x{:04x}", self.af());
        println!("BC: 0x{:04x}", self.bc());
        println!("DE: 0x{:04x}", self.de());
        println!("HL: 0x{:04x}", self.hl());
    }
}
