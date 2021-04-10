use super::bit;
use super::bus::{Bus, IntrFlags};
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

    pub bus: Bus,
    pub cyclesLeft: u8,
    fetched: u8,
    fetchedSigned: i8,
    currentOpcode: u8,

    pub prefixedInstruction: bool,
    cbFlag: bool,
    branchTaken: bool,
    justBooted: bool,
    halted: bool,
    masterInterrupt: bool,
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

    fn INT(&mut self, addr: u16) {
        match self.cyclesLeft {
            20 => {},
            16 => {},
            12 => {self.PUSH8(self.pc as u8)},
            8 => {self.PUSH8((self.pc >> 8) as u8)},
            4 => {self.pc = addr},
            _ => {},
        }
    }
    // mozda problemi oko sajkla ali sumnjam
    fn RET_CONDITIAL(&mut self, condition: bool) {
        match self.cyclesLeft {
            20 => {},
            16 => {if !condition {self.cyclesLeft = 4} else {self.branchTaken = true}},
            12 => {},
            8 => {},
            4 => { self.pc = (self.POP8() as u16) << 8; self.pc |= self.POP8() as u16},
            _ => {}
        }
    }

    fn RETI(&mut self) {
        match self.cyclesLeft {
            16 => {},
            12 => {},
            8 => {self.masterInterrupt = true},
            4 => {self.pc = (self.POP8() as u16) << 8; self.pc |= self.POP8() as u16},
            _ => {}
        }
    }

    fn RET(&mut self) {
        match self.cyclesLeft {
            16 => {},
            12 => {},
            8 => {},
            4 => {self.pc = (self.POP8() as u16) << 8; self.pc |= self.POP8() as u16},
            _ => {}
        }
    }

    fn JP_CONDITIAL(&mut self, condition: bool, addr: u16) {
        match self.cyclesLeft {
            16 => {},
            12 => {},
            8 => {if !condition {self.cyclesLeft = 4} else {self.branchTaken = true}},
            4 => {self.pc = addr;},
            _ => {}
        }
    }

    fn CALL_CONDITIONAL(&mut self, condition: bool, addr: u16) {
        match self.cyclesLeft {
            24 => {},
            20 => {},
            16 => {if !condition {self.cyclesLeft = 4} else {self.branchTaken = true}},
            12 => {self.PUSH8((self.pc + 3) as u8)},
            8 => {self.PUSH8(((self.pc + 3)>> 8) as u8)},
            4 => {self.pc = addr;},
            _ => {}
        }
    }

    // nisam siguran
    fn RST(&mut self, offset: u8) {
        match self.cyclesLeft {
            16 => {},
            12 => {self.PUSH8((self.pc) as u8)},
            8 => {self.PUSH8(((self.pc)>> 8) as u8)},
            4 => {self.pc = 0x0000 + offset as u16},
            _ => {}
        }
    }

    fn JR_CONDITIONAL(&mut self, condition: bool) {
        match self.cyclesLeft {
            12 => {},
            8 => {if !condition {self.cyclesLeft = 4} else {
                self.branchTaken = true;
                self.fetchedSigned = self.readByte(self.pc + 1) as i8;
            }},
            4 => {self.pc = self.pc.wrapping_add(self.fetchedSigned as u16)},
            _ => {}
        }
    }

    fn RLC(&mut self, op1: u8) -> u8{
        self.setFlag(bit::get(op1, 7), Flags::Carry);
        let result = op1.rotate_left(1);
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag(false, Flags::HCarry);
        result
    }

    fn RRC(&mut self, op1: u8) -> u8 {
        self.setFlag(bit::get(op1, 0), Flags::Carry);
        let result = op1.rotate_right(1);
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag(false, Flags::HCarry);
        result
    }

    fn RL(&mut self, op1: u8) -> u8 {
        let nCarry = bit::get(op1, 7);
        let mut result = op1.rotate_left(1);
        if self.getFlag(Flags::Carry) {
            result = bit::set(result, 0);
        } else {
            result = bit::clr(result, 0);
        }
        self.setFlag(nCarry, Flags::Carry);
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag(false, Flags::HCarry);
        result
    }

    fn RR(&mut self, op1: u8) -> u8 {
        let nCarry = bit::get(op1, 0);
        let mut result = op1.rotate_right(1);
        if self.getFlag(Flags::Carry) {
            result = bit::set(result, 7);
        } else {
            result = bit::clr(result, 7);
        }
        self.setFlag(nCarry, Flags::Carry);
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag(false, Flags::HCarry);
        result
    }

    fn SLA(&mut self, op1: u8) -> u8 {
        self.setFlag(bit::get(op1, 7),Flags::Carry);
        let result = op1 << 1;
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag(false, Flags::HCarry);
        result
    }

    fn SRA(&mut self, op1: u8) -> u8 {
        self.setFlag(bit::get(op1, 0),Flags::Carry);
        let oldBit = bit::get(op1, 7);
        let result = op1 >> 1;
        if oldBit {
            bit::set(result, 7);
        } 
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag(false, Flags::HCarry);
        result
    }

    fn SWAP(&mut self, op1: u8) -> u8 {
        let low = op1 & 0x0f;
        let high = op1 & 0xf0;
        let result = (low << 4) | (high >> 4);
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag(false, Flags::HCarry);
        self.setFlag(false, Flags::Carry);
        result
    }

    fn SRL(&mut self, op1: u8) -> u8 {
        self.setFlag(bit::get(op1, 0),Flags::Carry);
        let result = op1 >> 1;
        self.setZeroFlag(result);
        self.setFlag(false, Flags::Sub);
        self.setFlag(false, Flags::HCarry);
        result
    } 

    fn BIT (&mut self, op1: u8, n: u8) {
        let bit = bit::get(op1, n as usize);
        self.setFlag(bit, Flags::Zero);
        self.setFlag(false, Flags::Sub);
        self.setFlag(true, Flags::HCarry);
    }

    fn RES(&self, op1: u8, n: u8) -> u8 {
        bit::clr(op1, n as usize)
    }

    fn SET(&self, op1: u8, n: u8) -> u8 {
        bit::set(op1, n as usize)
    }

    fn unprefixedOpcodes(&mut self, opcode: u8){
        match opcode {
            0x00 => { // NOP
                
            },
            0x01 => { // LD BC,u16
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.c = self.readByte(self.pc + 1)},
                    4 => {self.b = self.readByte(self.pc + 2)},
                    _ => {}
                }
                
            },
            0x02 => { // LD (BC),A 
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getBC(), self.a)}
                    _ => {}
                }
                
            },
            0x03 => { // INC BC
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.setBC(self.getBC().wrapping_add(1))},
                    _ => {},
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
                    8 => {},
                    4 => {self.b = self.readByte(self.pc + 1)},
                    _ => {},
                }
            },
            0x07 => { // RLCA
                self.a = self.RLC(self.a);
                
            },
            0x08 => { // LD (u16),SP *OBAVEZNO TESTIRATI*
                match self.cyclesLeft {
                    20 => {},
                    16 => {self.fetched = self.readByte(self.pc + 1)},
                    12 => {self.fetched = self.readByte((self.fetched as u16) | ((self.readByte(self.pc + 2) as u16) << 8))},
                    8 => {self.sp = self.fetched as u16},
                    4 => {self.sp |= (self.fetched as u16) << 8},
                    _ => {},
                }
            },
            0x09 => { // ADD HL,BC *nisam siguran*
                match self.cyclesLeft {
                    8 => {},
                    4 => {
                        let (result, carry) = self.getHL().overflowing_add(self.getBC());
                        self.setFlag((self.getHL() & 0xf) + (self.getBC() & 0xf) > 0xf, Flags::HCarry);
                        self.writeBytes(self.getHL(), result);
                        self.setFlag(carry, Flags::Carry);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => {}
                }
            },
            0x0A => { // LD A,(BC)
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.a = self.readByte(self.getBC())}
                    _ => {}
                }
                
            },
            0x0B => { // DEC BC
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.setBC(self.getBC().wrapping_sub(1))},
                    _ => {},
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
                    8 => {},
                    4 => {self.c = self.readByte(self.pc + 1)},
                    _ => {},
                }
            },
            0x0F => { // RRCA
                self.a = self.RRC(self.a);
            }

            0x11 => { // LD DE,u16
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.e = self.readByte(self.pc + 1)},
                    4 => {self.d = self.readByte(self.pc + 2)},
                    _ => {}
                }
                
            },
            0x12 => { // LD (DE),A 
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getDE(), self.a)}
                    _ => {}
                }
                
            },
            0x13 => { // INC DE
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.setDE(self.getDE().wrapping_add(1))},
                    _ => {},
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
                    8 => {},
                    4 => {self.d = self.readByte(self.pc + 1)},
                    _ => {},
                }
                
            },
            0x17 => { // RLA
                self.a = self.RL(self.a);
            },
            0x18 => { // JR i8 
                self.JR_CONDITIONAL(true);
            },
            0x19 => { // ADD HL,DE *nisam siguran*
                match self.cyclesLeft {
                    8 => {},
                    4 => {
                        let (result, carry) = self.getHL().overflowing_add(self.getDE());
                        self.setFlag((self.getHL() & 0xf) + (self.getDE() & 0xf) > 0xf, Flags::HCarry);
                        self.writeBytes(self.getHL(), result);
                        self.setFlag(carry, Flags::Carry);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => {}
                }
            },
            0x1A => { // LD A,(DE)
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.a = self.readByte(self.getDE())}
                    _ => {}
                }
                
            },
            0x1B => { // DEC DE
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.setDE(self.getDE().wrapping_sub(1))},
                    _ => {},
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
                    8 => {},
                    4 => {self.e = self.readByte(self.pc + 1)},
                    _ => {},
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
                    12 => {},
                    8 => {self.l = self.readByte(self.pc + 1)},
                    4 => {self.h = self.readByte(self.pc + 2)},
                    _ => {}
                }
            },
            0x22 => { // LD (HL++),A 
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.a); self.setHL(self.getHL().wrapping_add(1))}
                    _ => {}
                }
            },
            0x23 => { // INC HL
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.setHL(self.getHL().wrapping_add(1))},
                    _ => {},
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
                    8 => {},
                    4 => {self.h = self.readByte(self.pc + 1)},
                    _ => {},
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
                    8 => {},
                    4 => {
                        let (result, carry) = self.getHL().overflowing_add(self.getHL());
                        self.setFlag((self.getHL() & 0xf) + (self.getHL() & 0xf) > 0xf, Flags::HCarry);
                        self.writeBytes(self.getHL(), result);
                        self.setFlag(carry, Flags::Carry);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => {}
                }
            },
            0x2A => { // LD A,(HL++)
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.a = self.readByte(self.getBC()); self.setHL(self.getHL().wrapping_add(1))}
                    _ => {}
                }
            },
            0x2B => { // DEC HL
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.setHL(self.getHL().wrapping_sub(1))},
                    _ => {},
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
                    8 => {},
                    4 => {self.l = self.readByte(self.pc + 1)},
                    _ => {},
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
                    12 => {},
                    8 => {self.sp = self.readByte(self.pc + 1) as u16},
                    4 => {self.sp |= (self.readByte(self.pc + 2) as u16) << 8},
                    _ => {}
                }
            },
            0x32 => { // LD (HL--),A 
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.a); self.setHL(self.getHL().wrapping_sub(1))}
                    _ => {}
                }
            },
            0x33 => { // INC SP
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.sp = self.sp.wrapping_add(1)},
                    _ => {},
                }
            },
            0x34 => { // INC (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {
                        self.setFlag((self.fetched & 0xf) + 1 > 0xf, Flags::HCarry);

                        self.fetched = self.fetched.wrapping_add(1);
                        self.writeByte(self.getHL(), self.fetched);

                        self.setZeroFlag(self.fetched);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => {},
                }
            },
            0x35 => { // DEC (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {
                        self.setFlag((self.fetched & 0xf) as i8 - 1 < 0x0, Flags::HCarry);

                        self.fetched = self.fetched.wrapping_sub(1);
                        self.writeByte(self.getHL(), self.fetched);

                        self.setZeroFlag(self.fetched);
                        self.setFlag(true, Flags::Sub);
                    },
                    _ => {},
                }
            },
            0x36 => { // LD (HL),u8
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.readByte(self.pc+1)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {},
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
                    8 => {},
                    4 => {
                        let (result, carry) = self.getHL().overflowing_add(self.sp);
                        self.setFlag((self.getHL() & 0xf) + (self.sp & 0xf) > 0xf, Flags::HCarry);
                        self.writeBytes(self.getHL(), result);
                        self.setFlag(carry, Flags::Carry);
                        self.setFlag(false, Flags::Sub);
                    },
                    _ => {}
                }
            },
            0x3A => { // LD A,(HL--)
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.a = self.readByte(self.getBC()); self.setHL(self.getHL().wrapping_sub(1))},
                    _ => {}
                }
            },
            0x3B => { // DEC SP
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.sp = self.sp.wrapping_sub(1)},
                    _ => {},
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
                    8 => {},
                    4 => {self.a = self.readByte(self.pc + 1)},
                    _ => {},
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
                    8 => {}
                    4 => {self.b = self.readByte(self.getHL());}
                    _ => {}
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
                    8 => {}
                    4 => {self.c = self.readByte(self.getHL());}
                    _ => {}
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
                    8 => {}
                    4 => {self.d = self.readByte(self.getHL());}
                    _ => {}
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
                    8 => {}
                    4 => {self.e = self.readByte(self.getHL());}
                    _ => {}
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
                    8 => {}
                    4 => {self.h = self.readByte(self.getHL());}
                    _ => {}
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
                    8 => {}
                    4 => {self.l = self.readByte(self.getHL());}
                    _ => {}
                }
            },
            0x6F => { // LD L,A
                self.l = self.a;
            },

            0x70 => { // LD (HL),B
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.b);}
                    _ => {}
                }
            },
            0x71 => { // LD (HL),C
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.c);}
                    _ => {}
                }
            },
            0x72 => { // LD (HL),D
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.d);}
                    _ => {}
                }
            },
            0x73 => { // LD (HL),E
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.e);}
                    _ => {}
                }
            },
            0x74 => { // LD (HL),H
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.h);}
                    _ => {}
                }
            },
            0x75 => { // LD (HL),L
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.l);}
                    _ => {}
                }
            },
            0x76 => { // HALT
                self.halted = true;
            }
            0x77 => { // LD (HL),A
                match self.cyclesLeft {
                    8 => {}
                    4 => {self.writeByte(self.getHL(), self.a);}
                    _ => {}
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
                    8 => {}
                    4 => {self.a = self.readByte(self.getHL());}
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {
                        self.a = self.ADD(self.a, self.fetched);
                    },
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {
                        self.a = self.ADC(self.a, self.fetched);
                    },
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {
                        self.a = self.SUB(self.a, self.fetched);
                    },
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {
                        self.a = self.SBC(self.a, self.fetched);
                    },
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.getHL())}
                    4 => {self.a = self.AND(self.a, self.fetched);}
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.a = self.XOR(self.a, self.fetched);},
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.getHL())}
                    4 => {self.a = self.OR(self.a, self.fetched);}
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.getHL())}
                    4 => {self.SUB(self.a, self.fetched);}
                    _ => {}
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
                    12 => {},
                    8 => {self.c = self.POP8()},
                    4 => {self.b = self.POP8()},
                    _ => {}
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
                    16 => {},
                    12 => {},
                    8 => {self.PUSH8(self.b)},
                    4 => {self.PUSH8(self.c)},
                    _ => {}
                }
            },
            0xC6 => { // ADD A,u8
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.a = self.ADD(self.a, self.fetched)},
                    _ => {}
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
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.a = self.ADC(self.a, self.fetched)},
                    _ => {}
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
                    12 => {},
                    8 => {self.e = self.POP8()},
                    4 => {self.d = self.POP8()},
                    _ => {}
                }
            },
            0xD2 => { // JP NC,u16
                self.JP_CONDITIAL(!self.getFlag(Flags::Carry), self.readBytes(self.pc + 1));
            },
            0xD3 => { // INT 0x40
                self.INT(0x40);
            }
            0xD4 => { // CALL NC,u16
                self.CALL_CONDITIONAL(!self.getFlag(Flags::Carry), self.readBytes(self.pc + 1))
            },
            0xD5 => { // PUSH DE
                match self.cyclesLeft {
                    16 => {},
                    12 => {},
                    8 => {self.PUSH8(self.d)},
                    4 => {self.PUSH8(self.e)},
                    _ => {}
                }
            },
            0xD6 => { // SUB A,u8
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.a = self.SUB(self.a, self.fetched)},
                    _ => {}
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
            0xDB => { // INT 0x60
                self.INT(0x60);
            }
            0xDC => { // CALL C,u16
                self.CALL_CONDITIONAL(self.getFlag(Flags::Carry), self.readBytes(self.pc + 1))
            },
            0xDE => { // SBC A,u8
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.a = self.SBC(self.a, self.fetched)},
                    _ => {}
                }
            },
            0xDF => { // RST 0x18
                self.RST(0x18)
            },

            0xE0 => { // LD (0xFF00+u8),A
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.writeByte(0xFF00 + self.fetched as u16, self.a)},
                    _ => {}
                }
            },
            0xE1 => { // POP HL
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.l = self.POP8()},
                    4 => {self.h = self.POP8()},
                    _ => {}
                }
            },
            0xE2 => { // LD (0xFF00+C),A
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.writeByte(0xFF00 + self.c as u16, self.a)},
                    _ => {}
                }
            },
            0xE3 => { // INT 0x48
                self.INT(0x48);
            },
            0xE4 => { // INT 0x50
                self.INT(0x50);
            }
            0xE5 => { // PUSH HL
                match self.cyclesLeft {
                    16 => {},
                    12 => {},
                    8 => {self.PUSH8(self.h)},
                    4 => {self.PUSH8(self.l)},
                    _ => {}
                }
            },
            0xE6 => { // AND A,u8
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.a = self.AND(self.a, self.fetched)},
                    _ => {}
                }
            },
            0xE7 => { // RST 0x20
                self.RST(0x20);
            },
            0xE8 => { // ADD SP,i8 cycle inaccurate i mozda ne radi
                match self.cyclesLeft {
                    16 => {},
                    12 => {self.fetchedSigned = self.readByte(self.pc + 1) as i8},
                    8 => {},
                    4 => {self.sp = self.sp.wrapping_add(self.fetchedSigned as u16)},
                    _ => {}
                }
            },
            0xE9 => { // JP HL
                self.pc = self.getHL();
            },
            0xEA => { // LD (u16),A
                match self.cyclesLeft {
                    16 => {},
                    12 => {},
                    8 => {},
                    4 => {self.writeByte(self.readBytes(self.pc + 1), self.a)},
                    _ => {}
                }
            },
            0xEE => { // XOR A,u8
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.a = self.XOR(self.a, self.fetched)},
                    _ => {}
                }
            },
            0xEF => { // RST 0x28
                self.RST(0x28)
            },
            

            0xF0 => { // LD A,(0xFF00+u8)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.a = self.readByte(0xFF00 + self.fetched as u16)},
                    _ => {}
                }
            },
            0xF1 => { // POP AF
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.f = self.POP8(); self.f = self.f & 0xF0},
                    4 => {self.a = self.POP8()},
                    _ => {}
                }
            },
            0xF2 => { // LD A,(0xFF00+C)
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.a = self.readByte(0xFF00 + self.c as u16)},
                    _ => {}
                }
            },
            0xF3 => { // DI 
                self.masterInterrupt = false;
            },
            0xF4 => { // INT 0x58
                self.INT(0x58);
            }
            0xF5 => { // PUSH AF
                match self.cyclesLeft {
                    16 => {},
                    12 => {},
                    8 => {self.PUSH8(self.a)},
                    4 => {self.PUSH8(self.f)},
                    _ => {}
                }
            },
            0xF6 => { // ADD A,u8
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.a = self.OR(self.a, self.fetched)},
                    _ => {}
                }
            },
            0xF7 => { // RST 0x30
                self.RST(0x30);
            },
            0xF8 => { // LD HL,SP+i8 cycle inaccurate i mozda ne radi
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetchedSigned = self.readByte(self.pc + 1) as i8},
                    4 => {    
                        self.sp = self.sp.wrapping_add(self.fetchedSigned as u16); 
                        self.setHL(self.sp)},
                    _ => {}
                }
            },
            0xF9 => { // LD SP,HL
                match self.cyclesLeft {
                    8 => {},
                    4 => {self.sp = self.getHL()},
                    _ => {}
                }
            },
            0xFA => { // LD A,(u16)
                match self.cyclesLeft {
                    16 => {},
                    12 => {},
                    8 => {},
                    4 => {self.a = self.readByte(self.readBytes(self.pc + 1))},
                    _ => {}
                }
            },
            0xFB => { // EI
                self.masterInterrupt = true;
            },
            0xFE => { // CP A,u8
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.pc + 1)},
                    4 => {self.SUB(self.a, self.fetched);},
                    _ => {}
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
            0x00 => { // RLC B
                self.b = self.RLC(self.b);
            },
            0x01 => { // RLC C
                self.c = self.RLC(self.c);
            },
            0x02 => { // RLC D
                self.d = self.RLC(self.d);
            },
            0x03 => { // RLC E
                self.e = self.RLC(self.e);
            },
            0x04 => { // RLC H
                self.h = self.RLC(self.h);
            },
            0x05 => { // RLC L
                self.l = self.RLC(self.l);
            },
            0x06 => { // RLC (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RLC(self.readByte(self.getHL()))},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x07 => { // RLC A
                self.a = self.RLC(self.a);
            },
            0x08 => { // RRC B
                self.b = self.RRC(self.b);
            },
            0x09 => { // RRC C
                self.c = self.RRC(self.c);
            },
            0x0A => { // RRC D
                self.d = self.RRC(self.d);
            },
            0x0B => { // RRC E
                self.e = self.RRC(self.e);
            },
            0x0C => { // RRC H
                self.h = self.RRC(self.h);
            },
            0x0D => { // RRC L
                self.l = self.RRC(self.l);
            },
            0x0E => { // RRC (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RRC(self.readByte(self.getHL()))},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x0F => { // RRC A
                self.a = self.RRC(self.a);
            },

            0x10 => { // RL B
                self.b = self.RL(self.b);
            },
            0x11 => { // RL C
                self.c = self.RL(self.c);
            },
            0x12 => { // RL D
                self.d = self.RL(self.d);
            },
            0x13 => { // RL E
                self.e = self.RL(self.e);
            },
            0x14 => { // RL H
                self.h = self.RL(self.h);
            },
            0x15 => { // RL L
                self.l = self.RL(self.l);
            },
            0x16 => { // RL (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RL(self.readByte(self.getHL()))},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x17 => { // RL A
                self.a = self.RL(self.a);
            },
            0x18 => { // RR B
                self.b = self.RR(self.b);
            },
            0x19 => { // RR C
                self.c = self.RR(self.c);
            },
            0x1A => { // RR D
                self.d = self.RR(self.d);
            },
            0x1B => { // RR E
                self.e = self.RR(self.e);
            },
            0x1C => { // RR H
                self.h = self.RR(self.h);
            },
            0x1D => { // RR L
                self.l = self.RR(self.l);
            },
            0x1E => { // RR (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RR(self.readByte(self.getHL()))},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x1F => { // RR A
                self.a = self.RR(self.a);
            },

            0x20 => { // SLA B
                self.b = self.SLA(self.b);
            },
            0x21 => { // SLA C
                self.c = self.SLA(self.c);
            },
            0x22 => { // SLA D
                self.d = self.SLA(self.d);
            },
            0x23 => { // SLA E
                self.e = self.SLA(self.e);
            },
            0x24 => { // SLA H
                self.h = self.SLA(self.h);
            },
            0x25 => { // SLA L
                self.l = self.SLA(self.l);
            },
            0x26 => { // SLA (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SLA(self.readByte(self.getHL()))},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x27 => { // SLA A
                self.a = self.SLA(self.a);
            },
            0x28 => { // SRA B
                self.b = self.SRA(self.b);
            },
            0x29 => { // SRA C
                self.c = self.SRA(self.c);
            },
            0x2A => { // SRA D
                self.d = self.SRA(self.d);
            },
            0x2B => { // SRA E
                self.e = self.SRA(self.e);
            },
            0x2C => { // SRA H
                self.h = self.SRA(self.h);
            },
            0x2D => { // SRA L
                self.l = self.SRA(self.l);
            },
            0x2E => { // SRA (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SRA(self.readByte(self.getHL()))},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x2F => { // SRA A
                self.a = self.SRA(self.a);
            },

            0x30 => { // SWAP B
                self.b = self.SWAP(self.b);
            },
            0x31 => { // SWAP C
                self.c = self.SWAP(self.c);
            },
            0x32 => { // SWAP D
                self.d = self.SWAP(self.d);
            },
            0x33 => { // SWAP E
                self.e = self.SWAP(self.e);
            },
            0x34 => { // SWAP H
                self.h = self.SWAP(self.h);
            },
            0x35 => { // SWAP L
                self.l = self.SWAP(self.l);
            },
            0x36 => { // SWAP (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SWAP(self.readByte(self.getHL()))},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x37 => { // SWAP A
                self.a = self.SWAP(self.a);
            },
            0x38 => { // SRL B
                self.b = self.SRL(self.b);
            },
            0x39 => { // SRL C
                self.c = self.SRL(self.c);
            },
            0x3A => { // SRL D
                self.d = self.SRL(self.d);
            },
            0x3B => { // SRL E
                self.e = self.SRL(self.e);
            },
            0x3C => { // SRL H
                self.h = self.SRL(self.h);
            },
            0x3D => { // SRL L
                self.l = self.SRL(self.l);
            },
            0x3E => { // SRL (HL)
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SRL(self.readByte(self.getHL()))},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x3F => { // SRL A
                self.a = self.SRL(self.a);
            },

            0x40 => { // BIT B,0
                self.BIT(self.b, 0);
            },
            0x41 => { // BIT C,0
                self.BIT(self.c, 0);
            },
            0x42 => { // BIT D,0
                self.BIT(self.d, 0);
            },
            0x43 => { // BIT E,0
                self.BIT(self.e, 0);
            },
            0x44 => { // BIT H,0
                self.BIT(self.h, 0);
            },
            0x45 => { // BIT L,0
                self.BIT(self.l, 0);
            },
            0x46 => { // BIT (HL),0
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.BIT(self.fetched, 0)},
                    _ => {}
                }
            },
            0x47 => { // BIT A,0
                self.BIT(self.a, 0);
            },
            0x48 => { // BIT B,1
                self.BIT(self.b, 1);
            },
            0x49 => { // BIT C,1
                self.BIT(self.c, 1);
            },
            0x4A => { // BIT D,1
                self.BIT(self.d, 1);
            },
            0x4B => { // BIT E,1
                self.BIT(self.e, 1);
            },
            0x4C => { // BIT H,1
                self.BIT(self.h, 1);
            },
            0x4D => { // BIT L,1
                self.BIT(self.l, 1);
            },
            0x4E => { // BIT (HL),1
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.BIT(self.fetched, 1)},
                    _ => {}
                }
            },
            0x4F => { // BIT A,1
                self.BIT(self.a, 1);
            },

            0x50 => { // BIT B,2
                self.BIT(self.b, 2);
            },
            0x51 => { // BIT C,2
                self.BIT(self.c, 2);
            },
            0x52 => { // BIT D,2
                self.BIT(self.d, 2);
            },
            0x53 => { // BIT E,2
                self.BIT(self.e, 2);
            },
            0x54 => { // BIT H,2
                self.BIT(self.h, 2);
            },
            0x55 => { // BIT L,2
                self.BIT(self.l, 2);
            },
            0x56 => { // BIT (HL),2
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.BIT(self.fetched, 2)},
                    _ => {}
                }
            },
            0x57 => { // BIT A,2
                self.BIT(self.a, 2);
            },
            0x58 => { // BIT B,3
                self.BIT(self.b, 3);
            },
            0x59 => { // BIT C,3
                self.BIT(self.c, 3);
            },
            0x5A => { // BIT D,3
                self.BIT(self.d, 3);
            },
            0x5B => { // BIT E,3
                self.BIT(self.e, 3);
            },
            0x5C => { // BIT H,3
                self.BIT(self.h, 3);
            },
            0x5D => { // BIT L,3
                self.BIT(self.l, 3);
            },
            0x5E => { // BIT (HL),3
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.BIT(self.fetched, 3)},
                    _ => {}
                }
            },
            0x5F => { // BIT A,3
                self.BIT(self.a, 3);
            },

            0x60 => { // BIT B,4
                self.BIT(self.b, 4);
            },
            0x61 => { // BIT C,4
                self.BIT(self.c, 4);
            },
            0x62 => { // BIT D,4
                self.BIT(self.d, 4);
            },
            0x63 => { // BIT E,4
                self.BIT(self.e, 4);
            },
            0x64 => { // BIT H,4
                self.BIT(self.h, 4);
            },
            0x65 => { // BIT L,4
                self.BIT(self.l, 4);
            },
            0x66 => { // BIT (HL),4
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.BIT(self.fetched, 4)},
                    _ => {}
                }
            },
            0x67 => { // BIT A,4
                self.BIT(self.a, 4);
            },
            0x68 => { // BIT B,5
                self.BIT(self.b, 5);
            },
            0x69 => { // BIT C,5
                self.BIT(self.c, 5);
            },
            0x6A => { // BIT D,5
                self.BIT(self.d, 5);
            },
            0x6B => { // BIT E,5
                self.BIT(self.e, 5);
            },
            0x6C => { // BIT H,5
                self.BIT(self.h, 5);
            },
            0x6D => { // BIT L,5
                self.BIT(self.l, 5);
            },
            0x6E => { // BIT (HL),5
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.BIT(self.fetched, 5)},
                    _ => {}
                }
            },
            0x6F => { // BIT A,5
                self.BIT(self.a, 5);
            },

            0x70 => { // BIT B,6
                self.BIT(self.b, 6);
            },
            0x71 => { // BIT C,6
                self.BIT(self.c, 6);
            },
            0x72 => { // BIT D,6
                self.BIT(self.d, 6);
            },
            0x73 => { // BIT E,6
                self.BIT(self.e, 6);
            },
            0x74 => { // BIT H,6
                self.BIT(self.h, 6);
            },
            0x75 => { // BIT L,6
                self.BIT(self.l, 6);
            },
            0x76 => { // BIT (HL),6
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.BIT(self.fetched, 6)},
                    _ => {}
                }
            },
            0x77 => { // BIT A,6
                self.BIT(self.a, 6);
            },
            0x78 => { // BIT B,7
                self.BIT(self.b, 7);
            },
            0x79 => { // BIT C,7
                self.BIT(self.c, 7);
            },
            0x7A => { // BIT D,7
                self.BIT(self.d, 7);
            },
            0x7B => { // BIT E,7
                self.BIT(self.e, 7);
            },
            0x7C => { // BIT H,7
                self.BIT(self.h, 7);
            },
            0x7D => { // BIT L,7
                self.BIT(self.l, 7);
            },
            0x7E => { // BIT (HL),7
                match self.cyclesLeft {
                    8 => {self.fetched = self.readByte(self.getHL())},
                    4 => {self.BIT(self.fetched, 7)},
                    _ => {}
                }
            },
            0x7F => { // BIT A,7
                self.BIT(self.a, 7);
            },

            0x80 => { // RES B,0
                self.b = self.RES(self.b, 0);
            },
            0x81 => { // RES C,0
                self.c = self.RES(self.c, 0);
            },
            0x82 => { // RES D,0
                self.d = self.RES(self.d, 0);
            },
            0x83 => { // RES E,0
                self.e = self.RES(self.e, 0);
            },
            0x84 => { // RES H,0
                self.h = self.RES(self.h, 0);
            },
            0x85 => { // RES L,0
                self.l = self.RES(self.l, 0);
            },
            0x86 => { // RES (HL),0
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RES(self.readByte(self.getHL()), 0)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x87 => { // RES A,0
                self.a = self.RES(self.a, 0);
            },
            0x88 => { // RES B,1
                self.b = self.RES(self.b, 1);
            },
            0x89 => { // RES C,1
                self.c = self.RES(self.c, 1);
            },
            0x8A => { // RES D,1
                self.d = self.RES(self.d, 1);
            },
            0x8B => { // RES E,1
                self.e = self.RES(self.e, 1);
            },
            0x8C => { // RES H,1
                self.h = self.RES(self.h, 1);
            },
            0x8D => { // RES L,1
                self.l = self.RES(self.l, 1);
            },
            0x8E => { // RES (HL),1
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RES(self.readByte(self.getHL()), 1)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x8F => { // RES A,1
                self.a = self.RES(self.a, 1);
            },

            0x90 => { // RES B,2
                self.b = self.RES(self.b, 2);
            },
            0x91 => { // RES C,2
                self.c = self.RES(self.c, 2);
            },
            0x92 => { // RES D,2
                self.d = self.RES(self.d, 2);
            },
            0x93 => { // RES E,2
                self.e = self.RES(self.e, 2);
            },
            0x94 => { // RES H,2
                self.h = self.RES(self.h, 2);
            },
            0x95 => { // RES L,2
                self.l = self.RES(self.l, 2);
            },
            0x96 => { // RES (HL),2
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RES(self.readByte(self.getHL()), 2)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x97 => { // RES A,2
                self.a = self.RES(self.a, 2);
            },
            0x98 => { // RES B,3
                self.b = self.RES(self.b, 3);
            },
            0x99 => { // RES C,3
                self.c = self.RES(self.c, 3);
            },
            0x9A => { // RES D,3
                self.d = self.RES(self.d, 3);
            },
            0x9B => { // RES E,3
                self.e = self.RES(self.e, 3);
            },
            0x9C => { // RES H,3
                self.h = self.RES(self.h, 3);
            },
            0x9D => { // RES L,3
                self.l = self.RES(self.l, 3);
            },
            0x9E => { // RES (HL),3
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RES(self.readByte(self.getHL()), 3)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0x9F => { // RES A,3
                self.a = self.RES(self.a, 3);
            },

            0xA0 => { // RES B,4
                self.b = self.RES(self.b, 4);
            },
            0xA1 => { // RES C,4
                self.c = self.RES(self.c, 4);
            },
            0xA2 => { // RES D,4
                self.d = self.RES(self.d, 4);
            },
            0xA3 => { // RES E,4
                self.e = self.RES(self.e, 4);
            },
            0xA4 => { // RES H,4
                self.h = self.RES(self.h, 4);
            },
            0xA5 => { // RES L,4
                self.l = self.RES(self.l, 4);
            },
            0xA6 => { // RES (HL),4
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RES(self.readByte(self.getHL()), 4)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xA7 => { // RES A,4
                self.a = self.RES(self.a, 4);
            },
            0xA8 => { // RES B,5
                self.b = self.RES(self.b, 5);
            },
            0xA9 => { // RES C,5
                self.c = self.RES(self.c, 5);
            },
            0xAA => { // RES D,5
                self.d = self.RES(self.d, 5);
            },
            0xAB => { // RES E,5
                self.e = self.RES(self.e, 5);
            },
            0xAC => { // RES H,5
                self.h = self.RES(self.h, 5);
            },
            0xAD => { // RES L,5
                self.l = self.RES(self.l, 5);
            },
            0xAE => { // RES (HL),5
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RES(self.readByte(self.getHL()), 5)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xAF => { // RES A,5
                self.a = self.RES(self.a, 5);
            },

            0xB0 => { // RES B,6
                self.b = self.RES(self.b, 6);
            },
            0xB1 => { // RES C,6
                self.c = self.RES(self.c, 6);
            },
            0xB2 => { // RES D,6
                self.d = self.RES(self.d, 6);
            },
            0xB3 => { // RES E,6
                self.e = self.RES(self.e, 6);
            },
            0xB4 => { // RES H,6
                self.h = self.RES(self.h, 6);
            },
            0xB5 => { // RES L,6
                self.l = self.RES(self.l, 6);
            },
            0xB6 => { // RES (HL),6
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RES(self.readByte(self.getHL()), 6)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xB7 => { // RES A,6
                self.a = self.RES(self.a, 6);
            },
            0xB8 => { // RES B,7
                self.b = self.RES(self.b, 7);
            },
            0xB9 => { // RES C,7
                self.c = self.RES(self.c, 7);
            },
            0xBA => { // RES D,7
                self.d = self.RES(self.d, 7);
            },
            0xBB => { // RES E,7
                self.e = self.RES(self.e, 7);
            },
            0xBC => { // RES H,7
                self.h = self.RES(self.h, 7);
            },
            0xBD => { // RES L,7
                self.l = self.RES(self.l, 7);
            },
            0xBE => { // RES (HL),7
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.RES(self.readByte(self.getHL()), 7)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xBF => { // RES A,7
                self.a = self.RES(self.a, 7);
            },

            0xC0 => { // SET B,0
                self.b = self.SET(self.b, 0);
            },
            0xC1 => { // SET C,0
                self.c = self.SET(self.c, 0);
            },
            0xC2 => { // SET D,0
                self.d = self.SET(self.d, 0);
            },
            0xC3 => { // SET E,0
                self.e = self.SET(self.e, 0);
            },
            0xC4 => { // SET H,0
                self.h = self.SET(self.h, 0);
            },
            0xC5 => { // RESSET L,0
                self.l = self.SET(self.l, 0);
            },
            0xC6 => { // SET (HL),0
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SET(self.readByte(self.getHL()), 0)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xC7 => { // SET A,0
                self.a = self.SET(self.a, 0);
            },
            0xC8 => { // SET B,1
                self.b = self.SET(self.b, 1);
            },
            0xC9 => { // SET C,1
                self.c = self.SET(self.c, 1);
            },
            0xCA => { // SET D,1
                self.d = self.SET(self.d, 1);
            },
            0xCB => { // SET E,1
                self.e = self.SET(self.e, 1);
            },
            0xCC => { // SET H,1
                self.h = self.SET(self.h, 1);
            },
            0xCD => { // SET L,1
                self.l = self.SET(self.l, 1);
            },
            0xCE => { // SET (HL),1
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SET(self.readByte(self.getHL()), 1)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xCF => { // SET A,1
                self.a = self.SET(self.a, 1);
            },

            0xD0 => { // SET B,2
                self.b = self.SET(self.b, 2);
            },
            0xD1 => { // SET C,2
                self.c = self.SET(self.c, 2);
            },
            0xD2 => { // SET D,2
                self.d = self.SET(self.d, 2);
            },
            0xD3 => { // SET E,2
                self.e = self.SET(self.e, 2);
            },
            0xD4 => { // SET H,2
                self.h = self.SET(self.h, 2);
            },
            0xD5 => { // SET L,2
                self.l = self.SET(self.l, 2);
            },
            0xD6 => { // SET (HL),2
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SET(self.readByte(self.getHL()), 2)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xD7 => { // SET A,2
                self.a = self.SET(self.a, 2);
            },
            0xD8 => { // SET B,3
                self.b = self.SET(self.b, 3);
            },
            0xD9 => { // SET C,3
                self.c = self.SET(self.c, 3);
            },
            0xDA => { // SET D,3
                self.d = self.SET(self.d, 3);
            },
            0xDB => { // SET E,3
                self.e = self.SET(self.e, 3);
            },
            0xDC => { // SET H,3
                self.h = self.SET(self.h, 3);
            },
            0xDD => { // SET L,3
                self.l = self.SET(self.l, 3);
            },
            0xDE => { // SET (HL),3
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SET(self.readByte(self.getHL()), 3)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xDF => { // SET A,3
                self.a = self.SET(self.a, 3);
            },

            0xE0 => { // SET B,4
                self.b = self.SET(self.b, 4);
            },
            0xE1 => { // SET C,4
                self.c = self.SET(self.c, 4);
            },
            0xE2 => { // SET D,4
                self.d = self.SET(self.d, 4);
            },
            0xE3 => { // SET E,4
                self.e = self.SET(self.e, 4);
            },
            0xE4 => { // SET H,4
                self.h = self.SET(self.h, 4);
            },
            0xE5 => { // SET L,4
                self.l = self.SET(self.l, 4);
            },
            0xE6 => { // SET (HL),4
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SET(self.readByte(self.getHL()), 4)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xE7 => { // SET A,4
                self.a = self.SET(self.a, 4);
            },
            0xE8 => { // SET B,5
                self.b = self.SET(self.b, 5);
            },
            0xE9 => { // SET C,5
                self.c = self.SET(self.c, 5);
            },
            0xEA => { // SET D,5
                self.d = self.SET(self.d, 5);
            },
            0xEB => { // SET E,5
                self.e = self.SET(self.e, 5);
            },
            0xEC => { // SET H,5
                self.h = self.SET(self.h, 5);
            },
            0xED => { // SET L,5
                self.l = self.SET(self.l, 5);
            },
            0xEE => { // SET (HL),5
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SET(self.readByte(self.getHL()), 5)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xEF => { // SET A,5
                self.a = self.SET(self.a, 5);
            },

            0xF0 => { // SET B,6
                self.b = self.SET(self.b, 6);
            },
            0xF1 => { // SET C,6
                self.c = self.SET(self.c, 6);
            },
            0xF2 => { // SET D,6
                self.d = self.SET(self.d, 6);
            },
            0xF3 => { // SET E,6
                self.e = self.SET(self.e, 6);
            },
            0xF4 => { // SET H,6
                self.h = self.SET(self.h, 6);
            },
            0xF5 => { // SET L,6
                self.l = self.SET(self.l, 6);
            },
            0xF6 => { // SET (HL),6
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SET(self.readByte(self.getHL()), 6)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xF7 => { // SET A,6
                self.a = self.SET(self.a, 6);
            },
            0xF8 => { // SET B,7
                self.b = self.SET(self.b, 7);
            },
            0xF9 => { // SET C,7
                self.c = self.SET(self.c, 7);
            },
            0xFA => { // SET D,7
                self.d = self.SET(self.d, 7);
            },
            0xFB => { // SET E,7
                self.e = self.SET(self.e, 7);
            },
            0xFC => { // SET H,7
                self.h = self.SET(self.h, 7);
            },
            0xFD => { // SET L,7
                self.l = self.SET(self.l, 7);
            },
            0xFE => { // RES (HL),7
                match self.cyclesLeft {
                    12 => {},
                    8 => {self.fetched = self.SET(self.readByte(self.getHL()), 7)},
                    4 => {self.writeByte(self.getHL(), self.fetched)},
                    _ => {}
                }
            },
            0xFF => { // SET A,7
                self.a = self.SET(self.a, 7);
            },
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
    }

    fn getInstructionInfo(&self, opcode: u8) -> (&str, u8, u8) {
        if self.prefixedInstruction {
            PREFIXED_INSTRUCTION_TABLE[opcode as usize]
        } else {
            UNPREFIXED_INSTRUCTION_TABLE[opcode as usize]
        }
    }

    fn handleInterrupts(&mut self) {
        if self.masterInterrupt && !self.cbFlag{
            if self.bus.getInterruptRequest(IntrFlags::VBlank) && self.bus.getInterruptEnable(IntrFlags::VBlank) {

            } else if self.bus.getInterruptRequest(IntrFlags::LCD) && self.bus.getInterruptEnable(IntrFlags::LCD) {

            } else if self.bus.getInterruptRequest(IntrFlags::Timer) && self.bus.getInterruptEnable(IntrFlags::Timer) {
                self.bus.resetInterruptRequest(IntrFlags::Timer);
                self.masterInterrupt = false;
                self.halted = false;
                self.currentOpcode = 0xE4;
                self.cyclesLeft = 20;
                return;
            } else if self.bus.getInterruptRequest(IntrFlags::Serial) && self.bus.getInterruptEnable(IntrFlags::Serial) {

            } else if self.bus.getInterruptRequest(IntrFlags::Joypad) && self.bus.getInterruptEnable(IntrFlags::Joypad) {

            }
        }
    }

    pub fn clock(&mut self) {
        if self.justBooted {
            self.currentOpcode = self.readByte(self.pc);
            let (_, _, cycles) = self.getInstructionInfo(self.currentOpcode);
            self.cyclesLeft = cycles * 4;
            self.justBooted = false;
        }
        if !self.halted {
            for _i in 0..4 {
            self.executeOneCycle(self.currentOpcode);
            self.cyclesLeft -= 1;
            if self.bus.timerRegisters.incrTimers() {
                self.bus.requestInterrupt(IntrFlags::Timer);
            }}
        }

        if self.cyclesLeft == 0 {
            let (_, length, _) = self.getInstructionInfo(self.currentOpcode);
            if !self.branchTaken {
                self.pc = self.pc.wrapping_add(length as u16);
            }
            self.branchTaken = false;

            // brateee testirati sve ovo ako je instrukcija prefiksovana ili nesto
            self.handleInterrupts();
            
            
            self.currentOpcode = self.readByte(self.pc);
            let (_, _, cycles) = self.getInstructionInfo(self.currentOpcode);
            self.cyclesLeft = cycles * 4;
        }
    }

    pub fn reset(&mut self) {
        self.setAF(0x01B0);
        self.setBC(0x0013);
        self.setDE(0x00D8);
        self.setHL(0x014D);
        self.sp = 0xFFFE;

    }
}

pub const UNPREFIXED_INSTRUCTION_TABLE: [(&str, u8, u8); 256]= [
    ("NOP", 1, 1),              ("LD BC,u16", 3, 3),    ("LD (BC), A", 1, 2),       ("INC BC", 1, 2),       ("INC B", 1, 1),        ("DEC B", 1, 1),        ("LD B,u8", 2, 2),      ("RLCA", 1, 1),         ("LD (u16),SP", 3, 5),  ("ADD HL,BC", 1, 2),    ("LD A,(BC)", 1, 2),    ("DEC BC", 1, 2),   ("INC C", 1, 1),        ("DEC C", 1, 1),    ("LD C,u8", 2, 2),      ("RRCA", 1, 1),
    ("STOP", 2, 1),             ("LD DE,u16", 3, 3),    ("LD (DE), A", 1, 2),       ("INC DE", 1, 2),       ("INC D", 1, 1),        ("DEC D", 1, 1),        ("LD D,u8", 2, 2),      ("RLA", 1, 1),          ("JR i8", 2, 3),        ("ADD HL,DE", 1, 2),    ("LD A,(DE)", 1, 2),    ("DEC DE", 1, 2),   ("INC E", 1, 1),        ("DEC E", 1, 1),    ("LD E,u8", 2, 2),      ("PRA", 1, 1),
    ("JR NZ,r8", 2, 12),        ("LD HL,16", 3, 3),     ("LD (HL++), A", 1, 2),     ("INC HL", 1, 2),       ("INC H", 1, 1),        ("DEC H", 1, 1),        ("LD H,u8", 2, 2),      ("DAA", 1, 1),          ("JR Z,i8", 2, 3),      ("ADD HL,HL", 1, 2),    ("LD A,(HL++)", 1, 2),  ("DEC HL", 1, 2),   ("INC L", 1, 1),        ("DEC L", 1, 1),    ("LD L,u8", 2, 2),      ("CPL", 1, 1),
    ("JR NC,r8", 2, 12),        ("LD SP,16", 3, 3),     ("LD (HL--), A", 1, 2),     ("INC SP", 1, 2),       ("INC (HL)", 1, 3),     ("DEC (HL)", 1, 3),     ("LD (HL),u8", 2, 3),   ("SCF", 1, 1),          ("JR C,i8", 2, 3),      ("ADD HL,SP", 1, 2),    ("LD A,(HL--)", 1, 2),  ("DEC SP", 1, 2),   ("INC A", 1, 1),        ("DEC A", 1, 1),    ("LD A,u8", 2, 2),      ("CCF", 1, 1),
    ("LD B,B", 1, 1),           ("LD B,C", 1, 1),       ("LD B,D", 1, 1),           ("LD B,E", 1, 1),       ("LD B,H", 1, 1),       ("LD B,L", 1, 1),       ("LD B,(HL)", 1, 2),    ("LD B,A", 1, 1),       ("LD C,B", 1, 1),       ("LD C,C", 1, 1),       ("LD C,D", 1, 1),       ("LD C,E", 1, 1),   ("LD C,H", 1, 1),       ("LD C,L", 1, 1),   ("LD C,(HL)", 1, 2),    ("LD C,A", 1, 1),
    ("LD D,B", 1, 1),           ("LD D,C", 1, 1),       ("LD D,D", 1, 1),           ("LD D,E", 1, 1),       ("LD D,H", 1, 1),       ("LD D,L", 1, 1),       ("LD D,(HL)", 1, 2),    ("LD D,A", 1, 1),       ("LD E,B", 1, 1),       ("LD E,C", 1, 1),       ("LD E,D", 1, 1),       ("LD E,E", 1, 1),   ("LD E,H", 1, 1),       ("LD E,L", 1, 1),   ("LD E,(HL)", 1, 2),    ("LD E,A", 1, 1),
    ("LD H,B", 1, 1),           ("LD H,C", 1, 1),       ("LD H,D", 1, 1),           ("LD H,E", 1, 1),       ("LD H,H", 1, 1),       ("LD H,L", 1, 1),       ("LD H,(HL)", 1, 2),    ("LD H,A", 1, 1),       ("LD L,B", 1, 1),       ("LD L,C", 1, 1),       ("LD L,D", 1, 1),       ("LD L,E", 1, 1),   ("LD L,H", 1, 1),       ("LD L,L", 1, 1),   ("LD L,(HL)", 1, 2),    ("LD L,A", 1, 1),
    ("LD (HL),B", 1, 2),        ("LD (HL),C", 1, 2),    ("LD (HL),D", 1, 2),        ("LD (HL),E", 1, 2),    ("LD (HL),H", 1, 2),    ("LD (HL),L", 1, 2),    ("HALT", 1, 1),         ("LD (HL),A", 1, 2),    ("LD A,B", 1, 1),       ("LD A,C", 1, 1),       ("LD A,D", 1, 1),       ("LD A,E", 1, 1),   ("LD A,H", 1, 1),       ("LD A,L", 1, 1),   ("LD A,(HL)", 1, 2),    ("LD A,A", 1, 1),
    ("ADD A,B", 1, 1),          ("ADD A,C", 1, 1),      ("ADD A,D", 1, 1),          ("ADD A,E", 1, 1),      ("ADD A,H", 1, 1),      ("ADD A,L", 1, 1),      ("ADD A,(HL)", 1, 2),   ("ADD A,A", 1, 1),      ("ADC A,B", 1, 1),      ("ADC A,C", 1, 1),      ("ADC A,D", 1, 1),      ("ADC A,E", 1, 1),  ("ADC A,H", 1, 1),      ("ADC A,L", 1, 1),  ("ADC A,(HL)", 1, 2),   ("ADC A,A", 1, 1),
    ("SUB A,B", 1, 1),          ("SUB A,C", 1, 1),      ("SUB A,D", 1, 1),          ("SUB A,E", 1, 1),      ("SUB A,H", 1, 1),      ("SUB A,L", 1, 1),      ("SUB A,(HL)", 1, 2),   ("SUB A,A", 1, 1),      ("SBC A,B", 1, 1),      ("SBC A,C", 1, 1),      ("SBC A,D", 1, 1),      ("SBC A,E", 1, 1),  ("SBC A,H", 1, 1),      ("SBC A,L", 1, 1),  ("SBC A,(HL)", 1, 2),   ("SBC A,A", 1, 1),
    ("AND A,B", 1, 1),          ("AND A,C", 1, 1),      ("AND A,D", 1, 1),          ("AND A,E", 1, 1),      ("AND A,H", 1, 1),      ("AND A,L", 1, 1),      ("AND A,(HL)", 1, 2),   ("AND A,A", 1, 1),      ("XOR A,B", 1, 1),      ("XOR A,C", 1, 1),      ("XOR A,D", 1, 1),      ("XOR A,E", 1, 1),  ("XOR A,H", 1, 1),      ("XOR A,L", 1, 1),  ("XOR A,(HL)", 1, 2),   ("XOR A,A", 1, 1),
    ("OR A,B", 1, 1),           ("OR A,C", 1, 1),       ("OR A,D", 1, 1),           ("OR A,E", 1, 1),       ("OR A,H", 1, 1),       ("OR A,L", 1, 1),       ("OR A,(HL)", 1, 2),    ("OR A,A", 1, 1),       ("CP A,B", 1, 1),       ("CP A,C", 1, 1),       ("CP A,D", 1, 1),       ("CP A,E", 1, 1),   ("CP A,H", 1, 1),       ("CP A,L", 1, 1),   ("CP A,(HL)", 1, 2),    ("CP A,A", 1, 1),
    ("RET NZ", 1, 5),           ("POP BC", 1, 3),       ("JP NZ,u16", 3, 4),        ("JP u16", 3, 4),       ("CALL NZ,u16", 3, 6),  ("PUSH BC", 1, 4),      ("ADD A,u8", 2, 2),     ("RST 0x00", 1, 4),     ("RET Z", 1, 5),        ("RET", 1, 4),          ("JP Z,u16", 3, 4),     ("CB", 1, 1),       ("CALL Z,u16", 3, 6),   ("CALL u16", 3, 6), ("ADC A,u8", 2, 2),     ("RST 0x08", 1, 4),
    ("RET NC", 1, 5),           ("POP DE", 1, 3),       ("JP NC,u16", 3, 4),        ("INT 0x40", 1, 5),     ("CALL NC,u16", 3, 6),  ("PUSH DE", 1, 4),      ("SUB A,u8", 2, 2),     ("RST 0x10", 1, 4),     ("RET C", 1, 5),        ("RETI", 1, 4),         ("JP C,u16", 3, 4),     ("INT 0x60", 1, 5), ("CALL C,u16", 3, 6),   ("", 0, 0),         ("SBC A,u8", 2, 2),     ("RST 0x18", 1, 4),
    ("LD (0xFF00+u8),A", 2, 3), ("POP HL", 1, 3),       ("LD (0xFF00+C),A", 1, 2),  ("INT 0x48", 1, 5),     ("INT 0x50", 1, 5),     ("PUSH HL", 1, 4),      ("AND A,u8", 2, 2),     ("RST 0x20", 1, 4),     ("ADD SP,i8", 2, 4),    ("JP HL", 1, 1),        ("LD (u16),A", 3, 4),   ("", 0, 0),         ("", 0, 0),             ("", 0, 0),         ("XOR A,u8", 2, 2),     ("RST 0x28", 1, 4),
    ("LD A,(0xFF00+u8)", 2, 3), ("POP AF", 1, 3),       ("LD A,(0xFF00+C)", 1, 2),  ("DI", 1, 1),           ("INT 0x58", 1, 5),     ("PUSH AF", 1, 4),      ("OR A,u8", 2, 2),      ("RST 0x30", 1, 4),     ("LD HL,SP+i8", 2, 3),  ("LD SP,HL", 1, 2),     ("LD A,(u16)", 3, 4),   ("EI", 1, 1),       ("", 0, 0),             ("", 0, 0),         ("CP A,u8", 2, 2),      ("RST 0x38", 1, 4),
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

