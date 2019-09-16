use mmu::MMU;

pub struct CPU {
    pub mmu: MMU,
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
    ime: bool,
    tick: u8, // This is T-cycle (4.194304 MHz), not M-cycle
    halted: bool,
}

impl CPU {
    /// Creates a new `CPU`
    pub fn new(rom_name: &str) -> Self {
        CPU {
            mmu: MMU::new(rom_name),
            pc: 0x100,
            sp: 0,
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            ime: false,
            tick: 0,
            halted: false,
        }
    }

    /// Reads AF register
    fn af(&self) -> u16 {
        (self.a as u16) << 8 | self.f as u16
    }

    /// Writes AF register
    fn set_af(&mut self, val: u16) {
        self.a = (val >> 8 & 0xff) as u8;
        self.f = (val & 0xff) as u8;
    }

    /// Reads BC register
    fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    /// Writes BC register
    fn set_bc(&mut self, val: u16) {
        self.b = (val >> 8 & 0xff) as u8;
        self.c = (val & 0xff) as u8;
    }

    /// Reads DE register
    fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    /// Writes DE register
    fn set_de(&mut self, val: u16) {
        self.d = (val >> 8 & 0xff) as u8;
        self.e = (val & 0xff) as u8;
    }

    /// Reads HL register
    fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    /// Writes HL register
    fn set_hl(&mut self, val: u16) {
        self.h = (val >> 8 & 0xff) as u8;
        self.l = (val & 0xff) as u8;
    }

    /// Sets Z flag
    fn set_f_z(&mut self, z: bool) {
        self.f = (self.f & !(1 << 7)) | (u8::from(z) << 7);
    }

    /// Returns Z flag
    fn f_z(&self) -> bool {
        (self.f >> 7) & 1 == 1
    }

    /// Sets N flag
    fn set_f_n(&mut self, n: bool) {
        self.f = (self.f & !(1 << 6)) | (u8::from(n) << 6);
    }

    /// Returns N flag
    fn f_n(&self) -> bool {
        (self.f >> 6) & 1 == 1
    }

    /// Sets H flag
    fn set_f_h(&mut self, h: bool) {
        self.f = (self.f & !(1 << 5)) | (u8::from(h) << 5);
    }

    /// Returns H flag
    fn f_h(&self) -> bool {
        (self.f >> 5) & 1 == 1
    }

    /// Sets C flag
    fn set_f_c(&mut self, c: bool) {
        self.f = (self.f & !(1 << 4)) | (u8::from(c) << 4);
    }

    /// Returns C flag
    fn f_c(&self) -> bool {
        (self.f >> 4) & 1 == 1
    }

    /// Converst 8-bit register index to name
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

    /// Converst 16-bit register index to name
    fn reg16_to_string(idx: u8) -> String {
        match idx {
            0 => String::from("BC"),
            1 => String::from("DE"),
            2 => String::from("HL"),
            3 => String::from("SP"),
            _ => panic!("Invalid operand index: {}", idx),
        }
    }

    /// Writes 8-bit operand
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
                self.write_mem8(hl, val);
            }
            7 => self.a = val,
            _ => panic!("Invalid operand index: {}", idx),
        }
    }

    /// Reads 8-bit operand
    fn read_r8(&mut self, idx: u8) -> u8 {
        match idx {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => {
                let hl = self.hl();
                self.read_mem8(hl)
            }
            7 => self.a,
            _ => panic!("Invalid operand index: {}", idx),
        }
    }

    /// Writes 16-bit operand
    fn write_r16(&mut self, idx: u8, val: u16) {
        match idx {
            0 => self.set_bc(val),
            1 => self.set_de(val),
            2 => self.set_hl(val),
            3 => self.sp = val,
            _ => panic!("Invalid operand index: {}", idx),
        }
    }

    /// Reads 16-bit operand
    fn read_r16(&mut self, idx: u8) -> u16 {
        match idx {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            3 => self.sp,
            _ => panic!("Invalid operand index: {}", idx),
        }
    }

    /// Reads 8-bit immediate from memory
    fn read_d8(&mut self) -> u8 {
        let pc = self.pc;
        let imm = self.read_mem8(pc);
        self.pc = self.pc.wrapping_add(1);

        imm
    }

    /// Reads 16-bit immediate from memory
    fn read_d16(&mut self) -> u16 {
        let pc = self.pc;
        let imm = self.read_mem16(pc);
        self.pc = self.pc.wrapping_add(2);

        imm
    }

    /// Checks branch condition
    fn cc(&self, idx: u8) -> bool {
        match idx {
            0 => !self.f_z(),
            1 => self.f_z(),
            2 => !self.f_c(),
            3 => self.f_c(),
            _ => panic!("Invalid branch condition index: {}", idx),
        }
    }

    /// Converts branch condition to name
    fn cc_to_string(idx: u8) -> String {
        match idx {
            0 => String::from("NZ"),
            1 => String::from("Z"),
            2 => String::from("NC"),
            3 => String::from("C"),
            _ => panic!("Invalid branch condition index: {}", idx),
        }
    }

    /// Writes 8-bit value to memory
    fn write_mem8(&mut self, addr: u16, val: u8) {
        self.mmu.write(addr, val);

        self.tick += 4;
    }

    /// Reads 8-bit value from memory
    fn read_mem8(&mut self, addr: u16) -> u8 {
        let ret = self.mmu.read(addr);

        self.tick += 4;

        ret
    }

    /// Writes 16-bit value to memory
    fn write_mem16(&mut self, addr: u16, val: u16) {
        self.write_mem8(addr, (val & 0xff) as u8);
        self.write_mem8(addr.wrapping_add(1), (val >> 8) as u8);
    }

    /// Reads 16-bit value from memory
    fn read_mem16(&mut self, addr: u16) -> u16 {
        let lo = self.read_mem8(addr);
        let hi = self.read_mem8(addr.wrapping_add(1));

        (hi as u16) << 8 | lo as u16
    }

    /// NOP
    fn nop(&mut self) {
        trace!("NOP");
    }

    /// LD r16, d16
    fn ld_r16_d16(&mut self, reg: u8) {
        let val = self.read_d16();

        trace!("LD {}, 0x{:04x}", Self::reg16_to_string(reg), val);

        self.write_r16(reg, val);
    }

    /// LD (d16), SP
    fn ld_ind_d16_sp(&mut self) {
        let addr = self.read_d16();
        let sp = self.sp;

        trace!("LD (0x{:04x}), SP", addr);

        self.write_mem16(addr, sp);
    }

    /// LD SP, HL
    fn ld_sp_hl(&mut self) {
        trace!("LD SP, HL");

        self.tick += 4;

        self.sp = self.hl();
    }

    /// ADD HL, r16
    fn add_hl_r16(&mut self, reg: u8) {
        trace!("ADD HL, {}", Self::reg16_to_string(reg));

        let hl = self.hl();
        let val = self.read_r16(reg);

        let half_carry = (hl & 0xfff) + (val & 0xfff) > 0xfff;
        let (res, carry) = hl.overflowing_add(val);
        self.set_hl(res);

        self.tick += 4;

        self.set_f_n(false);
        self.set_f_h(half_carry);
        self.set_f_c(carry);
    }

    fn _add_sp(&mut self, offset: i8) -> u16 {
        let val = offset as u16;

        let half_carry = (self.sp & 0x0f) + (val & 0x0f) > 0x0f;
        let carry = (self.sp & 0xff) + (val & 0xff) > 0xff;

        self.set_f_z(false);
        self.set_f_n(false);
        self.set_f_h(half_carry);
        self.set_f_c(carry);

        self.sp.wrapping_add(val)
    }

    /// ADD SP, d8
    fn add_sp_d8(&mut self) {
        let val = self.read_d8() as i8;

        trace!("ADD SP, {}", val);

        self.sp = self._add_sp(val);

        self.tick += 8;
    }

    /// LD HL, SP+d8
    fn ld_hl_sp_d8(&mut self) {
        let offset = self.read_d8() as i8;

        trace!("LD HL, SP{:+}", offset);

        self.tick += 4;

        let res = self._add_sp(offset);
        self.set_hl(res);
    }

    /// AND r8
    fn and_r8(&mut self, reg: u8) {
        trace!("AND {}", Self::reg_to_string(reg));

        let res = self.a & self.read_r8(reg);

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(true);
        self.set_f_c(false);
    }

    /// OR r8
    fn or_r8(&mut self, reg: u8) {
        trace!("OR {}", Self::reg_to_string(reg));

        let res = self.a | self.read_r8(reg);

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(false);
    }

    /// XOR r8
    fn xor_r8(&mut self, reg: u8) {
        trace!("XOR {}", Self::reg_to_string(reg));

        let res = self.a ^ self.read_r8(reg);

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(false);
    }

    /// CP r8
    fn cp_r8(&mut self, reg: u8) {
        trace!("CP {}", Self::reg_to_string(reg));

        let a = self.a;
        let val = self.read_r8(reg);

        self.set_f_z(a == val);
        self.set_f_n(true);
        self.set_f_h(a & 0x0f < val & 0x0f);
        self.set_f_c(a < val);
    }

    /// Decimal adjust register A
    fn daa(&mut self) {
        trace!("DAA");

        let mut a = self.a;

        if !self.f_n() {
            if self.f_c() || a > 0x99 {
                a = a.wrapping_add(0x60);
                self.set_f_c(true);
            }
            if self.f_h() || a & 0x0f > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if self.f_c() {
                a = a.wrapping_sub(0x60);
            }
            if self.f_h() {
                a = a.wrapping_sub(0x06);
            }
        }

        self.a = a;

        self.set_f_z(a == 0);
        self.set_f_h(false);
    }

    /// Complement A
    fn cpl(&mut self) {
        trace!("CPL");

        self.a = !self.a;
        self.set_f_n(true);
        self.set_f_h(true);
    }

    /// Complement carry flag
    fn ccf(&mut self) {
        trace!("CCF");

        self.set_f_n(false);
        self.set_f_h(false);

        let c = self.f_c();
        self.set_f_c(!c);
    }

    /// Set carry flag
    fn scf(&mut self) {
        trace!("SCF");

        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(true);
    }

    fn _add(&mut self, val: u8) {
        let half_carry = (self.a & 0xf) + (val & 0xf) > 0xf;
        let (res, carry) = self.a.overflowing_add(val);

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(half_carry);
        self.set_f_c(carry);
    }

    /// ADD r8
    fn add_r8(&mut self, reg: u8) {
        let val = self.read_r8(reg);

        trace!("ADD {}", Self::reg_to_string(reg));

        self._add(val);
    }

    /// ADC r8
    fn adc_r8(&mut self, reg: u8) {
        let val = self.read_r8(reg);

        trace!("ADC {}", Self::reg_to_string(reg));

        self._adc(val);
    }

    /// SUB r8
    fn sub_r8(&mut self, reg: u8) {
        let val = self.read_r8(reg);

        trace!("SUB {}", Self::reg_to_string(reg));

        self._sub(val);
    }

    /// SBC r8
    fn sbc_r8(&mut self, reg: u8) {
        let val = self.read_r8(reg);

        trace!("SBC {}", Self::reg_to_string(reg));

        self._sbc(val);
    }

    /// ADD d8
    fn add_d8(&mut self) {
        let val = self.read_d8();

        trace!("ADD 0x{:02x}", val);

        self._add(val);
    }

    fn _sub(&mut self, val: u8) {
        let half_carry = (self.a & 0xf) < (val & 0xf);
        let (res, carry) = self.a.overflowing_sub(val);

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(true);
        self.set_f_h(half_carry);
        self.set_f_c(carry);
    }

    /// SUB d8
    fn sub_d8(&mut self) {
        let val = self.read_d8();

        trace!("SUB 0x{:02x}", val);

        self._sub(val);
    }

    fn _adc(&mut self, val: u8) {
        let c = if self.f_c() { 1 } else { 0 };

        let res = self.a.wrapping_add(val).wrapping_add(c);
        let half_carry = (self.a & 0xf) + (val & 0xf) + c > 0xf;
        let carry = (self.a as u16) + (val as u16) + (c as u16) > 0xff;

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(half_carry);
        self.set_f_c(carry);
    }

    /// ADC d8
    fn adc_d8(&mut self) {
        let val = self.read_d8();

        trace!("ADC 0x{:02x}", val);

        self._adc(val);
    }

    fn _sbc(&mut self, val: u8) {
        let c = if self.f_c() { 1 } else { 0 };

        let res = self.a.wrapping_sub(val).wrapping_sub(c);
        let half_carry = (self.a & 0xf) < (val & 0xf) + c;
        let carry = (self.a as u16) < (val as u16) + (c as u16);

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(true);
        self.set_f_h(half_carry);
        self.set_f_c(carry);
    }

    /// SBC d8
    fn sbc_d8(&mut self) {
        let val = self.read_d8();

        trace!("SBC 0x{:02x}", val);

        self._sbc(val);
    }

    /// AND d8
    fn and_d8(&mut self) {
        let val = self.read_d8();

        trace!("AND 0x{:02x}", val);

        let res = self.a & val;

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(true);
        self.set_f_c(false);
    }

    /// OR d8
    fn or_d8(&mut self) {
        let val = self.read_d8();

        trace!("OR 0x{:02x}", val);

        let res = self.a | val;

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(false);
    }

    /// XOR d8
    fn xor_d8(&mut self) {
        let val = self.read_d8();

        trace!("XOR 0x{:02x}", val);

        let res = self.a ^ val;

        self.a = res;

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(false);
    }

    /// CP d8
    fn cp_d8(&mut self) {
        let imm = self.read_d8();

        trace!("CP 0x{:02x}", imm);

        let a = self.a;

        self.set_f_z(a == imm);
        self.set_f_n(true);
        self.set_f_h(a & 0x0f < imm & 0x0f);
        self.set_f_c(a < imm);
    }

    fn ldi_hl_a(&mut self) {
        trace!("LD (HL+), A");

        let addr = self.hl();
        let a = self.a;
        self.write_mem8(addr, a);
        let hl = self.hl();
        self.set_hl(hl.wrapping_add(1));
    }

    fn ldd_hl_a(&mut self) {
        trace!("LD (HL-), A");

        let addr = self.hl();
        let a = self.a;
        self.write_mem8(addr, a);
        let hl = self.hl();
        self.set_hl(hl.wrapping_sub(1));
    }

    fn ldi_a_hl(&mut self) {
        trace!("LD A, (HL+)");

        let addr = self.hl();
        self.a = self.read_mem8(addr);
        let hl = self.hl();
        self.set_hl(hl.wrapping_add(1));
    }

    fn ldd_a_hl(&mut self) {
        trace!("LD A, (HL-)");

        let addr = self.hl();
        self.a = self.read_mem8(addr);
        let hl = self.hl();
        self.set_hl(hl.wrapping_sub(1));
    }

    fn ld_ind_bc_a(&mut self) {
        trace!("LD (BC), A");

        let addr = self.bc();
        let a = self.a;
        self.write_mem8(addr, a);
    }

    fn ld_ind_de_a(&mut self) {
        trace!("LD (DE), A");

        let addr = self.de();
        let a = self.a;
        self.write_mem8(addr, a);
    }

    fn ld_a_ind_bc(&mut self) {
        trace!("LD A, (BC)");

        let bc = self.bc();

        self.a = self.read_mem8(bc);
    }

    fn ld_a_ind_de(&mut self) {
        trace!("LD A, (DE)");

        let de = self.de();

        self.a = self.read_mem8(de);
    }

    /// Test bit
    fn bit(&mut self, pos: u8, reg: u8) {
        trace!("BIT {}, {}", pos, Self::reg_to_string(reg));

        let z = (self.read_r8(reg) >> pos & 1) == 0;
        self.set_f_z(z);
        self.set_f_n(false);
        self.set_f_h(true);
    }

    /// Set bit
    fn set(&mut self, pos: u8, reg: u8) {
        trace!("SET {}, {}", pos, Self::reg_to_string(reg));

        let val = self.read_r8(reg);
        self.write_r8(reg, val | (1 << pos));
    }

    /// Reset bit
    fn res(&mut self, pos: u8, reg: u8) {
        trace!("RES {}, {}", pos, Self::reg_to_string(reg));

        let val = self.read_r8(reg);
        self.write_r8(reg, val & !(1 << pos));
    }

    fn _rl(&mut self, reg: u8) {
        let orig = self.read_r8(reg);
        let res = (orig << 1) | (if self.f_c() { 1 } else { 0 });
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig >> 7 & 1 == 1);
    }

    /// Rotate left through carry
    fn rl(&mut self, reg: u8) {
        trace!("RL {}", Self::reg_to_string(reg));

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
        trace!("RLC {}", Self::reg_to_string(reg));

        self._rlc(reg);
    }

    fn _rr(&mut self, reg: u8) {
        let orig = self.read_r8(reg);
        let res = (orig >> 1) | (if self.f_c() { 1 } else { 0 } << 7);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig & 1 == 1);
    }

    /// Rotate right through carry
    fn rr(&mut self, reg: u8) {
        trace!("RR {}", Self::reg_to_string(reg));

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
        trace!("RRC {}", Self::reg_to_string(reg));

        self._rrc(reg);
    }

    /// Shift left into carry
    fn sla(&mut self, reg: u8) {
        trace!("SLA {}", Self::reg_to_string(reg));

        let orig = self.read_r8(reg);
        let res = orig << 1;
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig & 0x80 > 0);
    }

    /// Shift right into carry
    fn sra(&mut self, reg: u8) {
        trace!("SRA {}", Self::reg_to_string(reg));

        let orig = self.read_r8(reg);
        let res = (orig >> 1) | (orig & 0x80);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig & 1 > 0);
    }

    /// Swap low/hi-nibble
    fn swap(&mut self, reg: u8) {
        trace!("SWAP {}", Self::reg_to_string(reg));

        let orig = self.read_r8(reg);
        let res = ((orig & 0x0f) << 4) | ((orig & 0xf0) >> 4);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(false);
    }

    /// Shift right through carry
    fn srl(&mut self, reg: u8) {
        trace!("SRL {}", Self::reg_to_string(reg));

        let orig = self.read_r8(reg);
        let res = orig >> 1;
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_n(false);
        self.set_f_h(false);
        self.set_f_c(orig & 1 == 1);
    }

    fn _jp(&mut self, addr: u16) {
        self.pc = addr;

        self.tick += 4;
    }

    fn jp_cc_d8(&mut self, cci: u8) {
        let addr = self.read_d16();

        trace!("JP {}, 0x{:04x}", Self::cc_to_string(cci), addr);

        if self.cc(cci) {
            self._jp(addr);
        }
    }

    /// Unconditional jump to d16
    fn jp_d16(&mut self) {
        let address = self.read_d16();

        trace!("JP 0x{:04x}", address);

        self._jp(address);
    }

    /// Unconditional jump to HL
    fn jp_hl(&mut self) {
        trace!("JP (HL)");

        self.pc = self.hl();
    }

    /// Jump to pc+d8 if CC
    fn jr_cc_d8(&mut self, cci: u8) {
        let offset = self.read_d8() as i8;

        trace!("JR {}, {}", Self::cc_to_string(cci), offset);

        if self.cc(cci) {
            self._jr(offset);
        }
    }

    fn _jr(&mut self, offset: i8) {
        self.pc = self.pc.wrapping_add(offset as u16);

        self.tick += 4;
    }

    /// Jump to pc+d8
    fn jr_d8(&mut self) {
        let offset = self.read_d8() as i8;

        trace!("JR {}", offset);

        self._jr(offset);
    }

    fn ld_io_d8_a(&mut self) {
        let offset = self.read_d8() as u16;
        let addr = 0xff00 | offset;
        let a = self.a;

        trace!("LD (0xff00+0x{:02x}), A", offset);

        self.write_mem8(addr, a);
    }

    fn ld_a_io_d8(&mut self) {
        let offset = self.read_d8() as u16;
        let addr = 0xff00 | offset;

        trace!("LD A, (0xff00+0x{:02x})", offset);

        self.a = self.read_mem8(addr);
    }

    fn ld_io_c_a(&mut self) {
        let addr = 0xff00 | self.c as u16;
        let a = self.a;

        trace!("LD (0xff00+C), A");

        self.write_mem8(addr, a);
    }

    fn ld_a_io_c(&mut self) {
        let addr = 0xff00 | self.c as u16;

        trace!("LD A, (0xff00+C)");

        self.a = self.read_mem8(addr);
    }

    /// LD r8, d8
    fn ld_r8_d8(&mut self, reg: u8) {
        let imm = self.read_d8();

        trace!("LD {}, 0x{:02x}", Self::reg_to_string(reg), imm);

        self.write_r8(reg, imm);
    }

    /// INC r8
    fn inc_r8(&mut self, reg: u8) {
        trace!("INC {}", Self::reg_to_string(reg));

        let orig = self.read_r8(reg);
        let res = orig.wrapping_add(1);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_h(orig & 0x0f == 0x0f);
        self.set_f_n(false);
    }

    /// DEC r8
    fn dec_r8(&mut self, reg: u8) {
        trace!("DEC {}", Self::reg_to_string(reg));

        let orig = self.read_r8(reg);
        let res = orig.wrapping_sub(1);
        self.write_r8(reg, res);

        self.set_f_z(res == 0);
        self.set_f_h(orig & 0x0f == 0x00);
        self.set_f_n(true);
    }

    /// LD r8, r8
    fn ld_r8_r8(&mut self, reg1: u8, reg2: u8) {
        trace!(
            "LD {}, {}",
            Self::reg_to_string(reg1),
            Self::reg_to_string(reg2)
        );

        let val = self.read_r8(reg2);
        self.write_r8(reg1, val);
    }

    fn _call(&mut self, addr: u16) {
        self.sp = self.sp.wrapping_sub(2);
        let sp = self.sp;
        let pc = self.pc;

        self.tick += 4;

        self.write_mem16(sp, pc);
        self.pc = addr;
    }

    /// CALL d16
    fn call_d16(&mut self) {
        let addr = self.read_d16();

        trace!("CALL 0x{:04x}", addr);

        self._call(addr);
    }

    /// CALL CC, d16
    fn call_cc_d16(&mut self, cci: u8) {
        let addr = self.read_d16();

        trace!("CALL {}, 0x{:04x}", Self::cc_to_string(cci), addr);

        if self.cc(cci) {
            self._call(addr);
        }
    }

    fn rst(&mut self, addr: u8) {
        trace!("RST 0x{:02x}", addr);

        self._call(addr as u16);
    }

    fn _ret(&mut self) {
        let sp = self.sp;
        self.pc = self.read_mem16(sp);
        self.sp = self.sp.wrapping_add(2);

        self.tick += 4;
    }

    /// RET
    fn ret(&mut self) {
        trace!("RET");

        self._ret();
    }

    /// RET CC
    fn ret_cc(&mut self, cci: u8) {
        trace!("RET {}", Self::cc_to_string(cci));

        self.tick += 4;

        if self.cc(cci) {
            self._ret();
        }
    }

    /// PUSH BC
    fn push_bc(&mut self) {
        trace!("PUSH BC");

        self.sp = self.sp.wrapping_sub(2);
        let val = self.bc();
        let sp = self.sp;

        self.tick += 4;

        self.write_mem16(sp, val);
    }

    /// PUSH DE
    fn push_de(&mut self) {
        trace!("PUSH DE");

        self.sp = self.sp.wrapping_sub(2);
        let val = self.de();
        let sp = self.sp;

        self.tick += 4;

        self.write_mem16(sp, val);
    }

    /// PUSH HL
    fn push_hl(&mut self) {
        trace!("PUSH HL");

        self.sp = self.sp.wrapping_sub(2);
        let val = self.hl();
        let sp = self.sp;

        self.tick += 4;

        self.write_mem16(sp, val);
    }

    /// PUSH AF
    fn push_af(&mut self) {
        trace!("PUSH AF");

        self.sp = self.sp.wrapping_sub(2);
        let val = self.af();
        let sp = self.sp;

        self.tick += 4;

        self.write_mem16(sp, val);
    }

    /// POP BC
    fn pop_bc(&mut self) {
        trace!("POP BC");

        let sp = self.sp;
        let val = self.read_mem16(sp);
        self.set_bc(val);
        self.sp = self.sp.wrapping_add(2);
    }

    /// POP DE
    fn pop_de(&mut self) {
        trace!("POP DE");

        let sp = self.sp;
        let val = self.read_mem16(sp);
        self.set_de(val);
        self.sp = self.sp.wrapping_add(2);
    }

    /// POP HL
    fn pop_hl(&mut self) {
        trace!("POP HL");

        let sp = self.sp;
        let val = self.read_mem16(sp);
        self.set_hl(val);
        self.sp = self.sp.wrapping_add(2);
    }

    /// POP AF
    fn pop_af(&mut self) {
        trace!("POP AF");

        let sp = self.sp;
        // lower nibble of F is always zero
        let val = self.read_mem16(sp) & 0xfff0;
        self.set_af(val);
        self.sp = self.sp.wrapping_add(2);
    }

    fn rlca(&mut self) {
        trace!("RLCA");

        self._rlc(7);
        self.set_f_z(false);
    }

    fn rla(&mut self) {
        trace!("RLA");

        self._rl(7);
        self.set_f_z(false);
    }

    fn rrca(&mut self) {
        trace!("RLRA");

        self._rrc(7);
        self.set_f_z(false);
    }

    fn rra(&mut self) {
        trace!("RRA");

        self._rr(7);
        self.set_f_z(false);
    }

    fn inc_r16(&mut self, reg: u8) {
        trace!("INC {}", Self::reg16_to_string(reg));

        let val = self.read_r16(reg);
        self.write_r16(reg, val.wrapping_add(1));

        self.tick += 4;
    }

    fn dec_r16(&mut self, reg: u8) {
        trace!("DEC {}", Self::reg16_to_string(reg));

        let val = self.read_r16(reg);
        self.write_r16(reg, val.wrapping_sub(1));

        self.tick += 4;
    }

    fn ld_ind_d16_a(&mut self) {
        let addr = self.read_d16();
        let a = self.a;

        trace!("LD (0x{:04x}), A", addr);

        self.write_mem8(addr, a);
    }

    fn ld_a_ind_d16(&mut self) {
        let addr = self.read_d16();

        trace!("LD A, (0x{:04x})", addr);

        self.a = self.read_mem8(addr);
    }

    /// Disable interrupt
    fn di(&mut self) {
        trace!("DI");

        self.ime = false;
    }

    /// Enable interrupt
    fn ei(&mut self) {
        trace!("EI");

        self.ime = true;
    }

    /// Enable interrupt and return
    fn reti(&mut self) {
        trace!("RETI");

        self.ime = true;

        self._ret();
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
            0x20...0x27 => self.sla(reg),
            0x28...0x2f => self.sra(reg),
            0x30...0x37 => self.swap(reg),
            0x38...0x3f => self.srl(reg),
            0x40...0x7f => self.bit(pos, reg),
            0x80...0xbf => self.res(pos, reg),
            0xc0...0xff => self.set(pos, reg),
            _ => panic!("Unimplemented opcode 0xcb 0x{:x}", opcode),
        }
    }

    /// HALT
    fn halt(&mut self) {
        trace!("HALT");

        if self.ime {
            self.halted = true;
        }
    }

    /// Execute a single instruction and handle IRQs.
    pub fn step(&mut self) -> u8 {
        let mut total_tick = 0;

        self.tick = 0;

        if self.halted {
            self.tick += 4;
        } else {
            self.fetch_and_exec();
        }

        total_tick += self.tick;

        self.mmu.update(self.tick);

        if self.ime {
            self.tick = 0;
            self.check_irqs();
            self.mmu.update(self.tick);

            total_tick += self.tick;
        }

        total_tick
    }

    /// Checks IRQs and execute ISRs if requested.
    fn check_irqs(&mut self) {
        // Bit 0 has the highest priority
        for i in 0..5 {
            let irq = self.mmu.int_flag & (1 << i) > 0;
            let ie = self.mmu.int_enable & (1 << i) > 0;

            // If interrupt is requested and enabled
            if irq && ie {
                self.call_isr(i);
                break;
            }
        }
    }

    /// Calls requested interrupt service routine.
    fn call_isr(&mut self, id: u8) {
        // Reset corresponding bit in IF
        self.mmu.int_flag &= !(1 << id);
        // Clear IME (disable any further interrupts)
        self.ime = false;
        self.halted = false;

        let isr: u16 = match id {
            0 => 0x40,
            1 => 0x48,
            2 => 0x50,
            3 => 0x80,
            4 => 0x70,
            _ => panic!("Invalid IRQ id {}", id),
        };

        self.tick += 8;

        debug!("Calling ISR 0x{:02x}", isr);

        self._call(isr);
    }

    /// Fetches and executes a single instructions.
    fn fetch_and_exec(&mut self) {
        let opcode = self.read_d8();
        let reg = opcode & 7;
        let reg2 = opcode >> 3 & 7;

        match opcode {
            // NOP
            0x00 => self.nop(),

            // LD r16, d16
            0x01 | 0x11 | 0x21 | 0x31 => self.ld_r16_d16(opcode >> 4),

            // LD (d16), SP
            0x08 => self.ld_ind_d16_sp(),

            // LD SP, HL
            0xf9 => self.ld_sp_hl(),

            // LD A, (r16)
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

            // Conditional absolute jump
            0xc2 | 0xd2 | 0xca | 0xda => self.jp_cc_d8(reg2),

            // Unconditional absolute jump
            0xc3 => self.jp_d16(),
            0xe9 => self.jp_hl(),

            // Conditional relative jump
            0x20 | 0x30 | 0x28 | 0x38 => self.jr_cc_d8(reg2 - 4),

            // Unconditional relative jump
            0x18 => self.jr_d8(),

            // Bit rotate on A
            0x07 => self.rlca(),
            0x17 => self.rla(),
            0x0f => self.rrca(),
            0x1f => self.rra(),

            // Arithmethic/logical operation on 16-bit register
            0x09 | 0x19 | 0x29 | 0x39 => self.add_hl_r16(opcode >> 4),
            0xe8 => self.add_sp_d8(),
            0xf8 => self.ld_hl_sp_d8(),

            // Arithmethic/logical operation on 8-bit register
            0x80...0x87 => self.add_r8(reg),
            0x88...0x8f => self.adc_r8(reg),
            0x90...0x97 => self.sub_r8(reg),
            0x98...0x9f => self.sbc_r8(reg),
            0xa0...0xa7 => self.and_r8(reg),
            0xb0...0xb7 => self.or_r8(reg),
            0xa8...0xaf => self.xor_r8(reg),
            0xb8...0xbf => self.cp_r8(reg),

            // DAA
            0x27 => self.daa(),

            // CPL
            0x2f => self.cpl(),

            // SCF, CCF
            0x37 => self.scf(),
            0x3f => self.ccf(),

            // Arithmethic/logical operation on A
            0xc6 => self.add_d8(),
            0xd6 => self.sub_d8(),
            0xe6 => self.and_d8(),
            0xf6 => self.or_d8(),
            0xce => self.adc_d8(),
            0xde => self.sbc_d8(),
            0xee => self.xor_d8(),
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
            0x06 | 0x0e | 0x16 | 0x1e | 0x26 | 0x2e | 0x36 | 0x3e => self.ld_r8_d8(reg2),

            // INC r8
            0x04 | 0x0c | 0x14 | 0x1c | 0x24 | 0x2c | 0x34 | 0x3c => self.inc_r8(reg2),

            // DEC r8
            0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d => self.dec_r8(reg2),

            // LD r8, r8
            0x40...0x75 | 0x77...0x7f => self.ld_r8_r8(reg2, reg),

            // LD (d16), A
            0xea => self.ld_ind_d16_a(),

            // LD A, (d16)
            0xfa => self.ld_a_ind_d16(),

            // INC, DEC r16
            0x03 | 0x13 | 0x23 | 0x33 => self.inc_r16(opcode >> 4),
            0x0b | 0x1b | 0x2b | 0x3b => self.dec_r16(opcode >> 4),

            // Unconditional call
            0xcd => self.call_d16(),

            // Conditional call
            0xc4 | 0xd4 | 0xcc | 0xdc => self.call_cc_d16(reg2),

            // Unconditional ret
            0xc9 => self.ret(),

            // Conditional ret
            0xc0 | 0xd0 | 0xc8 | 0xd8 => self.ret_cc(reg2),

            // RETI
            0xd9 => self.reti(),

            // RST
            0xc7 | 0xcf | 0xd7 | 0xdf | 0xe7 | 0xef | 0xf7 | 0xff => self.rst(opcode - 0xc7),

            // DI, EI
            0xf3 => self.di(),
            0xfb => self.ei(),

            // CB prefixed
            0xcb => self.prefix(),

            // HALT
            0x76 => self.halt(),

            _ => panic!("Unimplemented opcode 0x{:x}", opcode),
        }
    }

    /// Dumps current CPU state.
    #[allow(dead_code)]
    pub fn dump(&self) {
        println!("CPU State:");
        println!("PC: 0x{:04x}  SP: 0x{:04x}", self.pc, self.sp);
        println!("AF: 0x{:04x}  BC: 0x{:04x}", self.af(), self.bc());
        println!("DE: 0x{:04x}  HL: 0x{:04x}", self.de(), self.hl());
        println!("T:  {}", self.tick);
    }
}
