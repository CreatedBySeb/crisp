extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::env;
use std::fs;
use std::time::Duration;

const PROGRAM_OFFSET: usize = 0x200;
const MEMORY_SIZE: usize = 4096;
const PROGRAM_MAX_SIZE: usize = MEMORY_SIZE - PROGRAM_OFFSET;
const INSTRUCTION_FREQ: u32 = 700;
const SCREEN_HEIGHT: u8 = 32;
const SCREEN_WIDTH: u8 = 64;
const SCALING_FACTOR: u8 = 10;
const PIXEL_OFF: Color = Color::RGB(0, 0, 0);
const PIXEL_ON: Color = Color::RGB(255, 255, 255);

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

fn split_nibbles(byte: u8) -> (u8, u8) {
    (byte >> 4, byte & 15)
}

fn main() {
    let mut memory: [u8; MEMORY_SIZE] = [0; MEMORY_SIZE];
    let mut framebuffer: [u64; SCREEN_HEIGHT as usize] = [0; SCREEN_HEIGHT as usize];
    let mut registers: [u8; 16] = [0; 16];
    let mut index: usize = 0;
    let mut pc = PROGRAM_OFFSET;

    let args: Vec<String> = env::args().collect();
    let rom_path = &args[1];
    read_rom(&mut memory, rom_path);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window(
            "crust",
            u32::from(SCREEN_WIDTH) * SCALING_FACTOR as u32,
            u32::from(SCREEN_HEIGHT) * SCALING_FACTOR as u32,
        )
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(PIXEL_OFF);
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

        let mut advance = true;
        let bytes: [u8; 2] = [memory[pc], memory[pc + 1]];
        let (instruction, X) = split_nibbles(bytes[0]);
        let (Y, N) = split_nibbles(bytes[1]);
        println!("[{:#04x}] {:02x} {:02x}", pc, bytes[0], bytes[1]);

        match instruction {
            0x0 => {
                if bytes[1] == 0xe0 {
                    framebuffer.fill(0x0);
                } else if bytes[1] == 0xee {
                    unimplemented!("RET not implemented");
                } else {
                    println!("SYS calls are no-ops");
                }
            }

            0x1 => {
                pc = (usize::from(X) << 8) + usize::from(bytes[1]);
                advance = false;
            }

            0x6 => {
                registers[X as usize] = bytes[1];
            }

            0x7 => {
                registers[X as usize] += bytes[1];
            }

            0xa => {
                index = (usize::from(X) << 8) + usize::from(bytes[1]);
            }

            0xd => {
                // TODO: Support clipping sprites instead of wrapping them as an option
                let sprite = &memory[index..index + N as usize];
                let draw_x = registers[X as usize];
                let draw_y = registers[Y as usize] as usize;
                let mut collision: bool = false;

                for i in 0..N as usize {
                    let mut row = u64::from(sprite[i]) << 56;
                    row = row.rotate_right(draw_x.into());
                    collision |= (framebuffer[draw_y + i] & row) != 0;
                    framebuffer[draw_y + i] ^= row;
                }

                registers[0xf] = collision.into();
            }

            _ => {
                unimplemented!("Instruction {:x} not implemented", instruction);
            }
        }

        if advance {
            pc += 2;
        }

        canvas.set_draw_color(PIXEL_OFF);
        canvas.clear();
        canvas.set_draw_color(PIXEL_ON);

        let rows = (0..SCREEN_HEIGHT).filter(|x| framebuffer[*x as usize] != 0);

        for i in rows {
            let y = i32::from(i);
            let pixels =
                (0..SCREEN_WIDTH).filter(|x| (framebuffer[i as usize] & (1 << (63 - x))) != 0);

            for x in pixels {
                canvas.fill_rect(Rect::new(
                    i32::from(x) * SCALING_FACTOR as i32,
                    y * SCALING_FACTOR as i32,
                    SCALING_FACTOR as u32,
                    SCALING_FACTOR as u32,
                ));
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / INSTRUCTION_FREQ));
    }
}
