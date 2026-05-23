//! Crossbit - combine files with boolean operator
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::{
    fs::File,
    io::{self, BufReader, Read, Write},
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

impl Operator {
    const fn cross(self, bit1: u8, bit2: u8) -> u8 {
        match self {
            Operator::And => bit1 & bit2,
            Operator::Or => bit1 | bit2,
            Operator::Xor => bit1 ^ bit2,
        }
    }
}

fn crossbit(
    operator: Operator,
    file1: impl Read,
    file2: impl Read,
    mut out: impl Write,
) -> Result<()> {
    let file1 = BufReader::new(file1).bytes();
    let file2 = BufReader::new(file2).bytes();

    let mut index = 0;
    let mut buffer = [0; PAGE_SIZE];

    for pair in file1.zip(file2) {
        let byte = operator.cross(pair.0?, pair.1?);
        buffer[index] = byte;
        index += 1;
        if index >= PAGE_SIZE {
            out.write_all(&buffer)?;
            index = 0;
        }
    }
    out.write_all(&buffer[0..index])?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations() -> Result<()> {
        let tvs = [
            (
                Operator::And,
                vec![0b0110, 0b0001],
                vec![0b1010, 0b0101],
                vec![0b0010, 0b0001],
            ),
            (
                Operator::Or,
                vec![0b0110, 0b0001],
                vec![0b1010, 0b0101],
                vec![0b1110, 0b0101],
            ),
            (
                Operator::Xor,
                vec![0b0110, 0b0001],
                vec![0b1010, 0b0101],
                vec![0b1100, 0b0100],
            ),
        ];
        for tv in tvs {
            let mut out = Vec::with_capacity(tv.3.len());
            crossbit(tv.0, tv.1.as_slice(), tv.2.as_slice(), &mut out)?;
            assert_eq!(out, tv.3);
        }
        Ok(())
    }

    // Make sure we only produce the same number of bytes as the smaller of the two files
    #[test]
    fn size_diff() -> Result<()> {
        let tvs = [
            (
                Operator::And,
                vec![0b0110, 0b0001],
                vec![0b1010, 0b0101],
                vec![0b0010, 0b0001],
            ),
            (
                Operator::And,
                vec![0b0110],
                vec![0b1010, 0b0101],
                vec![0b0010],
            ),
            (
                Operator::And,
                vec![0b0110, 0b0001],
                vec![0b1010],
                vec![0b0010],
            ),
            (Operator::And, vec![0b0110, 0b0001], vec![], vec![]),
            (Operator::And, vec![], vec![], vec![]),
        ];
        for tv in tvs {
            let mut out = Vec::with_capacity(tv.3.len());
            crossbit(tv.0, tv.1.as_slice(), tv.2.as_slice(), &mut out)?;
            assert_eq!(out, tv.3);
        }
        Ok(())
    }
}
