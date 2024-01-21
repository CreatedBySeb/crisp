extern crate sdl2;

mod audio;
mod font;

use audio::get_audio_device;
use getrandom::getrandom;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::env;
use std::fs;
use std::time::Duration;
use std::time::Instant;

use crate::font::get_char_address;
use crate::font::load_font;

const PROGRAM_OFFSET: usize = 0x200;
const MEMORY_SIZE: usize = 4096;
const PROGRAM_MAX_SIZE: usize = MEMORY_SIZE - PROGRAM_OFFSET;
const INSTRUCTION_FREQ: u32 = 700;
const SCREEN_HEIGHT: u8 = 32;
const SCREEN_WIDTH: u8 = 64;
const SCALING_FACTOR: u8 = 10;
const PIXEL_OFF: Color = Color::RGB(0, 0, 0);
const PIXEL_ON: Color = Color::RGB(255, 255, 255);
const TIMER_FREQ: u32 = 60;

const KEYS: [Scancode; 16] = [
    Scancode::X,
    Scancode::Num1,
    Scancode::Num2,
    Scancode::Num3,
    Scancode::Q,
    Scancode::W,
    Scancode::E,
    Scancode::A,
    Scancode::S,
    Scancode::D,
    Scancode::Z,
    Scancode::C,
    Scancode::Num4,
    Scancode::R,
    Scancode::F,
    Scancode::V,
];

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
    let mut delay_timer = 0_u8;
    let mut sound_timer = 0_u8;
    let mut pc = PROGRAM_OFFSET;
    let mut sp = 0_u8;
    let mut stack: [usize; 16] = [0; 16];
    let mut last_tick = Instant::now();
    let tick_interval = Duration::new(0, 1_000_000_000u32 / TIMER_FREQ);

    load_font(&mut memory);

    let args: Vec<String> = env::args().collect();
    let rom_path = &args[1];
    read_rom(&mut memory, rom_path);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

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

    let audio_device = get_audio_device(audio_subsystem);

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut key_states = 0_u16;

    'running: loop {
        let mut keys_pressed = 0_u16;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

                Event::KeyDown {
                    scancode: Some(code),
                    repeat: false,
                    ..
                } => match KEYS.iter().position(|key| *key == code) {
                    Some(key) => {
                        let mask = 1 << key;
                        key_states ^= mask;
                        keys_pressed ^= mask;
                    }

                    None => {}
                },

                Event::KeyUp {
                    scancode: Some(code),
                    repeat: false,
                    ..
                } => match KEYS.iter().position(|key| *key == code) {
                    Some(key) => {
                        let mask = !(1 << key);
                        key_states &= mask;
                    }

                    None => {}
                },

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
                    sp -= 1;
                    pc = stack[sp as usize];
                } else {
                    println!("SYS calls are no-ops");
                }
            }

            0x1 | 0x2 => {
                if instruction == 0x2 {
                    stack[sp as usize] = pc;
                    sp += 1;
                }

                pc = (usize::from(X) << 8) + usize::from(bytes[1]);
                advance = false;
            }

            0x3 => {
                if registers[X as usize] == bytes[1] {
                    pc += 2;
                }
            }

            0x4 => {
                if registers[X as usize] != bytes[1] {
                    pc += 2;
                }
            }

            0x5 => {
                if registers[X as usize] == registers[Y as usize] {
                    pc += 2;
                }
            }

            0x6 => {
                registers[X as usize] = bytes[1];
            }

            0x7 => {
                let (new_value, overflow) = registers[X as usize].overflowing_add(bytes[1]);
                registers[X as usize] = new_value;
                registers[0xf] = overflow.into();
            }

            0x8 => match N {
                0x0 => {
                    registers[X as usize] = registers[Y as usize];
                }

                0x1 => {
                    registers[X as usize] |= registers[Y as usize];
                }

                0x2 => {
                    registers[X as usize] &= registers[Y as usize];
                }

                0x3 => {
                    registers[X as usize] ^= registers[Y as usize];
                }

                0x4 => {
                    let (new_value, overflow) =
                        registers[X as usize].overflowing_add(registers[Y as usize]);

                    registers[X as usize] = new_value;
                    registers[0xf] = overflow.into();
                }

                0x5 => {
                    let (new_value, overflow) =
                        registers[X as usize].overflowing_sub(registers[Y as usize]);

                    registers[X as usize] = new_value;
                    registers[0xf] = (!overflow).into();
                }

                0x6 => {
                    registers[0xf] = registers[X as usize] & 1;
                    registers[X as usize] >>= 1;
                }

                0x7 => {
                    let (new_value, overflow) =
                        registers[Y as usize].overflowing_sub(registers[X as usize]);

                    registers[X as usize] = new_value;
                    registers[0xf] = (!overflow).into();
                }

                0xe => {
                    registers[0xf] = (registers[X as usize].leading_ones() > 0).into();
                    registers[X as usize] <<= 1;
                }

                _ => {
                    panic!("N is invalid ({:x})", N);
                }
            },

            0x9 => {
                if registers[X as usize] != registers[Y as usize] {
                    pc += 2;
                }
            }

            0xa => {
                index = (usize::from(X) << 8) + usize::from(bytes[1]);
            }

            0xb => {
                pc = (usize::from(X) << 8) + usize::from(bytes[1]) + registers[0] as usize;
                advance = false;
            }

            0xc => {
                let mut value: [u8; 1] = [0];
                getrandom(&mut value).ok();
                registers[X as usize] = value[0] & bytes[1];
            }

            0xd => {
                // TODO: Support clipping sprites instead of wrapping them as an option
                let sprite = &memory[index..index + N as usize];
                let draw_x = registers[X as usize];
                let draw_y = registers[Y as usize] as usize;
                let mut collision: bool = false;

                for i in 0..N as usize {
                    if (draw_y as usize + i) > 31 {
                        break;
                    }

                    let mut row = u64::from(sprite[i]) << 56;
                    row = row.rotate_right(draw_x.into());
                    collision |= (framebuffer[draw_y + i] & row) != 0;
                    framebuffer[draw_y + i] ^= row;
                }

                registers[0xf] = collision.into();
            }

            0xe => {
                let mut value = (key_states & (1 << registers[X as usize])) != 0;

                if bytes[1] == 0xa1 {
                    value = !value;
                } else if bytes[1] != 0x9e {
                    println!("Invalid key mode {:02x}", bytes[1]);
                    return;
                }

                if value {
                    pc += 2;
                }
            }

            0xf => match bytes[1] {
                0x07 => {
                    registers[X as usize] = delay_timer;
                }

                0x0a => {
                    if keys_pressed == 0 {
                        advance = false;
                    } else {
                        registers[X as usize] = (0..16_u8)
                            .find(|key| (keys_pressed & (1 << key)) != 0)
                            .unwrap();
                    }
                }

                0x15 => {
                    delay_timer = registers[X as usize];
                }

                0x18 => {
                    sound_timer = registers[X as usize];

                    if sound_timer != 0 {
                        audio_device.resume();
                    }
                }

                0x1e => {
                    let value = (index + registers[X as usize] as usize) & 0xfff;
                    registers[0xf] = (value < index).into();
                    index = value;
                }

                0x29 => {
                    index = get_char_address(registers[X as usize]);
                }

                0x33 => {
                    let value = registers[X as usize];
                    memory[index] = value / 100;
                    memory[index + 1] = (value % 100) / 10;
                    memory[index + 2] = value % 10;
                }

                0x55 => {
                    for i in 0..=X as usize {
                        memory[index + i] = registers[i];
                    }
                }

                0x65 => {
                    for i in 0..=X as usize {
                        registers[i] = memory[index + i];
                    }
                }

                _ => {
                    panic!("Sub-instruction is invalid ({:02x})", bytes[1]);
                }
            },

            _ => {
                unimplemented!("Instruction {:x} not implemented", instruction);
            }
        }

        if advance {
            pc += 2;
        }

        if last_tick.elapsed() >= tick_interval {
            if delay_timer != 0 {
                delay_timer -= 1;
            }

            if sound_timer != 0 {
                sound_timer -= 1;

                if sound_timer == 0 {
                    audio_device.pause();
                }
            }

            last_tick = Instant::now();
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
