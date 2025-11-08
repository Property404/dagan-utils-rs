use anyhow::{Result, anyhow, bail};
use clap::Parser;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    num::NonZeroUsize,
    path::PathBuf,
};

// Pattern that may have a starting and ending line number
// Parsed from a Rust-like range pattern:
// `..`, `5..`, `6..=10`, etc
#[derive(Debug, Clone)]
struct Pattern {
    start: Option<NonZeroUsize>,
    // This end is INCLUSIVE
    end: Option<NonZeroUsize>,
}

impl Pattern {
    // Check if a line number would be included
    fn is_included(&self, line: NonZeroUsize) -> bool {
        if let Some(start) = self.start
            && line < start
        {
            return false;
        }
        if let Some(end) = self.end
            && line > end
        {
            return false;
        }
        true
    }

    // Construct a pattern from a string
    fn parse(pattern: &str) -> Result<Self> {
        fn try_nonzero(num: usize) -> Result<NonZeroUsize> {
            NonZeroUsize::new(num).ok_or_else(|| anyhow!("Line numbers are 1-indexed"))
        }

        if let Some((start, end)) = pattern.split_once("..") {
            let start = if start.is_empty() {
                None
            } else {
                Some(try_nonzero(start.parse()?)?)
            };

            let end = if end.is_empty() {
                None
            } else if let Some(end) = end.strip_prefix("=") {
                Some(try_nonzero(end.parse()?)?)
            } else {
                let num: usize = end.parse()?;
                if num <= 1 {
                    bail!("End of exclusive range must be greater than 1");
                }
                Some(try_nonzero(num - 1)?)
            };

            if let (Some(start), Some(end)) = (start, end)
                && start > end
            {
                bail!("Reverse patterns not supported");
            }

            Ok(Self { start, end })
        } else if let Ok(start) = pattern.parse::<usize>() {
            let val = Some(try_nonzero(start)?);
            Ok(Self {
                start: val,
                end: val,
            })
        } else {
            bail!("Could not interpret line number pattern: {pattern}");
        }
    }
}

fn write_lines(
    fin: impl Read,
    mut fout: impl Write,
    patterns: &str,
    options: Options,
) -> Result<()> {
    let patterns = patterns
        .split(",")
        .map(Pattern::parse)
        .collect::<Result<Vec<Pattern>>>()?;

    // We consume lines, so patterns must be given in order
    // In the future, this restriction could be lifted
    patterns.iter().try_fold(
        Pattern {
            start: None,
            end: None,
        },
        |prev, this| {
            if prev.start.is_some() || prev.end.is_some() {
                let prev_end = prev.end.unwrap_or(NonZeroUsize::MAX);
                let this_start = this.start.unwrap_or(NonZeroUsize::MIN);
                if prev_end > this_start {
                    return Err(anyhow!("Lines currently must be given in order"));
                }
            }
            Ok(this.clone())
        },
    )?;

    let fin = BufReader::new(fin);
    for (number, line) in fin.lines().enumerate() {
        // Lines are 1-indexed
        let number = NonZeroUsize::new(number + 1).expect("Overflow");
        let line = line?;

        // Write line as many times as the pattern list calls for it
        let mut can_break = true;
        for pattern in &patterns {
            if pattern.is_included(number) {
                if options.show_line_number {
                    write!(fout, "{number}\t")?;
                }
                // This seems to perform better than using `writeln!`
                fout.write_all(line.as_bytes())?;
                fout.write_all(b"\n")?;
            }
            // Don't bother reading the rest if we don't have to
            if let Some(end) = pattern.end {
                if end > number {
                    can_break = false;
                }
            } else {
                can_break = false
            }
        }
        if can_break {
            break;
        }
    }

    Ok(())
}

#[derive(Default)]
struct Options {
    show_line_number: bool,
}

/// Display selected lines from a file or stdin
#[derive(Parser)]
struct Args {
    /// Show line numbers
    #[clap(short = 'n')]
    show_line_number: bool,
    /// The lines or ranges of lines to display, separated by a comma
    ///
    /// # Examples
    ///
    /// "5" - show line 5  
    /// "1,6,7" - show lines 1, 6, and 7
    /// "5..7" - Show lines 5 and 6  
    /// "5..=7" - Show lines 5, 6, and 7  
    /// "1,5..7" - Show lines 1, 5, and 6
    /// ".." - Show all lines
    /// "5.." - Show all after and including 5
    /// "..7" - Show all lines up to 7, excluding 7
    /// "..=7" - Show all lines up to 7, including 7
    ///
    /// # Note
    ///
    /// Lines must be specified in order. This restriction might be lifted in the future.
    #[clap(verbatim_doc_comment)]
    lines: String,
    /// The file to read
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let options = Options {
        show_line_number: args.show_line_number,
    };
    let stdout = io::stdout().lock();
    if let Some(file) = args.file {
        let file = File::open(file)?;
        write_lines(file, stdout, &args.lines, options)?;
    } else {
        let stdin = io::stdin().lock();
        write_lines(stdin, stdout, &args.lines, options)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn lines_must_be_specified_in_order() {
        let failing_patterns = [
            ("4,4", false),
            ("4,5", false),
            ("5,4", true),
            ("1..9,4", true),
            ("8..9,4", true),
            ("2..4,1", true),
            ("2..4,4", false),
            ("2..=4,4", false),
        ];

        for tv in failing_patterns {
            let fin = Cursor::new(String::from("Foo\nBar\nBaz"));
            let mut fout = Vec::new();
            let patterns = tv.0;
            let should_error = tv.1;
            assert_eq!(
                should_error,
                write_lines(fin, &mut fout, patterns, Default::default()).is_err()
            );
        }
    }

    #[test]
    fn select_lines() -> Result<()> {
        let tvs: &[(&str, &str, &[&str])] = &[
            ("", "1,2,2", &[]),
            ("Foo\nBar", "1,2,2", &["Foo", "Bar", "Bar"]),
            ("Foo\nBar\nBaz", "..", &["Foo", "Bar", "Baz"]),
            ("Foo\nBar\nBaz", "1..", &["Foo", "Bar", "Baz"]),
            ("Foo\nBar\nBaz", "2..", &["Bar", "Baz"]),
            ("Foo\nBar\nBaz", "2..3", &["Bar"]),
            ("Foo\nBar\nBaz", "2..=3", &["Bar", "Baz"]),
            ("Foo\nBar\nBaz", "..=3", &["Foo", "Bar", "Baz"]),
            ("Foo\nBar\nBaz", "..3", &["Foo", "Bar"]),
            ("Foo\nBar\nBaz", "..3,3,3", &["Foo", "Bar", "Baz", "Baz"]),
            ("Foo\nBar", "..", &["Foo", "Bar"]),
            ("Foo\nBar\n", "..", &["Foo", "Bar"]),
            ("Foo\nBar\n\n", "..", &["Foo", "Bar", ""]),
            ("Foo\nBar\nBaz", "1,2..", &["Foo", "Bar", "Baz"]),
        ];

        for tv in tvs {
            let fin = Cursor::new(String::from(tv.0));
            let mut fout = Vec::new();
            let patterns = tv.1;
            write_lines(fin, &mut fout, patterns, Default::default())?;

            let actual_lines = String::from_utf8(fout)?;
            let actual_lines = actual_lines.lines().collect::<Vec<_>>();
            for (expected_line, actual_line) in tv.2.iter().zip(actual_lines.iter()) {
                assert_eq!(expected_line, actual_line);
            }
            assert_eq!(tv.2.len(), actual_lines.len());
        }
        Ok(())
    }

    #[test]
    fn pattern_parsing() {
        let p = Pattern::parse("1").unwrap();
        assert_eq!(p.start.unwrap().get(), 1);
        assert_eq!(p.end.unwrap().get(), 1);

        let p = Pattern::parse("..").unwrap();
        assert_eq!(p.start, None);
        assert_eq!(p.end, None);

        let p = Pattern::parse("5..").unwrap();
        assert_eq!(p.start.unwrap().get(), 5);
        assert_eq!(p.end, None);

        let p = Pattern::parse("42..100").unwrap();
        assert_eq!(p.start.unwrap().get(), 42);
        assert_eq!(p.end.unwrap().get(), 99);

        let p = Pattern::parse("..2").unwrap();
        assert_eq!(p.start, None);
        assert_eq!(p.end.unwrap().get(), 1);

        let p = Pattern::parse("..=2").unwrap();
        assert_eq!(p.start, None);
        assert_eq!(p.end.unwrap().get(), 2);

        let p = Pattern::parse("1..=1").unwrap();
        assert_eq!(p.start.unwrap().get(), 1);
        assert_eq!(p.end.unwrap().get(), 1);

        let p = Pattern::parse("5..=100").unwrap();
        assert_eq!(p.start.unwrap().get(), 5);
        assert_eq!(p.end.unwrap().get(), 100);

        assert!(Pattern::parse("0..5").is_err());
        assert!(Pattern::parse("..0").is_err());
        assert!(Pattern::parse("..1").is_err());
        assert!(Pattern::parse("0").is_err());
    }
}
