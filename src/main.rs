#![allow(non_snake_case)]
#![allow(dead_code)]
mod cpu;
mod bit;
mod bus;
mod visualizer;
extern crate sfml;
use sfml::{
    graphics::{
        Text, RenderTarget, RenderWindow, Color, Font
    },
    window::{ContextSettings, Event, Key, Style}
};
use std::{thread, time};


fn main() {
    let font = Font::from_file("fonts/RobotoMono-Medium.ttf").unwrap();
    
    let mut c = cpu::Z80::new();

    c.pc = 0xC000;

    c.writeByte(0xFF07, 0b101);

    c.writeByte(0xC000, 0x01);
    c.writeByte(0xC001, 0xff);
    c.writeByte(0xC002, 0x01);
    c.writeByte(0xC003, 0x03);

    let mut window = RenderWindow::new((1280, 720),
            "GBA Emulator - Badjaba",
            Style::CLOSE,
            &ContextSettings::default());

    window.set_framerate_limit(30);
    //window.draw(&t);
    let mut ramPage1 = 0xC000;
    let mut ramPage2 = 0xD000;
    


    while window.is_open() {
        while let Some(event) = window.poll_event() {
            match event {
                Event::Closed | Event::KeyPressed {code: Key::ESCAPE, ..} => return,
                Event::KeyPressed {code: Key::SPACE, ..} => {
                    c.clock();
                },
                //Event::KeyPressed {code: Key::R, ..} => {c.executeOpcode(0xc1);},
                Event::KeyPressed {code: Key::DOWN, ..} => {
                    if ramPage2 + 18 * 32 < 0xFFFF {ramPage2 += 32} else {}; 
                },
                Event::KeyPressed {code: Key::UP, ..} => {
                    if ramPage2 - 32 > 0x0 {ramPage2 -= 32} else {};
                },
                /*
                Event::KeyPressed {code: Key::S, ..} => c.registers.setFlag(!c.registers.getFlag(Flags::S), Flags::S),
                Event::KeyPressed {code: Key::Z, ..} => c.registers.setFlag(!c.registers.getFlag(Flags::Z), Flags::Z),
                Event::KeyPressed {code: Key::A, ..} => c.registers.setFlag(!c.registers.getFlag(Flags::A), Flags::A),
                Event::KeyPressed {code: Key::P, ..} => c.registers.setFlag(!c.registers.getFlag(Flags::P), Flags::P),
                Event::KeyPressed {code: Key::C, ..} => c.registers.setFlag(!c.registers.getFlag(Flags::C), Flags::C),
                */
                _ => {}
            }
        }
        window.clear(Color::BLUE);
        visualizer::renderFullDissassembly(&c, ramPage1, ramPage2, &font, &mut window);
        window.display();
    }
}


