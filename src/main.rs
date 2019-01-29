use std::env;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

mod cpu;
mod io_device;
mod mmu;
mod ppu;
mod timer;

fn main() {
    env_logger::init();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rgb", 320, 288)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 160, 144)
        .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut mmu = mmu::MMU::new();

    let args: Vec<String> = env::args().collect();
    mmu.load_boot_rom("dmg_boot.bin");
    mmu.load_rom(&args[1]);

    let mut cpu = cpu::CPU::new(mmu);

    'running: loop {
        while cpu.mmu.ppu.mode() != 1 {
            cpu.step();
        }

        texture
            .with_lock(None, |buf: &mut [u8], pitch: usize| {
                let fb = cpu.mmu.ppu.frame_buffer();

                for y in 0..144 {
                    for x in 0..160 {
                        let offset = y * pitch + x * 3;
                        let color = fb[y * 160 + x];

                        buf[offset] = color;
                        buf[offset + 1] = color;
                        buf[offset + 2] = color;
                    }
                }
            })
            .unwrap();

        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

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

        while cpu.mmu.ppu.mode() == 1 {
            cpu.step();
        }
    }
}
