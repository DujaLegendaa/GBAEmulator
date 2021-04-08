pub fn get(v: u8, n: usize) -> bool {
    (v & (1 << n)) != 0
}

pub fn set(v: u8, n: usize) -> u8 {
    v | (1 << n)
}

pub fn clr(v: u8, n: usize) -> u8 {
    v & !(1 << n)
}

pub fn get16(v: u16, n: usize) -> bool {
    (v & (1 << n)) != 0
}