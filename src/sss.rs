//! Standard Stream Split - duplicate stdin to both stdout and stderr
// TODO: alt names: speek? steek? ssp?
use anyhow::Result;
use clap::Parser;
use std::io::{self, Read, Write};

const PAGE_SIZE: usize = 4096;

/// Standard Stream Split - duplicate stdin to both stdout and stderr
#[derive(Parser)]
struct Args {}

fn main() -> Result<()> {
    let _args = Args::parse();
    let mut stdin = io::stdin().lock();
    let mut stderr = io::stderr().lock();
    let mut stdout = io::stdout().lock();
    let mut buf = [0u8; PAGE_SIZE];
    while let bytes = stdin.read(&mut buf)?
        && bytes != 0
    {
        stdout.write_all(&buf[0..bytes])?;
        stderr.write_all(&buf[0..bytes])?;
    }
    Ok(())
}
