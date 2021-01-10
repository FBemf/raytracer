use anyhow::{Context, Result};

use std::io::Write;
use std::time::Instant;

pub trait Progress {
    fn update(&mut self, numerator: usize, denominator: usize) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
}

pub struct TimedProgressBar<'a> {
    stream: Box<dyn Write + 'a>,
    length: usize,
    label: String,
    chars: Vec<String>,
    start_time: Instant,
}

impl<'a> TimedProgressBar<'a> {
    pub fn new<'b, T: Write + 'b>(
        stream: T,
        length: usize,
        label: &str,
        char_progression: &str,
        start_time: Instant,
    ) -> TimedProgressBar<'b> {
        assert_ne!(char_progression, "");
        TimedProgressBar {
            stream: Box::new(stream),
            length,
            label: String::from(label),
            start_time,
            chars: char_progression
                .chars()
                .map(|c| c.to_string())
                .collect::<Vec<String>>(),
        }
    }
}

impl Progress for TimedProgressBar<'_> {
    fn update(&mut self, numerator: usize, denominator: usize) -> Result<()> {
        let multiplier = self.chars.len() - 1;
        let progress = multiplier * numerator * self.length / denominator;
        let remainder = multiplier * self.length - progress;
        let full_char = self.chars.last().unwrap();
        let empty_char = self.chars.first().unwrap();
        let progress_modulus = progress % multiplier;
        let expected_time = if numerator != 0 {
            let secs = self.start_time.elapsed().as_secs() as usize * (denominator - numerator)
                / numerator;
            format!("{:3}:{:02}", secs / 60, secs % 60)
        } else {
            String::new()
        };
        self.stream
            .write(
                format!(
                    "\r{}: [{}{}{}] {} ",
                    self.label,
                    full_char.repeat(progress / multiplier),
                    if progress_modulus != 0 {
                        &self.chars[progress_modulus]
                    } else {
                        ""
                    },
                    empty_char.repeat(remainder / multiplier),
                    expected_time,
                )
                .as_bytes(),
            )
            .and(Ok(()))
            .context("Writing to progress bar")
    }

    fn clear(&mut self) -> Result<()> {
        self.stream
            .write(format!("\r{}\r", " ".repeat(self.length + self.label.len() + 12)).as_bytes())
            .and(Ok(()))
            .context("Clearing progress bar")
    }
}
