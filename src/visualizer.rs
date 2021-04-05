#![allow(non_snake_case)]
extern crate sfml;
use crate::cpu::{Z80, Flags};
use sfml::{
    graphics::{
        Text, RenderTarget, RenderWindow, Color, Font, Transformable
    },
};

const CHAR_SIZE: u32 = 14;

pub fn showRam(c: &Z80, startIndex: u16, nRows: u16, nCols: u16) -> String {
    let mut nStr = String::new();
    let mut n: u16 = 0;
    for _i in 0..nRows {
        let addr = startIndex + n;
        nStr.push_str(&format!("{:#06X}:\t", addr));
        for j in 0..nCols {
            nStr.push_str(&format!("{:02x} ", c.readByte(addr + (j as u16))));
            n += 1;
        }
        nStr.push('\n');
    }
    return nStr;
}

pub fn showRegisters(c: &Z80) -> String {
    let mut nStr = String::new();
    let mut nStr2 = String::new();
    let mut i = 0;
    nStr.push_str(&format!("A: {:02x} ", c.a));
    nStr2.push_str(&format!("F: {:02x} ", c.f));
    nStr.push_str(&format!("B: {:02x} ", c.b));
    nStr2.push_str(&format!("C: {:02x} ", c.c));
    nStr.push_str(&format!("D: {:02x} ", c.d));
    nStr2.push_str(&format!("E: {:02x} ", c.e));
    nStr.push_str(&format!("H: {:02x} ", c.h));
    nStr2.push_str(&format!("L: {:02x} ", c.l));
    nStr.push_str(&format!("SP: {:02x} ", c.sp));
    nStr2.push_str(&format!("PC: {:02x} ", c.pc));

    nStr.push('\n');
    nStr.push_str(&nStr2);
    return nStr;
}

/*
pub fn showCode(c: &Cpu, startIndex: u16, nInstructions: u16) -> String{
    let mut nStr = String::new();
    let mut opcodeLen = 0;
    let mut addr = startIndex;
    for _i in 0..nInstructions {
        addr += opcodeLen;
        nStr.push_str(&format!("{:#06X}\t", addr));
        match c.dissasemble(addr) {
            (x, y, _) => {
                nStr.push_str(&x);
                opcodeLen = y.into()
            }
        }
        nStr.push('\n');
    }
    return nStr;
}
*/

pub fn renderFullDissassembly(c: &Z80, ramPage1: u16, ramPage2: u16, f: &Font, w: &mut RenderWindow) {
    let mut ramText = Text::default();
    ramText.set_font(f);
    ramText.set_string(&showRam(c, ramPage1, 9, 12));
    ramText.set_character_size(CHAR_SIZE * 2);
    ramText.set_fill_color(Color::WHITE);

    let mut ramText2 = Text::default();
    ramText2.set_font(f);
    ramText2.set_string(&showRam(c, ramPage2, 9, 12));
    ramText2.set_character_size(CHAR_SIZE * 2);
    ramText2.set_fill_color(Color::WHITE);
    ramText2.set_position((ramText.position().x, ramText.local_bounds().height + 20.0));

    let mut tArr: [Text; 8] = [
    Text::new("Z ", f, CHAR_SIZE * 2),
    Text::new("N ", f, CHAR_SIZE * 2),
    Text::new("HC ", f, CHAR_SIZE * 2),
    Text::new("C ", f, CHAR_SIZE * 2),
    Text::new("0 ", f, CHAR_SIZE * 2),
    Text::new("0 ", f, CHAR_SIZE * 2),
    Text::new("0 ", f, CHAR_SIZE * 2),
    Text::new("0", f, CHAR_SIZE * 2),
    ];
    if c.getFlag(Flags::Zero) {tArr[0].set_fill_color(Color::GREEN)} else {tArr[0].set_fill_color(Color::RED)};
    if c.getFlag(Flags::Sub) {tArr[1].set_fill_color(Color::GREEN)} else {tArr[1].set_fill_color(Color::RED)};
    if c.getFlag(Flags::HCarry) {tArr[2].set_fill_color(Color::GREEN)} else {tArr[2].set_fill_color(Color::RED)};
    if c.getFlag(Flags::Carry) {tArr[3].set_fill_color(Color::GREEN)} else {tArr[3].set_fill_color(Color::RED)};
    tArr[0].set_position((ramText.local_bounds().width + 20.0, ramText.position().y));
    for i in 1..tArr.len() {
        tArr[i].set_position((tArr[i - 1].local_bounds().width + tArr[i - 1].position().x, tArr[i - 1].position().y));
    }


    let mut registerText = Text::default();
    registerText.set_font(f);
    registerText.set_string(&showRegisters(c));
    registerText.set_character_size(CHAR_SIZE + 6);
    registerText.set_fill_color(Color::WHITE);
    registerText.set_position((tArr[0].position().x, tArr[0].local_bounds().height + 20.0));

    /*
    let mut codeText = Text::default();
    codeText.set_font(f);
    codeText.set_string(&showCode(c, c.registers.pc, 30));
    codeText.set_character_size(CHAR_SIZE);
    codeText.set_fill_color(Color::WHITE);
    codeText.set_position((registerText.position().x, registerText.local_bounds().height + registerText.position().y + 20.0));

    
    let mut registerBinaryText = Text::default();
    registerBinaryText.set_font(f);
    registerBinaryText.set_string(&showRegistersBinary(c));
    registerBinaryText.set_character_size(CHAR_SIZE);
    registerBinaryText.set_fill_color(Color::WHITE);
    registerBinaryText.set_position((codeText.position().x + codeText.local_bounds().width + 20.0, codeText.position().y));
    */
    w.draw(&ramText);
    w.draw(&ramText2);
    for e in tArr.iter() {
        w.draw(e);
    }
    w.draw(&registerText);
    /*
    w.draw(&codeText);
    w.draw(&registerBinaryText);
    */
}