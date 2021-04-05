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
    pc: u16
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
            pc: 0
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
}
