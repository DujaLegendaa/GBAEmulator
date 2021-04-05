use super::bit;
use super::bus::{Bus};
struct Z80{
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,

    bus: Bus,
    cyclesLeft: u8,
    fetched: u8
}

enum Flags {
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
    fn new() -> Self{
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
        }
    }


    fn getAF(&self) -> u16{
        let mut af: u16;
        af = ((self.a as u16)<<8) | (self.f as u16);
        af
    }

    fn getBC(&self) -> u16{
        let mut bc: u16;
        bc = ((self.b as u16)<<8) | (self.c as u16);
        bc
    }

    fn getDE(&self) -> u16{
        let mut de: u16;
        de = ((self.d as u16)<<8) | (self.e as u16);
        de
    }

    fn getHL(&self) -> u16{
        let mut hl: u16;
        hl = ((self.h as u16)<<8) | (self.l as u16);
        hl
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

    fn getFlag(&self, fl: Flags) -> bool {
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

const nameVector: Vec<String> = vec![

];

impl Z80 {
    fn readByte(&self, addr: u16) -> u8 {
        self.bus.cpuRead(addr)
    }
    fn readBytes(&self, addr: u16) -> u16 {
        u16::from_le_bytes([
            self.bus.cpuRead(addr),
            self.bus.cpuRead(addr + 1)
        ])
    }
    fn writeByte(&mut self, addr: u16, data: u8) {
        self.bus.cpuWrite(addr, data);
    }
    fn writeBytes(&mut self, addr: u16, data: u16) {
        let dHI = (data >> 8) as u8;
        let dLO = data as u8;
        self.bus.cpuWrite(addr, dLO);
        self.bus.cpuWrite(addr, dHI);
    }
    fn executeOneCycle(&mut self, opcode: u8) {
        match opcode {
            0x00 => {

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
            }
            0x51 => { // LD D,C
                self.d = self.c;
            }
            0x52 => { // LD D,D
                self.d = self.d;
            }
            0x53 => { // LD D,E
                self.d = self.e;
            }
            0x54 => { // LD D,H
                self.d = self.h;
            }
            0x55 => { // LD D,L
                self.d = self.l;
            }
            0x56 => { // LD D,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.d = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x57 => { // LD D,A
                self.d = self.a;
            }
            0x58 => { // LD E,B
                self.e = self.b;
            }
            0x59 => { // LD E,C
                self.e = self.c;
            }
            0x5A => { // LD E,D
                self.e = self.d;
            }
            0x5B => { // LD E,E
                self.e = self.e;
            }
            0x5C => { // LD E,H
                self.e = self.h;
            }
            0x5D => { // LD E,L
                self.e = self.l;
            }
            0x5E => { // LD E,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.e = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x5F => { // LD E,A
                self.e = self.a;
            }

            0x60 => { // LD H,B
                self.h = self.b;
            }
            0x61 => { // LD H,C
                self.h = self.c;
            }
            0x62 => { // LD H,D
                self.h = self.d;
            }
            0x63 => { // LD H,E
                self.h = self.e;
            }
            0x64 => { // LD H,H
                self.h = self.h;
            }
            0x65 => { // LD H,L
                self.h = self.l;
            }
            0x66 => { // LD H,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.h = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x67 => { // LD H,A
                self.h = self.a;
            }
            0x68 => { // LD L,B
                self.l = self.b;
            }
            0x69 => { // LD L,C
                self.l = self.c;
            }
            0x6A => { // LD L,D
                self.l = self.d;
            }
            0x6B => { // LD L,E
                self.l = self.e;
            }
            0x6C => { // LD L,H
                self.l = self.h;
            }
            0x6D => { // LD L,L
                self.l = self.l;
            }
            0x6E => { // LD L,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.l = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x6F => { // LD L,A
                self.l = self.a;
            }

            0x70 => { // LD (HL),B
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.b);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x71 => { // LD (HL),C
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.c);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x72 => { // LD (HL),D
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.d);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x73 => { // LD (HL),E
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.e);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x74 => { // LD (HL),H
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.h);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x75 => { // LD (HL),L
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.l);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            //0x76 => { // HALT}
            0x77 => {
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.writeByte(self.getHL(), self.a);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x78 => { // LD A,B
                self.a = self.b;
            }
            0x79 => { // LD A,C
                self.a = self.c;
            }
            0x7A => { // LD A,D
                self.a = self.d;
            }
            0x7B => { // LD A,E
                self.a = self.e;
            }
            0x7C => { // LD A,H
                self.a = self.h;
            }
            0x7D => { // LD A,L
                self.a = self.l;
            }
            0x7E => { // LD A,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.a = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x7F => { // LD A,A
                self.a = self.a;
            }

            0xA0 => { // AND A,B
                self.a &= self.b;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(true,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xA1 => { // AND A,C
                self.a &= self.c;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(true,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xA2 => { // AND A,D
                self.a &= self.d;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(true,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xA3 => { // AND A,E
                self.a &= self.e;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(true,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xA4 => { // AND A,H
                self.a &= self.h;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(true,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xA5 => { // AND A,L
                self.a &= self.l;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(true,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xA6 => { // AND A,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.a &= self.readByte(self.getHL());
                        self.setZeroFlag(self.a);
                        self.setFlag(false,Flags::Sub);
                        self.setFlag(true,Flags::HCarry);
                        self.setFlag(false,Flags::Carry);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0xA7 => { // AND A,A
                self.a &= self.a;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(true,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xA8 => { // XOR A,B
                self.a ^= self.b;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xA9 => { // XOR A,C
                self.a ^= self.c;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xAA => { // XOR A,D
                self.a ^= self.d;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xAB => { // XOR A,E
                self.a ^= self.e;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xAC => { // XOR A,H
                self.a ^= self.h;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xAD => { // XOR A,L
                self.a ^= self.l;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xAE => { // XOR A,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.a ^= self.readByte(self.getHL());
                        self.setZeroFlag(self.a);
                        self.setFlag(false,Flags::Sub);
                        self.setFlag(false,Flags::HCarry);
                        self.setFlag(false,Flags::Carry);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0xAF => { // XOR A,A
                self.a ^= self.a;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }

            0xB0 => { // OR A,B
                self.a |= self.b;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xB1 => { // OR A,C
                self.a |= self.c;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xB2 => { // OR A,D
                self.a |= self.d;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xB3 => { // OR A,E
                self.a |= self.e;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xB4 => { // OR A,H
                self.a |= self.h;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xB5 => { // OR A,L
                self.a |= self.l;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            0xB6 => { // OR A,(HL)
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.a |= self.readByte(self.getHL());
                        self.setZeroFlag(self.a);
                        self.setFlag(false,Flags::Sub);
                        self.setFlag(false,Flags::HCarry);
                        self.setFlag(false,Flags::Carry);}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0xB7 => { // OR A,A
                self.a |= self.a;
                self.setZeroFlag(self.a);
                self.setFlag(false,Flags::Sub);
                self.setFlag(false,Flags::HCarry);
                self.setFlag(false,Flags::Carry);
            }
            _ => panic!("Unknown opcode or not implemented"),
        }
        self.cyclesLeft -= 1;
    }
}

