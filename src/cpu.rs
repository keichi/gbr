use mmu;

#[derive(Debug)]
pub struct CPU {
    mmu: mmu::MMU,
    pc: u16,
    sp: u16,
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
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
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
        }
    }

    /// Read AF register
    fn af(&self) -> u16 {
        (self.a as u16) << 8
    }

    /// Read BC register
    fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    /// Read DE register
    fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    /// Read HL register
    fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    /// Write HL register
    fn set_hl(&mut self, val: u16) {
        self.h = (val >> 8 & 0xff) as u8;
        self.l = (val & 0xff) as u8;
    }

    fn reg_to_string(idx: u8) -> String {
        match idx {
            0 => String::from("B"),
            1 => String::from("C"),
            2 => String::from("D"),
            3 => String::from("E"),
            4 => String::from("H"),
            5 => String::from("L"),
            6 => String::from("(HL)"),
            7 => String::from("A"),
            _ => panic!("Invalid operand index: {}", idx),
        }
    }

    /// Write 8-bit operand
    fn write_r8(&mut self, idx: u8, val: u8) {
        match idx {
            0 => self.b = val,
            1 => self.c = val,
            2 => self.d = val,
            3 => self.e = val,
            4 => self.h = val,
            5 => self.l = val,
            6 => {
                let hl = self.hl();
                self.mmu.write(hl, val)
            }
            7 => self.a = val,
            _ => panic!("Invalid operand index: {}", idx),
        }
    }

    /// Read 8-bit operand
    fn read_r8(&mut self, idx: u8) -> u8 {
        match idx {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.mmu.read(self.hl()),
            7 => self.a,
            _ => panic!("Invalid operand index: {}", idx),
        }
    }

    /// Read 8-bit immediate from memory
    fn read_d8(&mut self) -> u8 {
        let imm = self.mmu.read(self.pc);
        self.pc = self.pc.wrapping_add(1);

        imm
    }

    /// 16-bit load
    fn ld_r16_d16(&mut self, reg: Reg16) {
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
    fn and_r8(&mut self, reg: u8) {
        debug!("AND {}", Self::reg_to_string(reg));

        self.a &= self.read_r8(reg);
    }

    /// 8-bit OR
    fn or_r8(&mut self, reg: u8) {
        debug!("OR {}", Self::reg_to_string(reg));

        self.a |= self.read_r8(reg);
    }

    /// 8-bit XOR
    fn xor_r8(&mut self, reg: u8) {
        debug!("XOR {}", Self::reg_to_string(reg));

        self.a ^= self.read_r8(reg);
    }

    fn ldi_hl_a(&mut self) {
        debug!("LD (HL+), A");

        let addr = self.hl();
        self.mmu.write(addr, self.a);
        let hl = self.hl();
        self.set_hl(hl.wrapping_add(1));
    }

    fn ldd_hl_a(&mut self) {
        debug!("LD (HL-), A");

        let addr = self.hl();
        self.mmu.write(addr, self.a);
        let hl = self.hl();
        self.set_hl(hl.wrapping_sub(1));
    }

    fn ldi_a_hl(&mut self) {
        debug!("LD A, (HL+)");

        let addr = self.hl();
        self.a = self.mmu.read(addr);
        let hl = self.hl();
        self.set_hl(hl.wrapping_add(1));
    }

    fn ldd_a_hl(&mut self) {
        debug!("LD A, (HL-)");

        let addr = self.hl();
        self.a = self.mmu.read(addr);
        let hl = self.hl();
        self.set_hl(hl.wrapping_sub(1));
    }

    /// Test bit
    fn bit(&mut self, pos: u8, reg: u8) {
        debug!("BIT {}, {}", pos, Self::reg_to_string(reg));

        let z = self.read_r8(reg) >> pos & 1;
        self.f = (self.f & 0b00011111) | (z << pos) | 0b00100000;
    }

    fn jr_nz_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR NZ, {}", offset);

        if self.f & 0b10000000 != 0 {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn jr_nc_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR NC, {}", offset);

        if self.f & 0b00010000 != 0 {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn jr_z_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR Z, {}", offset);

        if self.f & 0b10000000 == 0 {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn jr_c_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR C, {}", offset);

        if self.f & 0b00010000 == 0 {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn ld_io_d8_a(&mut self) {
        let addr = 0xff00 | self.read_d8() as u16;

        debug!("LD 0x{:04x}, A", addr);

        self.mmu.write(addr, self.a);
    }

    fn ld_a_io_d8(&mut self) {
        let addr = 0xff00 | self.read_d8() as u16;

        debug!("LD A, 0x{:04x}", addr);

        self.a = self.mmu.read(addr);
    }

    fn ld_io_c_a(&mut self) {
        let addr = 0xff00 | self.c as u16;

        debug!("LD (0xff00+C), A");

        self.mmu.write(addr, self.a);
    }

    fn ld_a_io_c(&mut self) {
        let addr = 0xff00 | self.c as u16;

        debug!("LD A, (0xff00+C)");

        self.a = self.mmu.read(addr);
    }

    fn ld_r8_d8(&mut self, reg: u8) {
        let imm = self.read_d8();

        debug!("LD {}, 0x{:02x}", Self::reg_to_string(reg), imm);

        self.write_r8(reg, imm);
    }

    fn inc_r8(&mut self, reg: u8) {
        debug!("INC {}", Self::reg_to_string(reg));

        let val = self.read_r8(reg);
        self.write_r8(reg, val.wrapping_add(1));
    }

    fn dec_r8(&mut self, reg: u8) {
        debug!("DEC {}", Self::reg_to_string(reg));

        let val = self.read_r8(reg);
        self.write_r8(reg, val.wrapping_sub(1));
    }

    /// Prefixed instructions
    fn prefix(&mut self) {
        let opcode = self.read_d8();
        let pos = opcode >> 3 & 0x7;
        let reg = opcode & 0x7;

        match opcode {
            0x40...0x7f => self.bit(pos, reg),
            _ => println!("Unimplemented opcode 0xcb 0x{:x}", opcode),
        }
    }

    pub fn step(&mut self) {
        let opcode = self.read_d8();

        match opcode {
            // LD reg, imm
            0x01 => self.ld_r16_d16(Reg16::BC),
            0x11 => self.ld_r16_d16(Reg16::DE),
            0x21 => self.ld_r16_d16(Reg16::HL),
            0x31 => self.ld_r16_d16(Reg16::SP),

            // Conditional jump
            0x20 => self.jr_nz_d8(),
            0x30 => self.jr_nc_d8(),
            0x28 => self.jr_z_d8(),
            0x38 => self.jr_c_d8(),

            // AND reg
            0xa0...0xa7 => self.and_r8(opcode & 7),

            // OR reg
            0xb0...0xb7 => self.or_r8(opcode & 7),

            // XOR reg
            0xa8...0xaf => self.xor_r8(opcode & 7),

            // LDI, LDD
            0x22 => self.ldi_hl_a(),
            0x32 => self.ldd_hl_a(),
            0x2a => self.ldi_a_hl(),
            0x3a => self.ldd_a_hl(),

            // LD IO port
            0xe0 => self.ld_io_d8_a(),
            0xf0 => self.ld_a_io_d8(),
            0xe2 => self.ld_io_c_a(),
            0xf2 => self.ld_a_io_c(),

            // LD imm
            0x06 | 0x0e | 0x16 | 0x1e | 0x26 | 0x2e | 0x36 | 0x3e => self.ld_r8_d8(opcode >> 3 & 7),

            // INC imm
            0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x34 | 0x3c => self.inc_r8(opcode >> 3 & 7),

            // DEC imm
            0x05 | 0x0b | 0x15 | 0x1b | 0x25 | 0x2b | 0x35 | 0x3b => self.dec_r8(opcode >> 3 & 7),

            0xcb => self.prefix(),
            _ => panic!("Unimplemented opcode 0x{:x}", opcode),
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
