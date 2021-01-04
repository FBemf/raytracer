use anyhow::{Context, Result};
use std::io::Write;

pub struct Progress<'a> {
    stream: Box<dyn Write + 'a>,
    length: usize,
    label: String,
}

impl<'a> Progress<'a> {
    pub fn new<'b, T: Write + 'b>(stream: T, length: usize) -> Progress<'b> {
        Progress {
            stream: Box::new(stream),
            length,
            label: String::from("Progress"),
        }
    }

    pub fn set_label(&mut self, label: &str) {
        self.label = String::from(label);
    }

    pub fn update(&mut self, numerator: usize, denominator: usize) -> Result<()> {
        let progress = numerator * self.length / denominator;
        let remainder = self.length - progress;
        self.stream
            .write(
                format!(
                    "\r{}: [{}{}] ",
                    self.label,
                    "#".repeat(progress),
                    " ".repeat(remainder)
                )
                .as_bytes(),
            )
            .and(Ok(()))
            .context("Writing to progress bar")
    }

    pub fn clear(&mut self) -> Result<()> {
        self.stream
            .write(format!("\r{}\r", " ".repeat(self.length + self.label.len() + 4)).as_bytes())
            .and(Ok(()))
            .context("Clearing progress bar")
    }
}
