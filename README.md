# chip8 - CHIP-8 Emulator

This is a CHIP-8 emulator written in Rust.
The goal of this project is to write a composable CHIP-8 system without
a hard dependency to the display and sound backends.

This is achieved through the use of a IO port abstraction where the main
system is connected to external virtual components (keyboard, screen, beeper).

![Architecture Diagram](doc/architecture.png)

It is a work in progress.
