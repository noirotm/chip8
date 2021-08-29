use crate::generator::generate;
use crate::parser::parse_file;
use std::env::args;
use std::error::Error;
use std::fs::File;

mod ast;
mod generator;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = args();
    let input_file = args.nth(1).ok_or("missing input filename")?;
    let output_file = args.next().ok_or("missing output filename")?;
    let r = parse_file(&input_file)?;

    // println!("{:?}", &r);

    let mut output = File::create(&output_file)?;
    generate(&r, &mut output)?;

    Ok(())
}
