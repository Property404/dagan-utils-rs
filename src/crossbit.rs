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
    /// The bitwise or bytewise operator to use
    operator: Operator,
    /// The files on which to operate. Use `-` to read from stdin
    #[clap(num_args=2..)]
    files: Vec<PathBuf>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Operator {
    // ~~~~ bitwise operators ~~~
    /// Bitwise AND
    And,
    /// Bitwise XOR
    Xor,
    /// Bitwise OR
    Or,
    /// Bitwise NAND
    Nand,
    /// Bitwise NOR
    Nor,
    /// Bitwise XNOR
    Xnor,

    // ~~~~ BYTEwise operators ~~~
    /// Bytewise wrapping add
    WrappingAdd,
    /// Bytewise saturating add
    SaturatingAdd,
    /// Bytewise absolute difference - that is subtracting the lesser byte from the greater byte
    AbsDiff,
    /// Choose the greater byte
    Greater,
    /// Choose the lesser byte
    Lesser,
}

impl Operator {
    fn cross(self, byte1: u8, byte2: u8) -> u8 {
        match self {
            Operator::And => byte1 & byte2,
            Operator::Or => byte1 | byte2,
            Operator::Xor => byte1 ^ byte2,
            Operator::Nand => !(byte1 & byte2),
            Operator::Nor => !(byte1 | byte2),
            Operator::Xnor => !(byte1 ^ byte2),
            Operator::WrappingAdd => byte1.wrapping_add(byte2),
            Operator::SaturatingAdd => byte1.saturating_add(byte2),
            Operator::AbsDiff => byte1.abs_diff(byte2),
            Operator::Greater => byte1.max(byte2),
            Operator::Lesser => byte1.min(byte2),
        }
    }
}

fn crossbit(
    operator: Operator,
    mut streams: impl Iterator<Item = Box<dyn Read>>,
    mut out: impl Write,
) -> Result<()> {
    let primary = streams
        .next()
        .map(BufReader::new)
        .map(Read::bytes)
        .expect("Expected at least one stream");
    let mut streams = streams
        .map(BufReader::new)
        .map(Read::bytes)
        .collect::<Vec<_>>();
    assert!(!streams.is_empty());

    let mut index = 0;
    let mut buffer = [0; PAGE_SIZE];

    'outer: for byte in primary {
        // Combine all the bytes
        let mut byte: u8 = byte?;
        for stream in &mut streams {
            if let Some(secondary) = stream.next() {
                byte = operator.cross(byte, secondary?);
            } else {
                break 'outer;
            }
        }

        // Buffer or write
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
    let stdout = io::stdout().lock();

    // Map into files, and treat "-" as stdin
    let files = args
        .files
        .into_iter()
        .map(|path: PathBuf| {
            if let Some(strpath) = path.to_str()
                && strpath == "-"
            {
                let file: Box<dyn Read> = Box::new(io::stdin().lock());
                Ok(file)
            } else {
                File::open(path).map(|file| {
                    let file: Box<dyn Read> = Box::new(file);
                    file
                })
            }
        })
        .collect::<Result<Vec<Box<dyn Read>>, _>>()?;

    crossbit(args.operator, files.into_iter(), stdout)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tv(tv: Operator, inputs: &[&'static [u8]], output: &mut Vec<u8>) -> Result<()> {
        let inputs = inputs.iter().map(|input| {
            let input: Box<dyn Read> = Box::new(*input);
            input
        });
        crossbit(tv, inputs, output)
    }

    #[test]
    fn operations() -> Result<()> {
        let tvs = [
            (
                Operator::And,
                &[0b0110, 0b0001],
                &[0b1010, 0b0101],
                &[0b0010, 0b0001],
            ),
            (
                Operator::Nand,
                &[0b0110, 0b0001],
                &[0b1010, 0b0101],
                &[0b1111_1101, 0b1111_1110],
            ),
            (
                Operator::Or,
                &[0b0110, 0b0001],
                &[0b1010, 0b0101],
                &[0b1110, 0b0101],
            ),
            (
                Operator::Nor,
                &[0b0110, 0b0001],
                &[0b1010, 0b0101],
                &[0b1111_0001, 0b1111_1010],
            ),
            (
                Operator::Xor,
                &[0b0110, 0b0001],
                &[0b1010, 0b0101],
                &[0b1100, 0b0100],
            ),
            (
                Operator::Xnor,
                &[0b0110, 0b0001],
                &[0b1010, 0b0101],
                &[0b1111_0011, 0b1111_1011],
            ),
        ];
        for tv in tvs {
            let mut out = Vec::with_capacity(tv.3.len());
            test_tv(tv.0, &[tv.1.as_slice(), tv.2.as_slice()], &mut out)?;
            assert_eq!(out, tv.3);
        }
        Ok(())
    }

    // Make sure we only produce the same number of bytes as the smaller of the two files
    #[test]
    fn size_diff() -> Result<()> {
        #[allow(clippy::type_complexity)]
        let tvs: &[(Operator, &[u8], &[u8], &[u8])] = &[
            (
                Operator::And,
                &[0b0110, 0b0001],
                &[0b1010, 0b0101],
                &[0b0010, 0b0001],
            ),
            (Operator::And, &[0b0110], &[0b1010, 0b0101], &[0b0010]),
            (Operator::And, &[0b0110, 0b0001], &[0b1010], &[0b0010]),
            (Operator::And, &[0b0110, 0b0001], &[], &[]),
            (Operator::And, &[], &[], &[]),
        ];
        for tv in tvs {
            let mut out = Vec::with_capacity(tv.3.len());
            test_tv(tv.0, &[tv.1, tv.2], &mut out)?;
            assert_eq!(out, tv.3);
        }
        Ok(())
    }
}
