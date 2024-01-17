extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::env;
use std::fs;
use std::time::Duration;

const PROGRAM_OFFSET: usize = 0x200;
const MEMORY_SIZE: usize = 4096;
const PROGRAM_MAX_SIZE: usize = MEMORY_SIZE - PROGRAM_OFFSET;

fn read_rom(memory: &mut [u8; MEMORY_SIZE], rom_path: &String) {
    let contents = fs::read(rom_path).unwrap();
    assert!(
        contents.len() <= PROGRAM_MAX_SIZE,
        "ROM exceeded max size {}B",
        PROGRAM_MAX_SIZE
    );

    let length = contents.len();
    memory[PROGRAM_OFFSET..PROGRAM_OFFSET + length].copy_from_slice(&contents);
    println!("Read in ROM of length {}B", length);
}

fn main() {
    let mut memory: [u8; MEMORY_SIZE] = [0; MEMORY_SIZE];
    let mut pc = PROGRAM_OFFSET;

    let args: Vec<String> = env::args().collect();
    let rom_path = &args[1];
    read_rom(&mut memory, rom_path);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("crust", 640, 320)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
