pub struct Bus {
    ram: [u8; 0xff],
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram: [0; 0xff]
        }
    }
}