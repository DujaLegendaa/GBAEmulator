pub struct Bus {
    ram1: [u8; 4 * 1024],
    ram2: [u8; 4 * 1024],
    highRam: [u8; 127],

    interruptRegister: u8,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram1: [0; 4 * 1024],
            ram2: [0; 4 * 1024],
            highRam: [0; 127],

            interruptRegister: 0,
        }
    }

    pub fn cpuRead(&self, addr: u16) -> u8 {
        match addr {
            0x0000..= 0x3FFF => {todo!("Needs cartridge implementation")},
            0x4000..= 0x7FFF => {todo!("Needs cartridge and mapper implementation")},
            0x8000..= 0x9FFF => {
                todo!("Vram not implemented")
            },
            0xA000..= 0xBFFF => {todo!("Needs cartridge and mapper implementation")},
            0xC000..= 0xCFFF => {
                self.ram1[(addr & 0x0fff) as usize]
            },
            0xD000..= 0xDFFF => {
                self.ram2[(addr & 0x0fff) as usize]
            },
            0xE000..= 0xFDFF => {
                if addr <= 0xEFFF {
                    self.ram1[(addr & 0x0fff) as usize]
                } else {
                    self.ram2[(addr & 0x0fff) as usize]
                }
            },
            0xFE00..= 0xFE9F => {
                panic!("Not usable memory");
            },
            0xFEA0..= 0xFEFF => {
                todo!("I/O registers")
            },
            0xFF00..= 0xFF7F => {
                todo!("Sprite table not implemented");
            },
            0xFF80..= 0xFFFE => {
                self.highRam[((addr & 0x00ff) - 0x0080) as usize]
            },
            0xFFFF => {
                self.interruptRegister
            }
        }
    }

    pub fn cpuWrite(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..= 0x3FFF => {todo!("Needs cartridge implementation")},
            0x4000..= 0x7FFF => {todo!("Needs cartridge and mapper implementation")},
            0x8000..= 0x9FFF => {
                todo!("Vram not implemented")
            },
            0xA000..= 0xBFFF => {todo!("Needs cartridge and mapper implementation")},
            0xC000..= 0xCFFF => {
                self.ram1[(addr & 0x0fff) as usize] = data;
            },
            0xD000..= 0xDFFF => {
                self.ram2[(addr & 0x0fff) as usize] = data;
            },
            0xE000..= 0xFDFF => {
                if addr <= 0xEFFF {
                    self.ram1[(addr & 0x0fff) as usize] = data;
                } else {
                    self.ram2[(addr & 0x0fff) as usize] = data;
                }
            },
            0xFE00..= 0xFE9F => {
                panic!("Not usable memory");
            },
            0xFEA0..= 0xFEFF => {
                todo!("I/O registers")
            },
            0xFF00..= 0xFF7F => {
                todo!("Sprite table not implemented");
            },
            0xFF80..= 0xFFFE => {
                self.highRam[((addr & 0x00ff) - 0x0080) as usize] = data;
            },
            0xFFFF => {
                self.interruptRegister = data;
            }
        }
    }
}

/*
0000 	3FFF 	16 KiB ROM bank 00 	From cartridge, usually a fixed bank
4000 	7FFF 	16 KiB ROM Bank 01~NN 	From cartridge, switchable bank via mapper (if any)
8000 	9FFF 	8 KiB Video RAM (VRAM) 	In CGB mode, switchable bank 0/1
A000 	BFFF 	8 KiB External RAM 	From cartridge, switchable bank if any
C000 	CFFF 	4 KiB Work RAM (WRAM) 	
D000 	DFFF 	4 KiB Work RAM (WRAM) 	In CGB mode, switchable bank 1~7
E000 	FDFF 	Mirror of C000~DDFF (ECHO RAM) 	Nintendo says use of this area is prohibited.
FE00 	FE9F 	Sprite attribute table (OAM) 	
FEA0 	FEFF 	Not Usable 	Nintendo says use of this area is prohibited
FF00 	FF7F 	I/O Registers 	
FF80 	FFFE 	High RAM (HRAM) 	
FFFF 	FFFF 	Interrupts Enable Register (IE)
*/