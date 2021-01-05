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
    start_time: Instant,
}

impl<'a> TimedProgressBar<'a> {
    pub fn new<'b, T: Write + 'b>(
        stream: T,
        length: usize,
        label: &str,
        start_time: Instant,
    ) -> TimedProgressBar<'b> {
        TimedProgressBar {
            stream: Box::new(stream),
            length,
            label: String::from(label),
            start_time,
        }
    }
}

impl Progress for TimedProgressBar<'_> {
    fn update(&mut self, numerator: usize, denominator: usize) -> Result<()> {
        let progress = 2 * numerator * self.length / denominator;
        let remainder = 2 * self.length - progress;
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
                    ":".repeat(progress / 2),
                    if progress % 2 == 1 { "." } else { "" },
                    " ".repeat(remainder / 2),
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
