use chip8_system::port::connect;
use chip8_system::system::{Quirks, SystemOptions};
use chip8_system::timer::TimerMessage;
use chip8_system::System;
use clap::Clap;
use gui_druid::Terminal;
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
    #[clap(long, short, default_value = "500")]
    cpu_frequency: f64,

    /// Sets the input filename of the image to run
    filename: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options: Options = Options::parse();
    let mut sys_opts = SystemOptions::new();
    sys_opts.cpu_frequency_hz(options.cpu_frequency);

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

    let term = Terminal::new();

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
