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

            }

            0x40 => {
                self.b = self.b;
            }
            0x41 => {
                self.b = self.c;
            }
            0x42 => {
                self.b = self.d;
            }
            0x43 => {
                self.b = self.e;
            }
            0x44 => {
                self.b = self.h;
            }
            0x45 => {
                self.b = self.l;
            }
            0x46 => {
                match self.cyclesLeft {
                    2 => {}
                    1 => {self.b = self.readByte(self.getHL());}
                    _ => {panic!("cycles left incorrect")}
                }
            }
            0x47 => {
                self.b = self.a;
            }
            0x48 => {
                self.c = self.b;
            }
            0x49 => {
                self.c = self.c;
            }
            0x4A => {
                self.c = self.d;
            }
            0x4B => {
                self.c = self.e;
            }
            0x4C => {
                self.c = self.h;
            }
            0x4D => {
                self.c = self.l;
            }
            0x4E => {
                self.c = self.readByte(self.getHL());
            }
            0x4F => {
                self.c = self.a;
            }


            0x50 => {
                self.d = self.b;
            }
            0x51 => {
                self.d = self.c;
            }
            0x52 => {
                self.d = self.d;
            }
            0x53 => {
                self.d = self.e;
            }
            0x54 => {
                self.d = self.h;
            }
            0x55 => {
                self.d = self.l;
            }
            /*0x56 => {
                
            }
            */
            0x57 => {
                self.d = self.a;
            }
            0x58 => {
                self.e = self.b;
            }
            0x59 => {
                self.e = self.c;
            }
            0x5A => {
                self.e = self.d;
            }
            0x5B => {
                self.e = self.e;
            }
            0x5C => {
                self.e = self.h;
            }
            0x5D => {
                self.e = self.l;
            }
            /*0x5E => {
                self.e
            }
            */
            0x5F => {
                self.e = self.a;
            }

            0x60 => {
                self.h = self.b;
            }
            0x61 => {
                self.h = self.c;
            }
            0x62 => {
                self.h = self.d;
            }
            0x63 => {
                self.h = self.e;
            }
            0x64 => {
                self.h = self.h;
            }
            0x65 => {
                self.h = self.l;
            }
            /*
            0x66 => {
                self.h =
            }
            */
            0x67 => {
                self.h = self.a;
            }
            0x68 => {
                self.l = self.b;
            }
            0x69 => {
                self.l = self.c;
            }
            0x6A => {
                self.l = self.d;
            }
            0x6B => {
                self.l = self.e;
            }
            0x6C => {
                self.l = self.h;
            }
            0x6D => {
                self.l = self.l;
            }
            /*
            0x6E => {
                self.l =
            }
            */
            0x6F => {
                self.l = self.a;
            }

            
            _ => panic!("Unknown opcode or not implemented"),
        }
        self.cyclesLeft -= 1;
    }
}

