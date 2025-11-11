//! Standard Stream Split - duplicate stdin to both stdout and stderr
// TODO: alt names: speek? steek? ssp?
use anyhow::Result;
use clap::Parser;
use std::io::{self, Read, Write};

const PAGE_SIZE: usize = 4096;

/// Standard Stream Split - duplicate stdin to both stdout and stderr
#[derive(Parser)]
struct Args {}

fn stream_split(
    mut stdin: impl Read,
    mut stdout: impl Write,
    mut stderr: impl Write,
) -> Result<()> {
    let mut buf = [0u8; PAGE_SIZE];
    while let bytes = stdin.read(&mut buf)?
        && bytes != 0
    {
        stdout.write_all(&buf[0..bytes])?;
        stderr.write_all(&buf[0..bytes])?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let _args = Args::parse();
    stream_split(io::stdin().lock(), io::stdout().lock(), io::stderr().lock())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn basic_functionality() {
        let tvs = [
            "",
            "hi",
            "foo\nbar",
            "\n",
            "\r",
            "\r\nbaz\n\t",
            "The quic\nk brown fox jamped\n over the lazy dorg\n\n",
            "👋",
        ];
        for tv in tvs {
            let stdin = Cursor::new(String::from(tv));
            let mut stdout = Vec::<u8>::new();
            let mut stderr = Vec::<u8>::new();
            stream_split(stdin, &mut stdout, &mut stderr).unwrap();
            assert_eq!(tv, String::from_utf8(stdout).unwrap());
            assert_eq!(tv, String::from_utf8(stderr).unwrap());
        }
    }
}
