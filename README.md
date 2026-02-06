# gbr [![CI](https://github.com/keichi/gbr/actions/workflows/ci.yml/badge.svg)](https://github.com/keichi/gbr/actions/workflows/ci.yml)

Yet another Game Boy emulator in Rust.

![Mario](https://raw.githubusercontent.com/keichi/gbr/master/images/mario.png)

## Prerequisites

- Rust 1.31.1
- SDL2

## Status

- [x] CPU
    - [x] Instructions
    - [x] Instruction timing
    - [x] Interrupt handling
- [ ] PPU
    - [x] Background
    - [x] Window
    - [x] Sprite
    - [x] V-blank interrupt
    - [x] LCDC STAT interrupt
    - [x] Sprite and background priority
    - [ ] OAM bug
- [x] Joypad
    - [x] Joypad input
    - [x] Joypad interrupt
- [ ] Catridge
    - [x] Catridge loading
    - [x] Data
    - [x] MBC1
    - [ ] MBC3
    - [ ] MBC5
    - [ ] External RAM persistence
- [x] Timer
    - [x] Timer registers
    - [x] Timer overflow interrupt
- [ ] APU
