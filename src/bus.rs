use super::bit;
use super::timer::{Timers};
pub struct Bus {
    ram1: [u8; 4 * 1024],
    ram2: [u8; 4 * 1024],
    highRam: [u8; 127],

    pub interruptEnableRegister: u8,
    pub interruptRequestRegister: u8,
    pub timerRegisters: Timers
}

pub enum IntrFlags {
    VBlank = 0,
    LCD = 1,
    Timer = 2,
    Serial = 3,
    Joypad = 4
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram1: [0; 4 * 1024],
            ram2: [0; 4 * 1024],
            highRam: [0; 127],

            interruptEnableRegister: 0,
            interruptRequestRegister: 0,
            timerRegisters: Timers::new(),
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
            0xC000..= 0xCFFF => {self.ram1[(addr & 0x0fff) as usize]},
            0xD000..= 0xDFFF => {self.ram2[(addr & 0x0fff) as usize]},
            0xE000..= 0xFDFF => {
                if addr <= 0xEFFF {
                    self.ram1[(addr & 0x0fff) as usize]
                } else {
                    self.ram2[(addr & 0x0fff) as usize]
                }
            },
            0xFE00..= 0xFE9F => {
                todo!("Sprite table not implemented");
            },
            0xFEA0..= 0xFEFF => {
                panic!("Unusable memory")
            },
            0xFF00..= 0xFF7F => {
                match addr & 0x00FF {
                    0x00 => {todo!("Controller not implemented")},
                    0x01..= 0x02 => {todo!("Communication not implemented")},
                    0x04..= 0x07 => {
                        match addr & 0x000F {
                            0x4 => {((self.timerRegisters.divRegister & 0xFF00) >> 8) as u8},
                            0x5 => {self.timerRegisters.timaRegister},
                            0x6 => {self.timerRegisters.tmaRegister},
                            0x7 => {self.timerRegisters.tacRegister},
                            _ => {0}
                        }},
                    0x0F => {self.interruptRequestRegister},
                    0x10..= 0x26 => {/* Sound, not implementing*/0},
                    0x30..= 0x3F => {/* Waveform RAM, not implementing*/0},
                    0x40..= 0x4B => {todo!("LCD register not implemented")},
                    0x4F => {/* GBC VRAM Bank Select */0},
                    0x50 => {/* Set to disable boot ROM ??*/0},
                    0x51..= 0x55 => {/* GBC HDMA */0},
                    0x68..= 0x69 => {/* GBC BCP/OCP */0},
                    0x70 => {/* GBC WRAM Bank Select */0}
                    _ => {panic!("Unknown write to {}", addr)}
                }
            },
            0xFF80..= 0xFFFE => {
                self.highRam[((addr & 0x00ff) - 0x0080) as usize]
            },
            0xFFFF => {
                self.interruptEnableRegister
            }
        }
    }

    pub fn cpuWrite(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..= 0x3FFF => {panic!("Tried to write to ROM")},
            0x4000..= 0x7FFF => {panic!("Tried to write to ROM")},
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
                todo!("Sprite table not implemented");
            },
            0xFEA0..= 0xFEFF => {
                panic!("Unusable memory");
            },
            0xFF00..= 0xFF7F => {
                match addr & 0x00FF {
                    0x00 => {todo!("Controller not implemented")},
                    0x01..= 0x02 => {todo!("Communication not implemented")},
                    0x04..= 0x07 => {
                        match addr & 0x000F {
                            0x4 => {self.timerRegisters.divRegister = 0},
                            0x5 => {self.timerRegisters.timaWrite(data);},
                            0x6 => {self.timerRegisters.tmaWrite(data)},
                            0x7 => {self.timerRegisters.tacRegister = data},
                            _ => {}
                        }},
                    0x0F => {self.interruptRequestRegister = data},
                    0x10..= 0x26 => {/* Sound, not implementing*/},
                    0x30..= 0x3F => {/* Waveform RAM, not implementing*/},
                    0x40..= 0x4B => {todo!("LCD register not implemented")},
                    0x4F => {/* GBC VRAM Bank Select */},
                    0x50 => {/* Set to disable boot ROM ??*/},
                    0x51..= 0x55 => {/* GBC HDMA */},
                    0x68..= 0x69 => {/* GBC BCP/OCP */},
                    0x70 => {/* GBC WRAM Bank Select */}
                    _ => {panic!("Unknown write to {}", addr)}
                }
            },
            0xFF80..= 0xFFFE => {
                self.highRam[((addr & 0x00ff) - 0x0080) as usize] = data;
            },
            0xFFFF => {
                self.interruptEnableRegister = data;
            }
        }
    }

    pub fn requestInterrupt(&mut self, i: IntrFlags) {
        self.interruptRequestRegister = bit::set(self.interruptRequestRegister, i as usize);
    }

    pub fn getInterruptRequest(&self, i: IntrFlags) -> bool {
        bit::get(self.interruptRequestRegister, i as usize)
    }

    pub fn resetInterruptRequest(&mut self, i: IntrFlags) {
        self.interruptRequestRegister = bit::clr(self.interruptRequestRegister, i as usize);
    }

    pub fn getInterruptEnable(&self, i: IntrFlags) -> bool{
        bit::get(self.interruptEnableRegister, i as usize)
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