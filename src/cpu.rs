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

    /// Write BC register
    fn set_bc(&mut self, val: u16) {
        self.b = (val >> 8 & 0xff) as u8;
        self.c = (val & 0xff) as u8;
    }

    /// Read DE register
    fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    /// Write DE register
    fn set_de(&mut self, val: u16) {
        self.d = (val >> 8 & 0xff) as u8;
        self.e = (val & 0xff) as u8;
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

    fn set_f_z(&mut self, z: bool) {
        self.f = (self.f & !(1 << 7)) | (u8::from(z) << 7);
    }

    fn f_z(&self) -> bool {
        (self.f >> 7) & 1 == 1
    }

    fn set_f_h(&mut self, h: bool) {
        self.f = (self.f & !(1 << 6)) | (u8::from(h) << 6);
    }

    fn f_h(&self) -> bool {
        (self.f >> 6) & 1 == 1
    }

    fn set_f_n(&mut self, n: bool) {
        self.f = (self.f & !(1 << 5)) | (u8::from(n) << 5);
    }

    fn f_n(&self) -> bool {
        (self.f >> 5) & 1 == 1
    }

    fn set_f_c(&mut self, c: bool) {
        self.f = (self.f & !(1 << 4)) | (u8::from(c) << 4);
    }

    fn f_c(&self) -> bool {
        (self.f >> 4) & 1 == 1
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

    /// 8-bit CP
    fn cp_r8(&mut self, reg: u8) {
        debug!("CP {}", Self::reg_to_string(reg));

        let a = self.a;
        let val = self.read_r8(reg);

        self.set_f_z(a == val);
        self.set_f_n(true);
        // self.set_f_h(??);
        self.set_f_c(a < val);
    }

    /// 8-bit CP
    fn cp_d8(&mut self) {
        let imm = self.read_d8();

        debug!("CP 0x{:02x}", imm);

        let a = self.a;

        self.set_f_z(a == imm);
        self.set_f_n(true);
        // TODO self.set_f_h(??);
        self.set_f_c(a < imm);
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

    fn ld_ind_bc_a(&mut self) {
        debug!("LD (BC), A");

        let addr = self.bc();
        self.mmu.write(addr, self.a);
    }

    fn ld_ind_de_a(&mut self) {
        debug!("LD (DE), A");

        let addr = self.de();
        self.mmu.write(addr, self.a);
    }

    fn ld_a_ind_bc(&mut self) {
        debug!("LD A, (BC)");

        self.a = self.mmu.read(self.bc());
    }

    fn ld_a_ind_de(&mut self) {
        debug!("LD A, (DE)");

        self.a = self.mmu.read(self.de());
    }

    /// Test bit
    fn bit(&mut self, pos: u8, reg: u8) {
        debug!("BIT {}, {}", pos, Self::reg_to_string(reg));

        let z = (self.read_r8(reg) >> pos & 1) == 0;
        self.set_f_z(z);
        self.set_f_n(false);
        self.set_f_h(true);
    }

    /// Set bit
    fn set(&mut self, pos: u8, reg: u8) {
        debug!("SET {}, {}", pos, Self::reg_to_string(reg));

        let val = self.read_r8(reg);
        self.write_r8(reg, val | (1 << pos));
    }

    /// Reset bit
    fn res(&mut self, pos: u8, reg: u8) {
        debug!("RES {}, {}", pos, Self::reg_to_string(reg));

        let val = self.read_r8(reg);
        self.write_r8(reg, val & !(1 << pos));
    }

    fn _rl(&mut self, reg: u8) {
        let orig = self.read_r8(reg);
        let res = orig.rotate_left(1) | (if self.f_c() { 1 } else { 0 });
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig >> 7 & 1 == 1);
    }

    /// Rotate left through carry
    fn rl(&mut self, reg: u8) {
        debug!("RL {}", Self::reg_to_string(reg));

        self._rl(reg);
    }

    fn _rlc(&mut self, reg: u8) {
        let orig = self.read_r8(reg);
        let res = orig.rotate_left(1);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig >> 7 & 1 == 1);
    }

    /// Rotate left
    fn rlc(&mut self, reg: u8) {
        debug!("RLC {}", Self::reg_to_string(reg));

        self._rlc(reg);
    }

    fn _rr(&mut self, reg: u8) {
        let orig = self.read_r8(reg);
        let res = orig.rotate_right(1) | (if self.f_c() { 1 } else { 0 } << 7);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig & 1 == 1);
    }

    /// Rotate right through carry
    fn rr(&mut self, reg: u8) {
        debug!("RR {}", Self::reg_to_string(reg));

        self._rr(reg);
    }

    fn _rrc(&mut self, reg: u8) {
        let orig = self.read_r8(reg);
        let res = orig.rotate_right(1);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig & 1 == 1);
    }


    /// Rotate right
    fn rrc(&mut self, reg: u8) {
        debug!("RRC {}", Self::reg_to_string(reg));

        self._rrc(reg);
    }

    fn jr_nz_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR NZ, {}", offset);

        if !self.f_z() {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn jr_nc_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR NC, {}", offset);

        if !self.f_c() {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn jr_z_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR Z, {}", offset);

        if self.f_z() {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn jr_c_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR C, {}", offset);

        if self.f_c() {
            self.pc = self.pc.wrapping_add(offset as u16);
        }
    }

    fn jr_d8(&mut self) {
        let offset = self.read_d8() as i8;

        debug!("JR {}", offset);

        self.pc = self.pc.wrapping_add(offset as u16);
    }

    fn ld_io_d8_a(&mut self) {
        let offset = self.read_d8() as u16;
        let addr = 0xff00 | offset;

        debug!("LD (0xff00+0x{:02x}), A", offset);

        self.mmu.write(addr, self.a);
    }

    fn ld_a_io_d8(&mut self) {
        let offset = self.read_d8() as u16;
        let addr = 0xff00 | offset;

        debug!("LD A, (0xff00+0x{:02x})", offset);

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

        let orig = self.read_r8(reg);
        let res = orig.wrapping_add(1);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_h(orig & 0x0f == 0x0f);
        self.set_f_n(false);
    }

    fn dec_r8(&mut self, reg: u8) {
        debug!("DEC {}", Self::reg_to_string(reg));

        let orig = self.read_r8(reg);
        let res = orig.wrapping_sub(1);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_h(orig & 0x0f == 0x00);
        self.set_f_n(true);
    }

    fn ld_r8_r8(&mut self, reg1: u8, reg2: u8) {
        debug!(
            "LD {}, {}",
            Self::reg_to_string(reg1),
            Self::reg_to_string(reg2)
        );

        let val = self.read_r8(reg2);
        self.write_r8(reg1, val);
    }

    fn call(&mut self) {
        let lo = self.read_d8();
        let hi = self.read_d8();

        debug!("CALL 0x{:02x}{:02x}", hi, lo);

        self.sp = self.sp.wrapping_sub(2);
        self.mmu.write(self.sp, (self.pc & 0xff) as u8);
        self.mmu.write(self.sp.wrapping_add(1), (self.pc >> 8 & 0xff) as u8);
        self.pc = (hi as u16) << 8 | lo as u16;
    }

    fn ret(&mut self) {
        debug!("RET");

        let lo = self.mmu.read(self.sp);
        let hi = self.mmu.read(self.sp.wrapping_add(1));

        self.pc = (hi as u16) << 8 | lo as u16;
        self.sp = self.sp.wrapping_add(2);
    }

    fn push_bc(&mut self) {
        debug!("PUSH BC");

        self.sp = self.sp.wrapping_sub(2);
        self.mmu.write(self.sp, self.c);
        self.mmu.write(self.sp.wrapping_add(1), self.b);
    }

    fn push_de(&mut self) {
        debug!("PUSH DE");

        self.sp = self.sp.wrapping_sub(2);
        self.mmu.write(self.sp, self.e);
        self.mmu.write(self.sp.wrapping_add(1), self.d);
    }

    fn push_hl(&mut self) {
        debug!("PUSH HL");

        self.sp = self.sp.wrapping_sub(2);
        self.mmu.write(self.sp, self.l);
        self.mmu.write(self.sp.wrapping_add(1), self.h);
    }

    fn push_af(&mut self) {
        debug!("PUSH AF");

        self.sp = self.sp.wrapping_sub(2);
        self.mmu.write(self.sp, self.f);
        self.mmu.write(self.sp.wrapping_add(1), self.a);
    }

    fn pop_bc(&mut self) {
        debug!("POP BC");

        self.c = self.mmu.read(self.sp);
        self.b = self.mmu.read(self.sp.wrapping_add(1));

        self.sp = self.sp.wrapping_add(2);
    }

    fn pop_de(&mut self) {
        debug!("POP DE");

        self.e = self.mmu.read(self.sp);
        self.d = self.mmu.read(self.sp.wrapping_add(1));

        self.sp = self.sp.wrapping_add(2);
    }

    fn pop_hl(&mut self) {
        debug!("POP HL");

        self.l = self.mmu.read(self.sp);
        self.h = self.mmu.read(self.sp.wrapping_add(1));

        self.sp = self.sp.wrapping_add(2);
    }

    fn pop_af(&mut self) {
        debug!("POP AF");

        self.f = self.mmu.read(self.sp);
        self.a = self.mmu.read(self.sp.wrapping_add(1));

        self.sp = self.sp.wrapping_add(2);
    }

    fn rlca(&mut self) {
        debug!("RLCA");

        self._rlc(7);
        self.set_f_z(false);
    }

    fn rla(&mut self) {
        debug!("RLA");

        self._rl(7);
        self.set_f_z(false);
    }

    fn rrca(&mut self) {
        debug!("RLRA");

        self._rrc(7);
        self.set_f_z(false);
    }

    fn rra(&mut self) {
        debug!("RRA");

        self._rr(7);
        self.set_f_z(false);
    }

    fn inc_bc(&mut self) {
        debug!("INC BC");

        let val = self.bc();
        self.set_bc(val.wrapping_add(1));
    }

    fn inc_de(&mut self) {
        debug!("INC DE");

        let val = self.de();
        self.set_de(val.wrapping_add(1));
    }

    fn inc_hl(&mut self) {
        debug!("INC HL");

        let val = self.hl();
        self.set_hl(val.wrapping_add(1));
    }

    fn inc_sp(&mut self) {
        debug!("INC SP");

        self.sp = self.sp.wrapping_add(1);
    }

    fn ld_ind_d16_a(&mut self) {
        let lo = self.read_d8();
        let hi = self.read_d8();
        let addr = (hi as u16) << 8 | lo as u16;

        debug!("LD (0x{:04x}), A", addr);

        self.mmu.write(addr, self.a);
    }

    /// Prefixed instructions
    fn prefix(&mut self) {
        let opcode = self.read_d8();
        let pos = opcode >> 3 & 0x7;
        let reg = opcode & 0x7;

        match opcode {
            0x00...0x07 => self.rlc(reg),
            0x08...0x0f => self.rrc(reg),
            0x10...0x17 => self.rl(reg),
            0x18...0x1f => self.rr(reg),
            0x40...0x7f => self.bit(pos, reg),
            0x80...0xbf => self.res(pos, reg),
            0xc0...0xff => self.set(pos, reg),
            _ => println!("Unimplemented opcode 0xcb 0x{:x}", opcode),
        }
    }

    pub fn step(&mut self) {
        let opcode = self.read_d8();
        let reg = opcode & 7;
        let reg2 = opcode >> 3 & 7;

        match opcode {
            // LD r16, d16
            0x01 => self.ld_r16_d16(Reg16::BC),
            0x11 => self.ld_r16_d16(Reg16::DE),
            0x21 => self.ld_r16_d16(Reg16::HL),
            0x31 => self.ld_r16_d16(Reg16::SP),

            // LD A, [r16]
            0x02 => self.ld_ind_bc_a(),
            0x12 => self.ld_ind_de_a(),
            0x0a => self.ld_a_ind_bc(),
            0x1a => self.ld_a_ind_de(),

            // PUSH r16
            0xc5 => self.push_bc(),
            0xd5 => self.push_de(),
            0xe5 => self.push_hl(),
            0xf5 => self.push_af(),

            // POP r16
            0xc1 => self.pop_bc(),
            0xd1 => self.pop_de(),
            0xe1 => self.pop_hl(),
            0xf1 => self.pop_af(),

            // Conditional jump
            0x20 => self.jr_nz_d8(),
            0x30 => self.jr_nc_d8(),
            0x28 => self.jr_z_d8(),
            0x38 => self.jr_c_d8(),

            // Unconditional jump
            0x18 => self.jr_d8(),

            0x07 => self.rlca(),
            0x17 => self.rla(),
            0x0f => self.rrca(),
            0x1f => self.rra(),

            // AND r8
            0xa0...0xa7 => self.and_r8(reg),

            // OR r8
            0xb0...0xb7 => self.or_r8(reg),

            // XOR r8
            0xa8...0xaf => self.xor_r8(reg),

            // CP r8
            0xb8...0xbf => self.cp_r8(reg),

            // CP d8
            0xfe => self.cp_d8(),

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

            // LD r8, d8
            0x06|0x0e|0x16|0x1e|0x26|0x2e|0x36|0x3e => self.ld_r8_d8(reg2),

            // INC r8
            0x04|0x0c|0x14|0x1c|0x24|0x2c|0x34|0x3c => self.inc_r8(reg2),

            // DEC r8
            0x05|0x0d|0x15|0x1d|0x25|0x2d|0x35|0x3d => self.dec_r8(reg2),

            // LD r8, r8
            0x40...0x75 | 0x77...0x7f => self.ld_r8_r8(reg2, reg),

            // LD (d16), A
            0xea => self.ld_ind_d16_a(),

            // INC r16
            0x03 => self.inc_bc(),
            0x13 => self.inc_de(),
            0x23 => self.inc_hl(),
            0x33 => self.inc_sp(),

            // CALL d16
            0xcd => self.call(),

            // RET
            0xc9 => self.ret(),

            // CB prefixed
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
