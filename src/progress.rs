use anyhow::{Context, Result};

use std::convert::TryFrom;
use std::io::Write;
use std::time::{Duration, Instant};

pub trait Progress {
    fn update(&mut self) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
}

pub struct TimedProgressBar<'a> {
    stream: Box<dyn Write + 'a>,
    length: usize,
    label: String,
    chars: Vec<String>,
    number_samples: u32,
    update_times: Vec<Instant>,
    times_will_be_updated: u32,
}

impl<'a> TimedProgressBar<'a> {
    pub fn new<'b, T: Write + 'b>(
        stream: T,
        length: usize,
        label: &str,
        char_progression: &str,
        number_samples: u32,
        times_will_be_updated: u32,
    ) -> TimedProgressBar<'b> {
        assert_ne!(char_progression, "");
        TimedProgressBar {
            stream: Box::new(stream),
            length,
            label: String::from(label),
            chars: char_progression
                .chars()
                .map(|c| c.to_string())
                .collect::<Vec<String>>(),
            number_samples,
            update_times: Vec::with_capacity(times_will_be_updated as usize),
            times_will_be_updated,
        }
    }
}

impl Progress for TimedProgressBar<'_> {
    fn update(&mut self) -> Result<()> {
        // update list of times updated
        self.update_times.push(Instant::now());

        // derive a bunch of info to help draw the bar
        let length = self.length - self.label.len() - 15;
        let multiplier = self.chars.len() - 1;
        let progress =
            multiplier * self.update_times.len() * length / self.times_will_be_updated as usize;
        let remainder = multiplier * length - progress;
        let full_char = self.chars.last().unwrap();
        let empty_char = self.chars.first().unwrap();
        let progress_modulus = progress % multiplier;

        // predict how much longer is left
        let times_elapsed = self
            .update_times
            .iter()
            .rev()
            .take(self.number_samples as usize)
            .map(|i| i.elapsed());
        let update_times = times_elapsed
            .clone()
            .zip(times_elapsed.skip(1))
            .map(|(a, b)| b - a);
        let number_update_times = u32::try_from(update_times.len()).unwrap();
        let secs_left = if number_update_times > 0 {
            let average_update_time = update_times.sum::<Duration>() / number_update_times;
            ((self.times_will_be_updated - u32::try_from(self.update_times.len()).unwrap())
                * average_update_time)
                .as_secs()
        } else {
            0
        };
        // draw the progress bar
        let expected_time = format!(
            "{:2}:{:02}:{:02}",
            secs_left / 3600,
            (secs_left % 3600) / 60,
            secs_left % 60
        );
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
