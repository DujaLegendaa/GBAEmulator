use std::fs;

#[derive(Debug)]
pub struct Cartridge {
    data: Vec<u8>,
    gbcFlag: u8,
    cartType: CartridgeType,
    ramType: RamType,

    romSize: u8,

    ram: Option<Vec<u8>>,
}

#[derive(Debug)]
enum CartridgeType {
    Rom = 0x00,
    Mbc1 = 0x01,
    Mbc1Ram = 0x02,
    Mbc1RamBattery = 0x03,
}

#[derive(Debug)]
enum RamType {
    NoRam = 0x00,
    Unused = 0x01,
    Kb8 = 0x02,
    Kb32 = 0x03,
    Kb128 = 0x04,
    Kb64 = 0x05
}

impl Cartridge {
    pub fn new(path: String) -> Self {
        let d = fs::read(path).unwrap();
        let gbcF = d[0x0143];
        let t = match d[0x0147] {
            0x00 => CartridgeType::Rom,
            0x01 => CartridgeType::Mbc1,
            0x02 => CartridgeType::Mbc1Ram,
            0x03 => CartridgeType::Mbc1RamBattery,
            _ => panic!("Unsupported cartridge type")
        };
        let (ram, rT) = match d[0x0149] {
            0x00 => (None, RamType::NoRam),
            0x01 => (None, RamType::Unused),
            0x02 => (Some(Vec::with_capacity(8192)), RamType::Kb8),
            0x03 => (Some(Vec::with_capacity(8192 * 4)), RamType::Kb32),
            0x04 => (Some(Vec::with_capacity(8192 * 16)), RamType::Kb128),
            0x05 => (Some(Vec::with_capacity(8192 * 8)), RamType::Kb64),
            _ => panic!("Unreachable"),
        };

        let rSize = d[0x0148];
        Self {
            data: d,
            gbcFlag: gbcF,
            cartType: t,
            ramType: rT,

            romSize: rSize,

            ram: ram
        }
    }

    pub fn readRom(&self, addr: u16) -> u8 {
        match self.cartType {
            CartridgeType::Rom => {self.data[addr as usize]},
            _ => panic!("Mappers not implemented")
        }
    }

    pub fn readRam(&self, addr: u16) -> u8 {
        match self.cartType {
            CartridgeType::Rom => {0},
            _ => panic!("Mappers not implemented")
        }
    }

    pub fn writeRam(&mut self, addr: u16, d: u8) {
        todo!("Not implemented")
    }
}

/*
$00 	ROM ONLY
$01 	MBC1
$02 	MBC1+RAM
$03 	MBC1+RAM+BATTERY
$05 	MBC2
$06 	MBC2+BATTERY
$08 	ROM+RAM *
$09 	ROM+RAM+BATTERY *
$0B 	MMM01
$0C 	MMM01+RAM
$0D 	MMM01+RAM+BATTERY
$0F 	MBC3+TIMER+BATTERY
$10 	MBC3+TIMER+RAM+BATTERY
$11 	MBC3
$12 	MBC3+RAM
$13 	MBC3+RAM+BATTERY
$19 	MBC5
$1A 	MBC5+RAM
$1B 	MBC5+RAM+BATTERY
$1C 	MBC5+RUMBLE
$1D 	MBC5+RUMBLE+RAM
$1E 	MBC5+RUMBLE+RAM+BATTERY
$20 	MBC6
$22 	MBC7+SENSOR+RUMBLE+RAM+BATTERY
$FC 	POCKET CAMERA
$FD 	BANDAI TAMA5
$FE 	HuC3
$FF 	
*/

/*
$00 	0 	No RAM *
$01 	- 	Unused **
$02 	8 KB 	1 bank
$03 	32 KB 	4 banks of 8 KB each
$04 	128 KB 	16 banks of 8 KB each
$05 	64 KB 	8 banks of 8 KB each
*/

/*
$00 	32 KByte 	2 (No ROM banking)
$01 	64 KByte 	4
$02 	128 KByte 	8
$03 	256 KByte 	16
$04 	512 KByte 	32
$05 	1 MByte 	64
$06 	2 MByte 	128
$07 	4 MByte 	256
$08 	8 MByte 	512
*/