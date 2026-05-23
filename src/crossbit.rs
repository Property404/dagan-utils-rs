//! Crossbit - combine files with boolean operator
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};

const PAGE_SIZE: usize = 4096;

/// Crossbit - combine files with boolean operator
#[derive(Parser)]
struct Args {
    operator: Operator,
    file1: PathBuf,
    file2: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Operator {
    And,
    Xor,
    Or,
}

fn crossbit(
    operator: Operator,
    file1: impl Read,
    file2: impl Read,
    mut out: impl Write,
) -> Result<()> {
    let file1 = file1.bytes();
    let file2 = file2.bytes();
    for pair in file1.zip(file2) {
        let in1 = pair.0?;
        let in2 = pair.1?;
        let byte = match operator {
            Operator::And => in1 & in2,
            Operator::Or => in1 | in2,
            Operator::Xor => in1 ^ in2,
        };
        out.write_all(&[byte])?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let file1 = File::open(args.file1)?;
    let stdout = io::stdout().lock();

    if let Some(file2) = args.file2 {
        let file2 = File::open(file2)?;
        crossbit(args.operator, file1, file2, stdout)?;
    } else {
        crossbit(args.operator, file1, io::stdin().lock(), stdout)?;
    }
    Ok(())
}
