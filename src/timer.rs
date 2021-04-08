use super::bit;
pub struct Timers {
    pub divRegister: u16,
    pub timaRegister: u8,
    pub tacRegister: u8,
    pub tmaRegister: u8,
    lastCycleBit: bool,

    pub timerOverflowDelay: u8,
    pub timaOverflow: bool,
    pub oldTMA: u8,
    pub tmaWriteCycle: bool,
}

impl Timers {
    pub fn new() -> Self {
        Self {
        divRegister: 0,
        timaRegister: 0,
        tacRegister: 0,
        tmaRegister: 0,
        lastCycleBit: false,

        timerOverflowDelay: 0,
        timaOverflow: false,
        oldTMA: 0,
        tmaWriteCycle: false,
        }
    }

    pub fn incrTimers(&mut self) -> bool {
        self.divRegister = self.divRegister.wrapping_add(1);
        let mut interruptRequest = false;
        if self.timaOverflow {
            self.timerOverflowDelay = (self.timerOverflowDelay + 1) % 5;
            if self.timerOverflowDelay == 4 {
                if self.tmaWriteCycle {
                    self.timaRegister = self.oldTMA;
                } else {
                    self.timaRegister = self.tmaRegister;
                }
                self.timaOverflow = false;
                interruptRequest = true;
            }
        }
        let mut timaBit = false;
        let timerEnable = bit::get(self.tacRegister, 2);
        match self.tacRegister & 0b11 {
            0b00 => {timaBit = bit::get16(self.divRegister, 9)},
            0b01 => {timaBit = bit::get16(self.divRegister, 3)},
            0b10 => {timaBit = bit::get16(self.divRegister, 5)},
            0b11 => {timaBit = bit::get16(self.divRegister, 7)},
            _ => {}
        }
        let currentCycleBit = timaBit && timerEnable;

        if currentCycleBit && !self.lastCycleBit {
            let (r, overflow) = self.timaRegister.overflowing_add(1);
            self.timaOverflow = overflow;
            self.timaRegister = r;
        }

        self.lastCycleBit = timaBit && timerEnable;
        self.tmaWriteCycle = false;
        interruptRequest
    }
}