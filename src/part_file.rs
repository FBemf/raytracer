use anyhow::{anyhow, bail, Context, Result};

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

pub struct PartFile {
    pub file: File,
    pub path: PathBuf,
}

impl PartFile {
    pub fn new(image_name: &PathBuf) -> Result<PartFile> {
        let mut path = PathBuf::new();
        let base = image_name
            .file_name()
            .ok_or(anyhow!("bad image name"))?
            .to_str()
            .ok_or(anyhow!("bad image name"))?;
        for i in 0.. {
            path = PathBuf::from(format!("{}.{}.PART", base, i));
            if !path.exists() {
                break;
            }
        }
        Ok(PartFile {
            file: File::create(&path)?,
            path,
        })
    }

    pub fn write_part(&mut self, line_number: u32, part: Vec<u8>) -> Result<()> {
        self.file
            .write_all(format!("L{} ", line_number).as_bytes())?;
        self.file.write_all(&part)?;
        self.file.write_all("\n".as_bytes())?;
        self.file.flush()?;
        Ok(())
    }

    pub fn read(
        name: &PathBuf,
        image_height: u32,
        image_width: u32,
        recover_corrupt: bool,
    ) -> Result<Vec<Option<Vec<u8>>>> {
        let mut file = File::open(name)?;
        let mut list = vec![None; image_height as usize];
        let width2 = file_read_num(&mut file).context("Reading width")?;
        let height2 = file_read_num(&mut file).context("Reading height")?;
        if image_height as usize != height2 || image_width as usize != width2 {
            bail!(
                "Image dimensions expected to be {}x{}, but recovery file says they're {}x{}",
                image_width,
                image_height,
                width2,
                height2
            );
        }
        loop {
            match read_part(&mut file, &mut list, image_width as usize) {
                Ok(false) => {}
                Ok(true) => break,
                Err(e) => {
                    if recover_corrupt {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Ok(list)
    }
}

fn read_part(file: &mut File, list: &mut Vec<Option<Vec<u8>>>, image_width: usize) -> Result<bool> {
    match file_get_byte(file) {
        Err(e) => {
            if io::ErrorKind::UnexpectedEof == e.kind() {
                return Ok(true);
            } else {
                bail!("Read error {}", e);
            }
        }
        Ok(b'L') => {}
        Ok(_) => {
            bail!("Missing leading L");
        }
    }
    let line_number = file_read_num(file)?;
    let mut buf2 = vec![0; image_width * 3];
    file.read_exact(&mut buf2[..])?;
    list[line_number] = Some(buf2);
    if file_get_byte(file)? != b'\n' {
        bail!("Missing newline on line {}", line_number);
    }
    Ok(false)
}

fn file_get_byte(file: &mut File) -> io::Result<u8> {
    let mut buf = [0];
    file.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn file_read_num(file: &mut File) -> Result<usize> {
    let mut buf = [0];
    let mut num = String::new();
    loop {
        file.read_exact(&mut buf)?;
        if buf == [b' '] || buf == [b'\n'] {
            break;
        }
        num += &String::from_utf8_lossy(&buf);
    }
    Ok(num.parse::<usize>()?)
}
