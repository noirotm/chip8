use chip8_system::port::connect;
use chip8_system::system::{Quirks, SystemOptions};
use chip8_system::timer::TimerMessage;
use chip8_system::System;
use clap::Clap;
use gui_druid::{Color, ColorParseError, Terminal, TerminalOptions};
use sound_cpal::Beeper;
use std::error::Error;
use std::path::PathBuf;
use std::thread;

#[derive(Clap)]
struct Options {
    /// Load and store instructions do not increment the I register
    #[clap(long, short)]
    load_store_ignores_i: bool,

    /// Shift operations read the VX register instead of VY
    #[clap(long, short)]
    shift_reads_vx: bool,

    /// Set CPU frequency (> 0 and < 5000 Hz)
    #[clap(long, short)]
    cpu_frequency: Option<f64>,

    /// Set background color for the gui (hex HTML-like RGB color value)
    #[clap(long, short, parse(try_from_str = parse_color))]
    bg_color: Option<Color>,

    /// Set foreground color for the gui (hex HTML-like RGB color value)
    #[clap(long, short, parse(try_from_str = parse_color))]
    fg_color: Option<Color>,

    /// Sets input filename of the image to run
    filename: PathBuf,
}

fn parse_color(s: &str) -> Result<Color, ColorParseError> {
    Color::from_hex_str(s)
}

fn main() -> Result<(), Box<dyn Error>> {
    let options: Options = Options::parse();

    // system options
    let mut sys_opts = SystemOptions::new();
    if let Some(f) = options.cpu_frequency {
        sys_opts.cpu_frequency_hz(f);
    }

    // Setup quirks
    if options.load_store_ignores_i {
        sys_opts.quirk(Quirks::LOAD_STORE_IGNORES_I);
    }
    if options.shift_reads_vx {
        sys_opts.quirk(Quirks::SHIFT_READS_VX);
    }

    let mut system = System::new_with_options(sys_opts)?;
    let beeper = Beeper::new()?;
    connect::<_, _, TimerMessage, _>(&system, &beeper);

    // terminal options
    let mut term_opts = TerminalOptions::new();
    if let Some(c) = options.bg_color {
        term_opts.background_color(c);
    }
    if let Some(c) = options.fg_color {
        term_opts.foreground_color(c);
    }

    let term = Terminal::new_with_options(term_opts);

    // connect term output to system input
    connect(&term, &system);

    // connect system output to term input
    connect(&system, &term);

    // load program to run
    let filename = options.filename;
    system.load_image(&filename)?;

    thread::spawn(move || {
        if let Err(e) = system.run() {
            println!("System Error: {}", e);
        }
    });
    term.run();

    Ok(())
}
