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
    fn executeInstrction(&mut self, opcode: u8) {
        match opcode {
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
                self.b = self.readByte(self.getHL());
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

            _ => panic!("Unknown opcode or not implemented"),
        }
    }
}

