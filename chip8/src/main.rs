use chip8_system::port::connect;
use chip8_system::system::{Quirks, System, SystemOptions};
use clap::Parser;
use gui_druid::keyboard_map::load_profiles;
use gui_druid::{Color, ColorParseError, Terminal, TerminalOptions};
use sound_cpal::Beeper;
use std::error::Error;
use std::path::PathBuf;
use std::thread;

#[derive(Parser)]
struct Options {
    /// Set CPU frequency (> 0 and < 5000 Hz)
    #[clap(long, short)]
    cpu_frequency: Option<f64>,

    /// Set background color for the gui (hex HTML-like RGB color value)
    #[clap(long, short, value_parser = parse_color)]
    bg_color: Option<Color>,

    /// Set foreground color for the gui (hex HTML-like RGB color value)
    #[clap(long, short, value_parser = parse_color)]
    fg_color: Option<Color>,

    /// Set profile mapping physical to virtual keyboard (supported profiles: default, qwerty, azerty)
    #[clap(long, short)]
    kb_profile: Option<String>,

    /// Load and store instructions do not increment the I register
    #[clap(long, short, help_heading(Some("QUIRKS")))]
    load_store_ignores_i: bool,

    /// Shift operations read the VX register instead of VY
    #[clap(long, short, help_heading(Some("QUIRKS")))]
    shift_reads_vx: bool,

    /// Draw operations wrap pixels around the edges of the screen
    #[clap(long, short, help_heading(Some("QUIRKS")))]
    draw_wraps_pixels: bool,

    /// Set input filename of the image to run
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
    if options.draw_wraps_pixels {
        sys_opts.quirk(Quirks::DRAW_WRAPS_PIXELS);
    }

    let mut system = System::new_with_options(sys_opts)?;
    let beeper = Beeper::new()?;
    connect(&system.sound_timer, &beeper);

    // terminal options
    let mut term_opts = TerminalOptions::new();
    if let Some(c) = options.bg_color {
        term_opts.background_color(c);
    }
    if let Some(c) = options.fg_color {
        term_opts.foreground_color(c);
    }
    if let Some(profile) = options.kb_profile {
        let mut profiles = load_profiles();
        if let Some(km) = profiles.remove(&profile) {
            term_opts.keyboard_map(km);
        }
    }

    let term = Terminal::new_with_options(term_opts);

    // connect term output to system input
    connect(&term, &system.keyboard);

    // connect system output to term input
    connect(&system.display, &term);

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
