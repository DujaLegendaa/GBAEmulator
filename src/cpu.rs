use super::bit;
use super::bus::{Bus};
pub struct Z80{
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,

    bus: Bus,
    pub cyclesLeft: u8,
    fetched: u8,
    fetchedSigned: i8,
    currentOpcode: u8,

    pub prefixedInstruction: bool,
    cbFlag: bool,
    branchTaken: bool,
    justBooted: bool,
    halted: bool,
    masterInterrupt: bool
}

pub enum Flags {
    Zero = 7,
    Sub = 6, // only with DAA
    HCarry = 5, // only with DAA
    Carry = 4,
    /*
    When the result of a 8-bit addition is higher than $FF.
    When the result of a 16-bit addition is higher than $FFFF.
    When the result of a subtraction or comparison is lower than zero (like in Z80 and 80x86 CPUs, but unlike in 65XX and ARM CPUs).
    When a rotate/shift operation shifts out a "1" bit.
    */
}

impl Z80{
    pub fn new() -> Self{
        Self{
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,

            bus: Bus::new(),
            cyclesLeft: 0,
            fetched: 0,
            fetchedSigned: 0,
            currentOpcode: 0,

            prefixedInstruction: false,
            cbFlag: false,
            branchTaken: false,
            justBooted: true,
            halted: false,
            masterInterrupt: false,
        }
    }


    fn getAF(&self) -> u16{
        ((self.a as u16)<<8) | (self.f as u16)
    }

    fn getBC(&self) -> u16{
        ((self.b as u16)<<8) | (self.c as u16)
    }

    fn getDE(&self) -> u16{
        ((self.d as u16)<<8) | (self.e as u16)
    }

    fn getHL(&self) -> u16{
        ((self.h as u16)<<8) | (self.l as u16)
    }


    fn setAF(&mut self, data: u16) {
        self.a = (data >> 8) as u8;
        self.f = data as u8;
    }

    fn setBC(&mut self, data: u16) {
        self.b = (data >> 8) as u8;
        self.c = data as u8;
    }

    fn setDE(&mut self, data: u16) {
        self.d = (data >> 8) as u8;
        self.e = data as u8;
    }

    fn setHL(&mut self, data: u16) {
        self.h = (data >> 8) as u8;
        self.l = data as u8;
    }

    pub fn getFlag(&self, fl: Flags) -> bool {
        bit::get(self.f, fl as usize)
    }

    fn setFlag(&mut self, v: bool, fl: Flags) {
        if v {
            bit::set(self.f, fl as usize);
        } else {
            bit::clr(self.f, fl as usize);
        }
    }

    fn setZeroFlag(&mut self, d: u8) {
        self.setFlag(d == 0, Flags::Zero);
    }

    
}

impl Z80 {
    pub fn readByte(&self, addr: u16) -> u8 {
        self.bus.cpuRead(addr)
    }
    pub fn readBytes(&self, addr: u16) -> u16 {
        u16::from_le_bytes([
            self.bus.cpuRead(addr),
            self.bus.cpuRead(addr + 1)
        ])
    }
    pub fn writeByte(&mut self, addr: u16, data: u8) {
        self.bus.cpuWrite(addr, data);
    }
    fn writeBytes(&mut self, addr: u16, data: u16) {
        let dHI = (data >> 8) as u8;
        let dLO = data as u8;
        self.bus.cpuWrite(addr, dLO);
        self.bus.cpuWrite(addr, dHI);
    }

    fn ADD(&mut self, op1: u8, op2: u8) -> u8{
        let (result, carry) = op1.overflowing_add(op2);
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag((op1 & 0xf) + (op2 & 0xf) > 0xf, Flags::HCarry);
        self.setFlag(carry, Flags::Carry);
        result
    }

    fn ADC(&mut self, op1: u8, op2: u8) -> u8 {
        let (result1, carry1) = op1.overflowing_add(op2);
        let (result2, carry2) = result1.overflowing_add(self.getFlag(Flags::Carry) as u8);
        self.setZeroFlag(result2);
        self.setFlag(false, Flags::Sub);
        self.setFlag((op1 & 0xf) + (op2 & 0xf) + (self.getFlag(Flags::Carry) as u8) > 0xf, Flags::HCarry);
        self.setFlag(carry1 | carry2, Flags::Carry);
        result2
    }

    fn SUB(&mut self, op1: u8, op2: u8) -> u8 {
        let (result, carry) = op1.overflowing_sub(op2);
        self.setZeroFlag(result);
        self.setFlag(true, Flags::Sub);
        self.setFlag(((op1 & 0xf) as i8 - (op2 & 0xf) as i8) < 0x0, Flags::HCarry);
        self.setFlag(carry, Flags::Carry);
        result
    }

    fn SBC(&mut self, op1: u8, op2: u8) -> u8 {
        let (result1, carry1) = op1.overflowing_sub(op2);
        let (result2, carry2) = result1.overflowing_sub(self.getFlag(Flags::Carry) as u8);
        self.setZeroFlag(result2);
        self.setFlag(true, Flags::Sub);
        self.setFlag((op1 & 0xf) as i8 - (op2 & 0xf) as i8 - (self.getFlag(Flags::Carry) as i8) > 0xf, Flags::HCarry);
        self.setFlag(carry1 | carry2, Flags::Carry);
        result2
    }

    fn AND(&mut self, op1: u8, op2: u8) -> u8 {
        let r = op1 & op2;
        self.setZeroFlag(r);
        self.setFlag(false,Flags::Sub);
        self.setFlag(true,Flags::HCarry);
        self.setFlag(false,Flags::Carry);
        r
    }

    fn XOR(&mut self, op1: u8, op2: u8) -> u8 {
        let r = op1 ^ op2;
        self.setZeroFlag(r);
        self.setFlag(false,Flags::Sub);
        self.setFlag(false,Flags::HCarry);
        self.setFlag(false,Flags::Carry);
        r
    }

    fn OR(&mut self, op1: u8, op2: u8) -> u8 {
        let r = op1 | op2;
        self.setZeroFlag(r);
        self.setFlag(false,Flags::Sub);
        self.setFlag(false,Flags::HCarry);
        self.setFlag(false,Flags::Carry);
        r
    }

    fn POP8(&mut self) -> u8 {
        let d: u8 = self.readByte(self.sp);
        self.sp += 1;
        d
    }

    fn PUSH8(&mut self, d: u8) {
        self.sp -= 1;
        self.writeByte(self.sp, d);
    }

    // mozda problemi oko sajkla ali sumnjam
    fn RET_CONDITIAL(&mut self, condition: bool) {
        match self.cyclesLeft {
            5 => {},
            4 => {if !condition {self.cyclesLeft = 1} else {self.branchTaken = true}},
            3 => {},
            2 => {},
            1 => { self.pc = (self.POP8() as u16) << 8; self.pc |= self.POP8() as u16},
            _ => {panic!("cycles left incorrect")}
        }
    }

    fn RETI(&mut self) {
        match self.cyclesLeft {
            4 => {},
            3 => {},
            2 => {self.pc = (self.POP8() as u16) << 8; self.pc |= self.POP8() as u16},
            1 => {self.masterInterrupt = true;},
            _ => {panic!("cycles left incorrect")}
        }
    }

    fn RET(&mut self) {
        match self.cyclesLeft {
            4 => {},
            3 => {},
            2 => {},
            1 => {self.pc = (self.POP8() as u16) << 8; self.pc |= self.POP8() as u16},
            _ => {panic!("cycles left incorrect")}
        }
    }

    fn JP_CONDITIAL(&mut self, condition: bool, addr: u16) {
        match self.cyclesLeft {
            4 => {},
            3 => {},
            2 => {if !condition {self.cyclesLeft = 1} else {self.branchTaken = true}},
            1 => {self.pc = addr;},
            _ => {panic!("cycles left incorrect")}
        }
    }

    fn CALL_CONDITIONAL(&mut self, condition: bool, addr: u16) {
        match self.cyclesLeft {
            6 => {},
            5 => {},
            4 => {if !condition {self.cyclesLeft = 1} else {self.branchTaken = true}},
            3 => {self.PUSH8((self.pc + 3) as u8)},
            2 => {self.PUSH8(((self.pc + 3)>> 8) as u8)},
            1 => {self.pc = addr;},
            _ => {panic!("cycles left incorrect")}
        }
    }

    // nisam siguran
    fn RST(&mut self, offset: u8) {
        match self.cyclesLeft {
            4 => {},
            3 => {self.PUSH8((self.pc) as u8)},
            2 => {self.PUSH8(((self.pc)>> 8) as u8)},
            1 => {self.pc = 0x0000 + offset as u16},
            _ => {panic!("cycles left incorrect")}
        }
    }

    fn JR_CONDITIONAL(&mut self, condition: bool) {
        match self.cyclesLeft {
            3 => {},
            2 => {if !condition {self.cyclesLeft = 1} else {
                self.branchTaken = true;
                self.fetchedSigned = self.readByte(self.pc + 1) as i8;
            }},
            1 => {self.pc = self.pc.wrapping_add(self.fetchedSigned as u16)},
            _ => {panic!("cycles left incorrect")}
        }
    }

    fn unprefixedOpcodes(&mut self, opcode: u8){
        match opcode {
            0x00 => { // NOP
                
            },
            0x01 => { // LD BC,u16
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.c = self.readByte(self.pc + 1)},
                    1 => {self.b = self.readByte(self.pc + 2)},
                    _ => {panic!("cycles left incorrect")}
                }
                
            },
            0x02 => { // LD (BC),A 
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getBC(), self.a)}
                    _ => {panic!("cycles left incorrect")}
                }
                
            },
            0x03 => { // INC BC
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.setBC(self.getBC().wrapping_add(1))},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x04 => { // INC B
                self.setFlag((self.b & 0xf) + 1 > 0xf, Flags::HCarry);
                self.b = self.b.wrapping_add(1);
                self.setZeroFlag(self.b);
                self.setFlag(false, Flags::Sub);
            },
            0x05 => { // DEC B
                self.setFlag((self.b & 0xf) as i8 - 1 < 0, Flags::HCarry);
                self.b = self.b.wrapping_sub(1);
                self.setZeroFlag(self.b);
                self.setFlag(true, Flags::Sub);
            },
            0x06 => { // LD B,u8
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.b = self.readByte(self.pc + 1)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x07 => { // RLCA
                self.setFlag(bit::get(self.a, 7), Flags::Carry);
                self.a = self.a.rotate_left(1);
                self.setFlag(false, Flags::Zero);
                self.setFlag(false, Flags::Sub);
                self.setFlag(false, Flags::HCarry);
                
            },
            0x08 => { // LD (u16),SP *OBAVEZNO TESTIRATI*
                match self.cyclesLeft {
                    5 => {},
                    4 => {self.fetched = self.readByte(self.pc + 1)},
                    3 => {self.fetched = self.readByte((self.fetched as u16) | ((self.readByte(self.pc + 2) as u16) << 8))},
                    2 => {self.sp = self.fetched as u16},
                    1 => {self.sp |= (self.fetched as u16) << 8},
                    _ => panic!("cycles left incorrect")
                }
            },
            0x09 => { // ADD HL,BC *nisam siguran*
                match self.cyclesLeft {
                    2 => {},
                    1 => {
                        let (result, carry) = self.getHL().overflowing_add(self.getBC());
                        self.setFlag((self.getHL() & 0xf) + (self.getBC() & 0xf) > 0xf, Flags::HCarry);
                        self.writeBytes(self.getHL(), result);
                        self.setFlag(carry, Flags::Carry);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => panic!("cycles left incorrect")
                }
            },
            0x0A => { // LD A,(BC)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.a = self.readByte(self.getBC())}
                    _ => {panic!("cycles left incorrect")}
                }
                
            },
            0x0B => { // DEC BC
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.setBC(self.getBC().wrapping_sub(1))},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x0C => { // INC C
                self.setFlag((self.c & 0xf) + 1 > 0xf, Flags::HCarry);
                self.c = self.c.wrapping_add(1);
                self.setZeroFlag(self.b);
                self.setFlag(false, Flags::Sub);
            },
            0x0D => { // DEC C
                self.setFlag((self.c & 0xf) as i8 - 1 < 0, Flags::HCarry);
                self.c = self.c.wrapping_sub(1);
                self.setZeroFlag(self.b);
                self.setFlag(true, Flags::Sub);
            },
            0x0E => { // LD C,u8
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.c = self.readByte(self.pc + 1)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x0F => { // RRCA
                self.setFlag(bit::get(self.a, 0), Flags::Carry);
                self.a = self.a.rotate_right(1);
                self.setFlag(false, Flags::Zero);
                self.setFlag(false, Flags::Sub);
                self.setFlag(false, Flags::HCarry);
            }

            0x11 => { // LD DE,u16
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.e = self.readByte(self.pc + 1)},
                    1 => {self.d = self.readByte(self.pc + 2)},
                    _ => {panic!("cycles left incorrect")}
                }
                
            },
            0x12 => { // LD (DE),A 
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getDE(), self.a)}
                    _ => {panic!("cycles left incorrect")}
                }
                
            },
            0x13 => { // INC DE
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.setDE(self.getDE().wrapping_add(1))},
                    _ => {panic!("cycles left incorrect")},
                }
                
            },
            0x14 => { // INC D
                self.setFlag((self.d & 0xf) + 1 > 0xf, Flags::HCarry);
                self.d = self.b.wrapping_add(1);
                self.setZeroFlag(self.d);
                self.setFlag(false, Flags::Sub);
                
            },
            0x15 => { // DEC D
                self.setFlag((self.d & 0xf) as i8 - 1 < 0, Flags::HCarry);
                self.d = self.d.wrapping_sub(1);
                self.setZeroFlag(self.d);
                self.setFlag(true, Flags::Sub);
                
            },
            0x16 => { // LD D,u8
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.d = self.readByte(self.pc + 1)},
                    _ => {panic!("cycles left incorrect")},
                }
                
            },
            0x17 => { // RLA
                let nCarry = bit::get(self.a, 7);
                if self.getFlag(Flags::Carry) {
                    self.a = bit::set(self.a, 0);
                } else {
                    self.a = bit::clr(self.a, 0);
                }
                self.setFlag(nCarry, Flags::Carry);
                self.setFlag(false, Flags::Zero);
                self.setFlag(false, Flags::Sub);
                self.setFlag(false, Flags::HCarry);
            },
            0x18 => { // JR i8 
                self.JR_CONDITIONAL(true);
            },
            0x19 => { // ADD HL,DE *nisam siguran*
                match self.cyclesLeft {
                    2 => {},
                    1 => {
                        let (result, carry) = self.getHL().overflowing_add(self.getDE());
                        self.setFlag((self.getHL() & 0xf) + (self.getDE() & 0xf) > 0xf, Flags::HCarry);
                        self.writeBytes(self.getHL(), result);
                        self.setFlag(carry, Flags::Carry);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => panic!("cycles left incorrect")
                }
            },
            0x1A => { // LD A,(DE)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.a = self.readByte(self.getDE())}
                    _ => {panic!("cycles left incorrect")}
                }
                
            },
            0x1B => { // DEC DE
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.setDE(self.getDE().wrapping_sub(1))},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x1C => { // INC E
                self.setFlag((self.e & 0xf) + 1 > 0xf, Flags::HCarry);
                self.e = self.e.wrapping_add(1);
                self.setZeroFlag(self.b);
                self.setFlag(false, Flags::Sub);
            },
            0x1D => { // DEC E
                self.setFlag((self.e & 0xf) as i8 - 1 < 0, Flags::HCarry);
                self.e = self.e.wrapping_sub(1);
                self.setZeroFlag(self.b);
                self.setFlag(true, Flags::Sub);
            },
            0x1E => { // LD E,u8
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.e = self.readByte(self.pc + 1)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x1F => { // RRA
                let nCarry = bit::get(self.a, 0);
                if self.getFlag(Flags::Carry) {
                    self.a = bit::set(self.a, 7);
                } else {
                    self.a = bit::clr(self.a, 7);
                }
                self.setFlag(nCarry, Flags::Carry);
                self.setFlag(false, Flags::Zero);
                self.setFlag(false, Flags::Sub);
                self.setFlag(false, Flags::HCarry);
            },

            0x20 => { // JR NZ,i8 
                self.JR_CONDITIONAL(!self.getFlag(Flags::Zero))
            },
            0x21 => { // LD HL,u16
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.l = self.readByte(self.pc + 1)},
                    1 => {self.h = self.readByte(self.pc + 2)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x22 => { // LD (HL++),A 
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.a); self.setHL(self.getHL().wrapping_add(1))}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x23 => { // INC HL
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.setHL(self.getHL().wrapping_add(1))},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x24 => { // INC H
                self.setFlag((self.h & 0xf) + 1 > 0xf, Flags::HCarry);
                self.h = self.b.wrapping_add(1);
                self.setZeroFlag(self.h);
                self.setFlag(false, Flags::Sub);
            },
            0x25 => { // DEC H
                self.setFlag((self.h & 0xf) as i8 - 1 < 0, Flags::HCarry);
                self.h = self.h.wrapping_sub(1);
                self.setZeroFlag(self.h);
                self.setFlag(true, Flags::Sub);
            },
            0x26 => { // LD H,u8
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.h = self.readByte(self.pc + 1)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x27 => { // DAA NOT IMPLEMENTED
                todo!("Implement DAA");
            },
            0x28 => { // JR Z,i8 
                self.JR_CONDITIONAL(self.getFlag(Flags::Zero))
            },
            0x29 => { // ADD HL,HL *nisam siguran*
                match self.cyclesLeft {
                    2 => {},
                    1 => {
                        let (result, carry) = self.getHL().overflowing_add(self.getHL());
                        self.setFlag((self.getHL() & 0xf) + (self.getHL() & 0xf) > 0xf, Flags::HCarry);
                        self.writeBytes(self.getHL(), result);
                        self.setFlag(carry, Flags::Carry);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => panic!("cycles left incorrect")
                }
            },
            0x2A => { // LD A,(HL++)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.a = self.readByte(self.getBC()); self.setHL(self.getHL().wrapping_add(1))}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x2B => { // DEC HL
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.setHL(self.getHL().wrapping_sub(1))},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x2C => { // INC L
                self.setFlag((self.l & 0xf) + 1 > 0xf, Flags::HCarry);
                self.l = self.l.wrapping_add(1);
                self.setZeroFlag(self.b);
                self.setFlag(false, Flags::Sub);
            },
            0x2D => { // DEC L
                self.setFlag((self.l & 0xf) as i8 - 1 < 0, Flags::HCarry);
                self.l = self.l.wrapping_sub(1);
                self.setZeroFlag(self.b);
                self.setFlag(true, Flags::Sub);
            },
            0x2E => { // LD L,u8
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.l = self.readByte(self.pc + 1)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x2F => { // CPL
                self.a = !self.a;
                self.setFlag(true, Flags::Sub);
                self.setFlag(true, Flags::HCarry);
            },

            0x30 => { // JR NC,i8 
                self.JR_CONDITIONAL(!self.getFlag(Flags::Carry))
            },
            0x31 => { // LD SP,u16
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.sp = self.readByte(self.pc + 1) as u16},
                    1 => {self.sp |= (self.readByte(self.pc + 2) as u16) << 8},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x32 => { // LD (HL--),A 
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.a); self.setHL(self.getHL().wrapping_sub(1))}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x33 => { // INC SP
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.sp = self.sp.wrapping_add(1)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x34 => { // INC (HL)
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.fetched = self.readByte(self.getHL())},
                    1 => {
                        self.setFlag((self.fetched & 0xf) + 1 > 0xf, Flags::HCarry);

                        self.fetched = self.fetched.wrapping_add(1);
                        self.writeByte(self.getHL(), self.fetched);

                        self.setZeroFlag(self.fetched);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x35 => { // DEC (HL)
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.fetched = self.readByte(self.getHL())},
                    1 => {
                        self.setFlag((self.fetched & 0xf) as i8 - 1 < 0x0, Flags::HCarry);

                        self.fetched = self.fetched.wrapping_sub(1);
                        self.writeByte(self.getHL(), self.fetched);

                        self.setZeroFlag(self.fetched);
                        self.setFlag(true, Flags::Sub);
                    },
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x36 => { // LD (HL),u8
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.fetched = self.readByte(self.pc+1)},
                    1 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x37 => { // SCF
                self.setFlag(true, Flags::Carry);
                self.setFlag(false, Flags::Sub);
                self.setFlag(false, Flags::HCarry);
            },
            0x38 => { // JR C,i8 
                self.JR_CONDITIONAL(self.getFlag(Flags::Carry))
            },
            0x39 => { // ADD HL,DE *nisam siguran*
                match self.cyclesLeft {
                    2 => {},
                    1 => {
                        let (result, carry) = self.getHL().overflowing_add(self.sp);
                        self.setFlag((self.getHL() & 0xf) + (self.sp & 0xf) > 0xf, Flags::HCarry);
                        self.writeBytes(self.getHL(), result);
                        self.setFlag(carry, Flags::Carry);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => panic!("cycles left incorrect")
                }
            },
            0x3A => { // LD A,(HL--)
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.a = self.readByte(self.getBC()); self.setHL(self.getHL().wrapping_sub(1))},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x3B => { // DEC SP
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.sp = self.sp.wrapping_sub(1)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x3C => { // INC A
                self.setFlag((self.a & 0xf) + 1 > 0xf, Flags::HCarry);
                self.a = self.a.wrapping_add(1);
                self.setZeroFlag(self.b);
                self.setFlag(false, Flags::Sub);
            },
            0x3D => { // DEC A
                self.setFlag((self.a & 0xf) as i8 - 1 < 0, Flags::HCarry);
                self.a = self.a.wrapping_sub(1);
                self.setZeroFlag(self.b);
                self.setFlag(true, Flags::Sub);
            },
            0x3E => { // LD A,u8
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.a = self.readByte(self.pc + 1)},
                    _ => {panic!("cycles left incorrect")},
                }
            },
            0x3F => { // CCF
                self.setFlag(!self.getFlag(Flags::Carry), Flags::Carry);
                self.setFlag(false, Flags::Sub);
                self.setFlag(false, Flags::HCarry);
            }

            0x40 => { // LD B,B
                self.b = self.b;
            },
            0x41 => { // LD B,C
                self.b = self.c;
            },
            0x42 => { // LD B,D
                self.b = self.d;
            },
            0x43 => { // LD B,E
                self.b = self.e;
            },
            0x44 => { // LD B,H
                self.b = self.h;
            },
            0x45 => { // LD B,L
                self.b = self.l;
            },
            0x46 => { // LD B,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.b = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x47 => { // LD B,A
                self.b = self.a;
            },
            0x48 => { // LD C,B
                self.c = self.b;
            },
            0x49 => { // LD C,C
                self.c = self.c;
            },
            0x4A => { // LD C,D
                self.c = self.d;
            },
            0x4B => { // LD C,E
                self.c = self.e;
            },
            0x4C => { // LD C,H
                self.c = self.h;
            },
            0x4D => { // LD C,L
                self.c = self.l;
            },
            0x4E => { // LD C,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.c = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x4F => { // LD C,A
                self.c = self.a;
            },


            0x50 => { // LD D,B
                self.d = self.b;
            },
            0x51 => { // LD D,C
                self.d = self.c;
            },
            0x52 => { // LD D,D
                self.d = self.d;
            },
            0x53 => { // LD D,E
                self.d = self.e;
            },
            0x54 => { // LD D,H
                self.d = self.h;
            },
            0x55 => { // LD D,L
                self.d = self.l;
            },
            0x56 => { // LD D,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.d = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x57 => { // LD D,A
                self.d = self.a;
            },
            0x58 => { // LD E,B
                self.e = self.b;
            },
            0x59 => { // LD E,C
                self.e = self.c;
            },
            0x5A => { // LD E,D
                self.e = self.d;
            },
            0x5B => { // LD E,E
                self.e = self.e;
            },
            0x5C => { // LD E,H
                self.e = self.h;
            },
            0x5D => { // LD E,L
                self.e = self.l;
            },
            0x5E => { // LD E,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.e = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x5F => { // LD E,A
                self.e = self.a;
            },

            0x60 => { // LD H,B
                self.h = self.b;
            },
            0x61 => { // LD H,C
                self.h = self.c;
            },
            0x62 => { // LD H,D
                self.h = self.d;
            },
            0x63 => { // LD H,E
                self.h = self.e;
            },
            0x64 => { // LD H,H
                self.h = self.h;
            },
            0x65 => { // LD H,L
                self.h = self.l;
            },
            0x66 => { // LD H,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.h = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x67 => { // LD H,A
                self.h = self.a;
            },
            0x68 => { // LD L,B
                self.l = self.b;
            },
            0x69 => { // LD L,C
                self.l = self.c;
            },
            0x6A => { // LD L,D
                self.l = self.d;
            },
            0x6B => { // LD L,E
                self.l = self.e;
            },
            0x6C => { // LD L,H
                self.l = self.h;
            },
            0x6D => { // LD L,L
                self.l = self.l;
            },
            0x6E => { // LD L,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.l = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x6F => { // LD L,A
                self.l = self.a;
            },

            0x70 => { // LD (HL),B
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.b);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x71 => { // LD (HL),C
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.c);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x72 => { // LD (HL),D
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.d);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x73 => { // LD (HL),E
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.e);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x74 => { // LD (HL),H
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.h);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x75 => { // LD (HL),L
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.l);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x76 => { // HALT
                self.halted = true;
            }
            0x77 => { // LD (HL),A
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.a);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x78 => { // LD A,B
                self.a = self.b;
            },
            0x79 => { // LD A,C
                self.a = self.c;
            },
            0x7A => { // LD A,D
                self.a = self.d;
            },
            0x7B => { // LD A,E
                self.a = self.e;
            },
            0x7C => { // LD A,H
                self.a = self.h;
            },
            0x7D => { // LD A,L
                self.a = self.l;
            },
            0x7E => { // LD A,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.a = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x7F => { // LD A,A
                self.a = self.a;
            },

            0x80 => { // ADD A,B
                self.a = self.ADD(self.a, self.b);
            },
            0x81 => { // ADD A,C
                self.a = self.ADD(self.a, self.c);
            },
            0x82 => { // ADD A,D
                self.a = self.ADD(self.a, self.d);
            },
            0x83 => { // ADD A,E
                self.a = self.ADD(self.a, self.e);
            },
            0x84 => { // ADD A,H
                self.a = self.ADD(self.a, self.h);
            },
            0x85 => { // ADD A,L
                self.a = self.ADD(self.a, self.l);
            },
            0x86 => { // ADD A,(HL) *mozda popraviti*
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.getHL())},
                    1 => {
                        self.a = self.ADD(self.a, self.fetched);
                    },
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x87 => { // ADD A,A
                self.a = self.ADD(self.a, self.a);
            },
            0x88 => { // ADC A,B
                self.a = self.ADC(self.a, self.b);
            },
            0x89 => { // ADC A,C
                self.a = self.ADC(self.a, self.c);
            },
            0x8A => { // ADC A,D
                self.a = self.ADC(self.a, self.d);
            },
            0x8B => { // ADC A,E
                self.a = self.ADC(self.a, self.e);
            },
            0x8C => { // ADC A,H
                self.a = self.ADC(self.a, self.h);
            },
            0x8D => { // ADC A,L
                self.a = self.ADC(self.a, self.l);
            },
            0x8E => { // ADC A,(HL) *mozda popraviti*
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.getHL())},
                    1 => {
                        self.a = self.ADC(self.a, self.fetched);
                    },
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x8F => { // ADC A,A
                self.a = self.ADC(self.a, self.a);
            },
            
            0x90 => { // SUB A,B
                self.a = self.SUB(self.a, self.b);
            },
            0x91 => { // SUB A,C
                self.a = self.SUB(self.a, self.c);
            },
            0x92 => { // SUB A,D
                self.a = self.SUB(self.a, self.d);
            },
            0x93 => { // SUB A,E
                self.a = self.SUB(self.a, self.e);
            },
            0x94 => { // SUB A,H
                self.a = self.SUB(self.a, self.h);
            },
            0x95 => { // SUB A,L
                self.a = self.SUB(self.a, self.l);
            },
            0x96 => { // SUB A,(HL) *mozda popraviti*
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.getHL())},
                    1 => {
                        self.a = self.SUB(self.a, self.fetched);
                    },
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x97 => { // SUB A,A
                self.a = self.SUB(self.a, self.a);
            },
            0x98 => { // SBC A,B
                self.a = self.SBC(self.a, self.b);
            },
            0x99 => { // SBC A,C
                self.a = self.SBC(self.a, self.c);
            },
            0x9A => { // SBC A,D
                self.a = self.SBC(self.a, self.d);
            },
            0x9B => { // SBC A,E
                self.a = self.SBC(self.a, self.e);
            },
            0x9C => { // SBC A,H
                self.a = self.SBC(self.a, self.h);
            },
            0x9D => { // SBC A,L
                self.a = self.SBC(self.a, self.l);
            },
            0x9E => { // SBC A,(HL) *mozda popraviti*
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.getHL())},
                    1 => {
                        self.a = self.SBC(self.a, self.fetched);
                    },
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0x9F => { // SBC A,A
                self.a = self.SBC(self.a, self.a);
            },
        

            0xA0 => { // AND A,B
                self.a = self.AND(self.a, self.b);
            },
            0xA1 => { // AND A,C
                self.a = self.AND(self.a, self.c);
            },
            0xA2 => { // AND A,D
                self.a = self.AND(self.a, self.d);
            },
            0xA3 => { // AND A,E
                self.a = self.AND(self.a, self.e);
            },
            0xA4 => { // AND A,H
                self.a = self.AND(self.a, self.h);
            },
            0xA5 => { // AND A,L
                self.a = self.AND(self.a, self.l);
            },
            0xA6 => { // AND A,(HL)
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.getHL())}
                    1 => {self.a = self.AND(self.a, self.fetched);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xA7 => { // AND A,A
                self.a = self.AND(self.a, self.a);
            },
            0xA8 => { // XOR A,B
                self.a = self.XOR(self.a, self.b);
            },
            0xA9 => { // XOR A,C
                self.a = self.XOR(self.a, self.c);
            },
            0xAA => { // XOR A,D
                self.a = self.XOR(self.a, self.d);
            },
            0xAB => { // XOR A,E
                self.a = self.XOR(self.a, self.e);
            },
            0xAC => { // XOR A,H
                self.a = self.XOR(self.a, self.h);
            },
            0xAD => { // XOR A,L
                self.a = self.XOR(self.a, self.l);
            },
            0xAE => { // XOR A,(HL)
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.getHL())},
                    1 => {self.a = self.XOR(self.a, self.fetched);},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xAF => { // XOR A,A
                self.a = self.XOR(self.a, self.a);
            },

            0xB0 => { // OR A,B
                self.a = self.OR(self.a, self.b);
            },
            0xB1 => { // OR A,C
                self.a = self.OR(self.a, self.c);
            },
            0xB2 => { // OR A,D
                self.a = self.OR(self.a, self.d);
            },
            0xB3 => { // OR A,E
                self.a = self.OR(self.a, self.e);
            },
            0xB4 => { // OR A,H
                self.a = self.OR(self.a, self.h);
            },
            0xB5 => { // OR A,L
                self.a = self.OR(self.a, self.l);
            },
            0xB6 => { // OR A,(HL)
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.getHL())}
                    1 => {self.a = self.OR(self.a, self.fetched);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xB7 => { // OR A,A
                self.a = self.OR(self.a, self.a);
            },
            0xB8 => { // CP A,B
                self.SUB(self.a, self.b);
            },
            0xB9 => { // CP A,C
                self.SUB(self.a, self.c);
            },
            0xBA => { // CP A,D
                 self.SUB(self.a, self.d);
            },
            0xBB => { // CP A,E
                self.SUB(self.a, self.e);
            },
            0xBC => { // CP A,H
                self.SUB(self.a, self.h);
            },
            0xBD => { // CP A,L
                self.SUB(self.a, self.l);
            },
            0xBE => { // CP A,(HL)
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.getHL())}
                    1 => {self.SUB(self.a, self.fetched);}
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xBF => { // CP A,A
                self.SUB(self.a, self.a);
            },

            0xC0 => { // RET NZ
                self.RET_CONDITIAL(!self.getFlag(Flags::Zero));
            },
            0xC1 => { // POP BC
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.c = self.POP8()},
                    1 => {self.b = self.POP8()},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xC2 => { // JP NZ,u16
                self.JP_CONDITIAL(!self.getFlag(Flags::Zero), self.readBytes(self.pc + 1));
            },
            0xC3 => { // JP u16
                self.JP_CONDITIAL(true, self.readBytes(self.pc + 1));
            },
            0xC4 => { // CALL NZ,u16
                self.CALL_CONDITIONAL(!self.getFlag(Flags::Zero), self.readBytes(self.pc + 1))
            },
            0xC5 => { // PUSH BC
                match self.cyclesLeft {
                    4 => {},
                    3 => {},
                    2 => {self.PUSH8(self.b)},
                    1 => {self.PUSH8(self.c)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xC6 => { // ADD A,u8
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.a = self.ADD(self.a, self.fetched)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xC7 => { // RST 0x00
                self.RST(0x00);
            },
            0xC8 => { // RET Z
                self.RET_CONDITIAL(self.getFlag(Flags::Zero));
            },
            0xC9 => { // RET
                self.RET();
            },
            0xCA => { // JP Z,u16
                self.JP_CONDITIAL(self.getFlag(Flags::Zero), self.readBytes(self.pc + 1));
            }
            0xCB => { // CB Prefix
                self.cbFlag = true
            },
            0xCC => { // CALL Z,u16
                self.CALL_CONDITIONAL(self.getFlag(Flags::Zero), self.readBytes(self.pc + 1))
            },
            0xCD => { // CALL,u16
                self.CALL_CONDITIONAL(true, self.readBytes(self.pc + 1));
            },
            0xCE => { // ADC A,u8
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.a = self.ADC(self.a, self.fetched)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xCF => { // RST 0x08
                self.RST(0x08)
            },

            0xD0 => { // RET NC
                self.RET_CONDITIAL(!self.getFlag(Flags::Carry));
            },
            0xD1 => { // POP DE
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.e = self.POP8()},
                    1 => {self.d = self.POP8()},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xD2 => { // JP NC,u16
                self.JP_CONDITIAL(!self.getFlag(Flags::Carry), self.readBytes(self.pc + 1));
            },
            0xD4 => { // CALL NC,u16
                self.CALL_CONDITIONAL(!self.getFlag(Flags::Carry), self.readBytes(self.pc + 1))
            },
            0xD5 => { // PUSH DE
                match self.cyclesLeft {
                    4 => {},
                    3 => {},
                    2 => {self.PUSH8(self.d)},
                    1 => {self.PUSH8(self.e)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xD6 => { // SUB A,u8
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.a = self.SUB(self.a, self.fetched)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xD7 => { // RST 0x10
                self.RST(0x10);
            },
            0xD8 => { // RET C
                self.RET_CONDITIAL(self.getFlag(Flags::Carry));
            },
            0xD9 => { // RETI
                self.RETI();
            },
            0xDA => { // JP C,u16
                self.JP_CONDITIAL(self.getFlag(Flags::Carry), self.readBytes(self.pc + 1));
            },
            0xDB => { // CALL C,u16
                self.CALL_CONDITIONAL(self.getFlag(Flags::Carry), self.readBytes(self.pc + 1))
            },
            0xDE => { // SBC A,u8
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.a = self.SBC(self.a, self.fetched)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xDF => { // RST 0x18
                self.RST(0x18)
            },

            0xE0 => { // LD (0xFF00+u8),A
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.writeByte(0xFF00 + self.fetched as u16, self.a)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xE1 => { // POP HL
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.l = self.POP8()},
                    1 => {self.h = self.POP8()},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xE2 => { // LD (0xFF00+C),A
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.writeByte(0xFF00 + self.c as u16, self.a)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xE5 => { // PUSH HL
                match self.cyclesLeft {
                    4 => {},
                    3 => {},
                    2 => {self.PUSH8(self.h)},
                    1 => {self.PUSH8(self.l)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xE6 => { // AND A,u8
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.a = self.AND(self.a, self.fetched)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xE7 => { // RST 0x20
                self.RST(0x20);
            },
            0xE8 => { // ADD SP,i8 cycle inaccurate i mozda ne radi
                match self.cyclesLeft {
                    4 => {},
                    3 => {self.fetchedSigned = self.readByte(self.pc + 1) as i8},
                    2 => {},
                    1 => {self.sp = self.sp.wrapping_add(self.fetchedSigned as u16)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xE9 => { // JP HL
                self.pc = self.getHL();
            },
            0xEA => { // LD (u16),A
                match self.cyclesLeft {
                    4 => {},
                    3 => {},
                    2 => {},
                    1 => {self.writeByte(self.readBytes(self.pc + 1), self.a)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xEE => { // XOR A,u8
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.a = self.XOR(self.a, self.fetched)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xEF => { // RST 0x28
                self.RST(0x28)
            },
            

            0xF0 => { // LD A,(0xFF00+u8)
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.a = self.readByte(0xFF00 + self.fetched as u16)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xF1 => { // POP AF
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.f = self.POP8(); self.f = self.f & 0xF0},
                    1 => {self.a = self.POP8()},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xF2 => { // LD A,(0xFF00+C)
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.a = self.readByte(0xFF00 + self.c as u16)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xF3 => { // DI 
                self.masterInterrupt = false;
            },
            0xF5 => { // PUSH AF
                match self.cyclesLeft {
                    4 => {},
                    3 => {},
                    2 => {self.PUSH8(self.a)},
                    1 => {self.PUSH8(self.f)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xF6 => { // ADD A,u8
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.a = self.OR(self.a, self.fetched)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xF7 => { // RST 0x30
                self.RST(0x30);
            },
            0xF8 => { // LD HL,SP+i8 cycle inaccurate i mozda ne radi
                match self.cyclesLeft {
                    3 => {},
                    2 => {self.fetchedSigned = self.readByte(self.pc + 1) as i8},
                    1 => {    
                        self.sp = self.sp.wrapping_add(self.fetchedSigned as u16); 
                        self.setHL(self.sp)},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xF9 => { // LD SP,HL
                match self.cyclesLeft {
                    2 => {},
                    1 => {self.sp = self.getHL()},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xFA => { // LD A,(u16)
                match self.cyclesLeft {
                    4 => {},
                    3 => {},
                    2 => {},
                    1 => {self.a = self.readByte(self.readBytes(self.pc + 1))},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xFB => { // EI
                self.masterInterrupt = true;
            },
            0xFE => { // CP A,u8
                match self.cyclesLeft {
                    2 => {self.fetched = self.readByte(self.pc + 1)},
                    1 => {self.SUB(self.a, self.fetched);},
                    _ => {panic!("cycles left incorrect")}
                }
            },
            0xFF => { // RST 0x38
                self.RST(0x38)
            },
            _ => panic!("Unknown opcode or not implemented"),
        }
    }

    fn prefixedOpcodes(&mut self, opcode: u8){

        match opcode {
            _ => ()//panic!("Prefixed opcodes not implemented")
        }
        if self.cyclesLeft == 0 {self.prefixedInstruction = false}
    }

    fn executeOneCycle(&mut self, opcode: u8) {
        if self.prefixedInstruction || self.cbFlag {
            self.prefixedOpcodes(opcode);
            self.cbFlag = false;
        } else {
            self.unprefixedOpcodes(opcode);
        }
        
        self.cyclesLeft -= 1;
    }

    fn getInstructionInfo(&self, opcode: u8) -> (&str, u8, u8) {
        if self.prefixedInstruction {
            PREFIXED_INSTRUCTION_TABLE[opcode as usize]
        } else {
            UNPREFIXED_INSTRUCTION_TABLE[opcode as usize]
        }
    }

    pub fn clock(&mut self) {
        if self.halted {
            return;
        }
        if self.justBooted {
            self.currentOpcode = self.readByte(self.pc);
            let (_, _, cycles) = self.getInstructionInfo(self.currentOpcode);
            self.cyclesLeft = cycles;
            self.justBooted = false;
        }
        self.executeOneCycle(self.currentOpcode);

        if self.cyclesLeft == 0 {
            let (_, length, _) = self.getInstructionInfo(self.currentOpcode);
            if !self.branchTaken {
                self.pc = self.pc.wrapping_add(length as u16);
            }
            self.branchTaken = false;
            
            self.currentOpcode = self.readByte(self.pc);
            let (_, _, cycles) = self.getInstructionInfo(self.currentOpcode);
            self.cyclesLeft = cycles;
        }
    }
}

pub const UNPREFIXED_INSTRUCTION_TABLE: [(&str, u8, u8); 256]= [
    ("NOP", 1, 1),              ("LD BC,u16", 3, 3),    ("LD (BC), A", 1, 2),       ("INC BC", 1, 2),       ("INC B", 1, 1),        ("DEC B", 1, 1),        ("LD B,u8", 2, 2),      ("RLCA", 1, 1),         ("LD (u16),SP", 3, 5),  ("ADD HL,BC", 1, 2),    ("LD A,(BC)", 1, 2),    ("DEC BC", 1, 2),   ("INC C", 1, 1),        ("DEC C", 1, 1),    ("LD C,u8", 2, 2),      ("RRCA", 1, 1),
    ("STOP", 2, 1),             ("LD DE,u16", 3, 3),    ("LD (DE), A", 1, 2),       ("INC DE", 1, 2),       ("INC D", 1, 1),        ("DEC D", 1, 1),        ("LD D,u8", 2, 2),      ("RLA", 1, 1),          ("JR i8", 2, 3),        ("ADD HL,DE", 1, 2),    ("LD A,(DE)", 1, 2),    ("DEC DE", 1, 2),   ("INC E", 1, 1),        ("DEC E", 1, 1),    ("LD E,u8", 2, 2),      ("PRA", 1, 1),
    ("", 0, 0),                 ("LD HL,16", 3, 3),     ("LD (HL++), A", 1, 2),     ("INC HL", 1, 2),       ("INC H", 1, 1),        ("DEC H", 1, 1),        ("LD H,u8", 2, 2),      ("DAA", 1, 1),          ("JR Z,i8", 2, 3),      ("ADD HL,HL", 1, 2),    ("LD A,(HL++)", 1, 2),  ("DEC HL", 1, 2),   ("INC L", 1, 1),        ("DEC L", 1, 1),    ("LD L,u8", 2, 2),      ("CPL", 1, 1),
    ("", 0, 0),                 ("LD SP,16", 3, 3),     ("LD (HL--), A", 1, 2),     ("INC SP", 1, 2),       ("INC (HL)", 1, 3),     ("DEC (HL)", 1, 3),     ("LD (HL),u8", 2, 3),   ("SCF", 1, 1),          ("JR C,i8", 2, 3),      ("ADD HL,SP", 1, 2),    ("LD A,(HL--)", 1, 2),  ("DEC SP", 1, 2),   ("INC A", 1, 1),        ("DEC A", 1, 1),    ("LD A,u8", 2, 2),      ("CCF", 1, 1),
    ("LD B,B", 1, 1),           ("LD B,C", 1, 1),       ("LD B,D", 1, 1),           ("LD B,E", 1, 1),       ("LD B,H", 1, 1),       ("LD B,L", 1, 1),       ("LD B,(HL)", 1, 2),    ("LD B,A", 1, 1),       ("LD C,B", 1, 1),       ("LD C,C", 1, 1),       ("LD C,D", 1, 1),       ("LD C,E", 1, 1),   ("LD C,H", 1, 1),       ("LD C,L", 1, 1),   ("LD C,(HL)", 1, 2),    ("LD C,A", 1, 1),
    ("LD D,B", 1, 1),           ("LD D,C", 1, 1),       ("LD D,D", 1, 1),           ("LD D,E", 1, 1),       ("LD D,H", 1, 1),       ("LD D,L", 1, 1),       ("LD D,(HL)", 1, 2),    ("LD D,A", 1, 1),       ("LD E,B", 1, 1),       ("LD E,C", 1, 1),       ("LD E,D", 1, 1),       ("LD E,E", 1, 1),   ("LD E,H", 1, 1),       ("LD E,L", 1, 1),   ("LD E,(HL)", 1, 2),    ("LD E,A", 1, 1),
    ("LD H,B", 1, 1),           ("LD H,C", 1, 1),       ("LD H,D", 1, 1),           ("LD H,E", 1, 1),       ("LD H,H", 1, 1),       ("LD H,L", 1, 1),       ("LD H,(HL)", 1, 2),    ("LD H,A", 1, 1),       ("LD L,B", 1, 1),       ("LD L,C", 1, 1),       ("LD L,D", 1, 1),       ("LD L,E", 1, 1),   ("LD L,H", 1, 1),       ("LD L,L", 1, 1),   ("LD L,(HL)", 1, 2),    ("LD L,A", 1, 1),
    ("LD (HL),B", 1, 2),        ("LD (HL),C", 1, 2),    ("LD (HL),D", 1, 2),        ("LD (HL),E", 1, 2),    ("LD (HL),H", 1, 2),    ("LD (HL),L", 1, 2),    ("HALT", 1, 1),         ("LD (HL),A", 1, 2),    ("LD A,B", 1, 1),       ("LD A,C", 1, 1),       ("LD A,D", 1, 1),       ("LD A,E", 1, 1),   ("LD A,H", 1, 1),       ("LD A,L", 1, 1),   ("LD A,(HL)", 1, 2),    ("LD A,A", 1, 1),
    ("ADD A,B", 1, 1),          ("ADD A,C", 1, 1),      ("ADD A,D", 1, 1),          ("ADD A,E", 1, 1),      ("ADD A,H", 1, 1),      ("ADD A,L", 1, 1),      ("ADD A,(HL)", 1, 2),   ("ADD A,A", 1, 1),      ("ADC A,B", 1, 1),      ("ADC A,C", 1, 1),      ("ADC A,D", 1, 1),      ("ADC A,E", 1, 1),  ("ADC A,H", 1, 1),      ("ADC A,L", 1, 1),  ("ADC A,(HL)", 1, 2),   ("ADC A,A", 1, 1),
    ("SUB A,B", 1, 1),          ("SUB A,C", 1, 1),      ("SUB A,D", 1, 1),          ("SUB A,E", 1, 1),      ("SUB A,H", 1, 1),      ("SUB A,L", 1, 1),      ("SUB A,(HL)", 1, 2),   ("SUB A,A", 1, 1),      ("SBC A,B", 1, 1),      ("SBC A,C", 1, 1),      ("SBC A,D", 1, 1),      ("SBC A,E", 1, 1),  ("SBC A,H", 1, 1),      ("SBC A,L", 1, 1),  ("SBC A,(HL)", 1, 2),   ("SBC A,A", 1, 1),
    ("AND A,B", 1, 1),          ("AND A,C", 1, 1),      ("AND A,D", 1, 1),          ("AND A,E", 1, 1),      ("AND A,H", 1, 1),      ("AND A,L", 1, 1),      ("AND A,(HL)", 1, 2),   ("AND A,A", 1, 1),      ("XOR A,B", 1, 1),      ("XOR A,C", 1, 1),      ("XOR A,D", 1, 1),      ("XOR A,E", 1, 1),  ("XOR A,H", 1, 1),      ("XOR A,L", 1, 1),  ("XOR A,(HL)", 1, 2),   ("XOR A,A", 1, 1),
    ("OR A,B", 1, 1),           ("OR A,C", 1, 1),       ("OR A,D", 1, 1),           ("OR A,E", 1, 1),       ("OR A,H", 1, 1),       ("OR A,L", 1, 1),       ("OR A,(HL)", 1, 2),    ("OR A,A", 1, 1),       ("CP A,B", 1, 1),       ("CP A,C", 1, 1),       ("CP A,D", 1, 1),       ("CP A,E", 1, 1),   ("CP A,H", 1, 1),       ("CP A,L", 1, 1),   ("CP A,(HL)", 1, 2),    ("CP A,A", 1, 1),
    ("RET NZ", 1, 5),           ("POP BC", 1, 3),       ("JP NZ,u16", 3, 4),        ("JP u16", 3, 4),       ("CALL NZ,u16", 3, 6),  ("PUSH BC", 1, 4),      ("ADD A,u8", 2, 2),     ("RST 0x00", 1, 4),     ("RET Z", 1, 5),        ("RET", 1, 4),          ("JP Z,u16", 3, 4),     ("CB", 1, 1),       ("CALL Z,u16", 3, 6),   ("CALL u16", 3, 6), ("ADC A,u8", 2, 2),     ("RST 0x08", 1, 4),
    ("RET NC", 1, 5),           ("POP DE", 1, 3),       ("JP NC,u16", 3, 4),        ("", 0, 0),             ("CALL NC,u16", 3, 6),  ("PUSH DE", 1, 4),      ("SUB A,u8", 2, 2),     ("RST 0x10", 1, 4),     ("RET C", 1, 5),        ("RETI", 1, 4),         ("JP C,u16", 3, 4),     ("", 0, 0),         ("CALL C,u16", 3, 6),   ("", 0, 0),         ("SBC A,u8", 2, 2),     ("RST 0x18", 1, 4),
    ("LD (0xFF00+u8),A", 2, 3), ("POP HL", 1, 3),       ("LD (0xFF00+C),A", 1, 2),  ("", 0, 0),             ("", 0, 0),             ("PUSH HL", 1, 4),      ("AND A,u8", 2, 2),     ("RST 0x20", 1, 4),     ("ADD SP,i8", 2, 4),    ("JP HL", 1, 1),        ("LD (u16),A", 3, 4),   ("", 0, 0),         ("", 0, 0),             ("", 0, 0),         ("XOR A,u8", 2, 2),     ("RST 0x28", 1, 4),
    ("LD A,(0xFF00+u8)", 2, 3), ("POP AF", 1, 3),       ("LD A,(0xFF00+C)", 1, 2),  ("DI", 1, 1),           ("", 0, 0),             ("PUSH AF", 1, 4),      ("OR A,u8", 2, 2),      ("RST 0x30", 1, 4),     ("LD HL,SP+i8", 2, 3),  ("LD SP,HL", 1, 2),     ("LD A,(u16)", 3, 4),   ("EI", 1, 1),       ("", 0, 0),             ("", 0, 0),         ("CP A,u8", 2, 2),      ("RST 0x38", 1, 4),
];


pub const PREFIXED_INSTRUCTION_TABLE: [(&str, u8, u8); 256] = [
    ("RLC B", 1, 1),    ("RLC C", 1, 1),    ("RLC D", 1, 1),    ("RLC E", 1, 1),    ("RLC H", 1, 1),    ("RLC L", 1, 1),       ("RLC (HL)", 1, 3),   ("RLC A", 1, 1),       ("RRC B", 1, 1),    ("RRC C", 1, 1),    ("RRC D", 1, 1),    ("RRC E", 1, 1),    ("RRC H", 1, 1),    ("RRC L", 1, 1),    ("RRC (HL)", 1, 3),    ("RRC A", 1, 1),
    ("RL B", 1, 1),     ("RL C", 1, 1),     ("RL D", 1, 1),     ("RL E", 1, 1),     ("RL H", 1, 1),     ("RL L", 1, 1),        ("RL (HL)", 1, 3),    ("RL A", 1, 1),        ("RR B", 1, 1),     ("RR C", 1, 1),     ("RR D", 1, 1),     ("RR E", 1, 1),     ("RR H", 1, 1),     ("RR L", 1, 1),     ("RR (HL)", 1, 3),     ("RR A", 1, 1),
    ("SLA B", 1, 1),    ("SLA C", 1, 1),    ("SLA D", 1, 1),    ("SLA E", 1, 1),    ("SLA H", 1, 1),    ("SLA L", 1, 1),       ("SLA (HL)", 1, 3),   ("SLA A", 1, 1),       ("SRA B", 1, 1),    ("SRA C", 1, 1),    ("SRA D", 1, 1),    ("SRA E", 1, 1),    ("SRA H", 1, 1),    ("SRA L", 1, 1),    ("SRA (HL)", 1, 3),    ("SRA A", 1, 1),
    ("SWAP B", 1, 1),   ("SWAP C", 1, 1),   ("SWAP D", 1, 1),   ("SWAP E", 1, 1),   ("SWAP H", 1, 1),   ("SWAP L", 1, 1),      ("SWAP (HL)", 1, 3),  ("SWAP A", 1, 1),      ("SRL B", 1, 1),    ("SRL C", 1, 1),    ("SRL D", 1, 1),    ("SRL E", 1, 1),    ("SRL H", 1, 1),    ("SRL L", 1, 1),    ("SRL (HL)", 1, 3),    ("SRL A", 1, 1),
    ("BIT 0,B", 1, 1),  ("BIT 0,C", 1, 1),  ("BIT 0,D", 1, 1),  ("BIT 0,E", 1, 1),  ("BIT 0,H", 1, 1),  ("BIT 0,L", 1, 1),     ("BIT 0,(HL)", 1, 2), ("BIT 0,A", 1, 1),     ("BIT 1,B", 1, 1),  ("BIT 1,C", 1, 1),  ("BIT 1,D", 1, 1),  ("BIT 1,E", 1, 1),  ("BIT 1,H", 1, 1),  ("BIT 1,L", 1, 1),  ("BIT 1,(HL)", 1, 2),  ("BIT 1,A", 1, 1),
    ("BIT 2,B", 1, 1),  ("BIT 2,C", 1, 1),  ("BIT 2,D", 1, 1),  ("BIT 2,E", 1, 1),  ("BIT 2,H", 1, 1),  ("BIT 2,L", 1, 1),     ("BIT 2,(HL)", 1, 2), ("BIT 2,A", 1, 1),     ("BIT 3,B", 1, 1),  ("BIT 3,C", 1, 1),  ("BIT 3,D", 1, 1),  ("BIT 3,E", 1, 1),  ("BIT 3,H", 1, 1),  ("BIT 3,L", 1, 1),  ("BIT 3,(HL)", 1, 2),  ("BIT 3,A", 1, 1),
    ("BIT 4,B", 1, 1),  ("BIT 4,C", 1, 1),  ("BIT 4,D", 1, 1),  ("BIT 4,E", 1, 1),  ("BIT 4,H", 1, 1),  ("BIT 4,L", 1, 1),     ("BIT 4,(HL)", 1, 2), ("BIT 4,A", 1, 1),     ("BIT 5,B", 1, 1),  ("BIT 5,C", 1, 1),  ("BIT 5,D", 1, 1),  ("BIT 5,E", 1, 1),  ("BIT 5,H", 1, 1),  ("BIT 5,L", 1, 1),  ("BIT 5,(HL)", 1, 2),  ("BIT 5,A", 1, 1),
    ("BIT 6,B", 1, 1),  ("BIT 6,C", 1, 1),  ("BIT 6,D", 1, 1),  ("BIT 6,E", 1, 1),  ("BIT 6,H", 1, 1),  ("BIT 6,L", 1, 1),     ("BIT 6,(HL)", 1, 2), ("BIT 6,A", 1, 1),     ("BIT 7,B", 1, 1),  ("BIT 7,C", 1, 1),  ("BIT 7,D", 1, 1),  ("BIT 7,E", 1, 1),  ("BIT 7,H", 1, 1),  ("BIT 7,L", 1, 1),  ("BIT 7,(HL)", 1, 2),  ("BIT 7,A", 1, 1),
    ("RES 0,B", 1, 1),  ("RES 0,C", 1, 1),  ("RES 0,D", 1, 1),  ("RES 0,E", 1, 1),  ("RES 0,H", 1, 1),  ("RES 0,L", 1, 1),     ("RES 0,(HL)", 1, 3), ("RES 0,A", 1, 1),     ("RES 1,B", 1, 1),  ("RES 1,C", 1, 1),  ("RES 1,D", 1, 1),  ("RES 1,E", 1, 1),  ("RES 1,H", 1, 1),  ("RES 1,L", 1, 1),  ("RES 1,(HL)", 1, 3),  ("RES 1,A", 1, 1),
    ("RES 2,B", 1, 1),  ("RES 2,C", 1, 1),  ("RES 2,D", 1, 1),  ("RES 2,E", 1, 1),  ("RES 2,H", 1, 1),  ("RES 2,L", 1, 1),     ("RES 2,(HL)", 1, 3), ("RES 2,A", 1, 1),     ("RES 3,B", 1, 1),  ("RES 3,C", 1, 1),  ("RES 3,D", 1, 1),  ("RES 3,E", 1, 1),  ("RES 3,H", 1, 1),  ("RES 3,L", 1, 1),  ("RES 3,(HL)", 1, 3),  ("RES 3,A", 1, 1),
    ("RES 4,B", 1, 1),  ("RES 4,C", 1, 1),  ("RES 4,D", 1, 1),  ("RES 4,E", 1, 1),  ("RES 4,H", 1, 1),  ("RES 4,L", 1, 1),     ("RES 4,(HL)", 1, 3), ("RES 4,A", 1, 1),     ("RES 5,B", 1, 1),  ("RES 5,C", 1, 1),  ("RES 5,D", 1, 1),  ("RES 5,E", 1, 1),  ("RES 5,H", 1, 1),  ("RES 5,L", 1, 1),  ("RES 5,(HL)", 1, 3),  ("RES 5,A", 1, 1),
    ("RES 6,B", 1, 1),  ("RES 6,C", 1, 1),  ("RES 6,D", 1, 1),  ("RES 6,E", 1, 1),  ("RES 6,H", 1, 1),  ("RES 6,L", 1, 1),     ("RES 6,(HL)", 1, 3), ("RES 6,A", 1, 1),     ("RES 7,B", 1, 1),  ("RES 7,C", 1, 1),  ("RES 7,D", 1, 1),  ("RES 7,E", 1, 1),  ("RES 7,H", 1, 1),  ("RES 7,L", 1, 1),  ("RES 7,(HL)", 1, 3),  ("RES 7,A", 1, 1),
    ("SET 0,B", 1, 1),  ("SET 0,C", 1, 1),  ("SET 0,D", 1, 1),  ("SET 0,E", 1, 1),  ("SET 0,H", 1, 1),  ("SET 0,L", 1, 1),     ("SET 0,(HL)", 1, 3), ("SET 0,A", 1, 1),     ("SET 1,B", 1, 1),  ("SET 1,C", 1, 1),  ("SET 1,D", 1, 1),  ("SET 1,E", 1, 1),  ("SET 1,H", 1, 1),  ("SET 1,L", 1, 1),  ("SET 1,(HL)", 1, 3),  ("SET 1,A", 1, 1),
    ("SET 2,B", 1, 1),  ("SET 2,C", 1, 1),  ("SET 2,D", 1, 1),  ("SET 2,E", 1, 1),  ("SET 2,H", 1, 1),  ("SET 2,L", 1, 1),     ("SET 2,(HL)", 1, 3), ("SET 2,A", 1, 1),     ("SET 3,B", 1, 1),  ("SET 3,C", 1, 1),  ("SET 3,D", 1, 1),  ("SET 3,E", 1, 1),  ("SET 3,H", 1, 1),  ("SET 3,L", 1, 1),  ("SET 3,(HL)", 1, 3),  ("SET 3,A", 1, 1),
    ("SET 4,B", 1, 1),  ("SET 4,C", 1, 1),  ("SET 4,D", 1, 1),  ("SET 4,E", 1, 1),  ("SET 4,H", 1, 1),  ("SET 4,L", 1, 1),     ("SET 4,(HL)", 1, 3), ("SET 4,A", 1, 1),     ("SET 5,B", 1, 1),  ("SET 5,C", 1, 1),  ("SET 5,D", 1, 1),  ("SET 5,E", 1, 1),  ("SET 5,H", 1, 1),  ("SET 5,L", 1, 1),  ("SET 5,(HL)", 1, 3),  ("SET 5,A", 1, 1),
    ("SET 6,B", 1, 1),  ("SET 6,C", 1, 1),  ("SET 6,D", 1, 1),  ("SET 6,E", 1, 1),  ("SET 6,H", 1, 1),  ("SET 6,L", 1, 1),     ("SET 6,(HL)", 1, 3), ("SET 6,A", 1, 1),     ("SET 7,B", 1, 1),  ("SET 7,C", 1, 1),  ("SET 7,D", 1, 1),  ("SET 7,E", 1, 1),  ("SET 7,H", 1, 1),  ("SET 7,L", 1, 1),  ("SET 7,(HL)", 1, 3),  ("SET 7,A", 1, 1),

];

