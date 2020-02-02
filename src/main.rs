use std::io::{ stdin, Read };

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::playback;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::playback;

#[cfg(not(any(windows, unix)))]
error!("unsupported platform");

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    if let Err(e) = playback() {
        eprintln!("An error occured: {}", e);
    }
}

fn read_hex() -> Result<Option<u64>> {
    let mut line = String::new();
    stdin().read_line(&mut line)?;
    let line = line.trim();
    if line.is_empty() {
        return Ok(None)
    }
    Ok(Some(u64::from_str_radix(line, 16)?))
}

fn read_input() -> Result<Option<(u64, u64)>> {
    let mut line = String::new();
    stdin().read_line(&mut line)?;
    if line.is_empty() {
        return Ok(None)
    }
    let mut input = 0;
    let mut repeat_index = None;
    let mut repeat_end = None;
    for (i, c) in line.char_indices() {
        match c.to_ascii_lowercase() {
            '<' if repeat_index.is_none() => input |= 0x01,
            '>' if repeat_index.is_none() => input |= 0x02,
            'd' if repeat_index.is_none() => input |= 0x04,
            'v' if repeat_index.is_none() => input |= 0x08,
            'l' if repeat_index.is_none() => input |= 0x10,
            'r' if repeat_index.is_none() => input |= 0x20,
            'h' if repeat_index.is_none() => input |= 0x40,
            '0'..='9' => if repeat_index.is_none() {
                repeat_index = Some(i);
            }
            ' ' => if repeat_index.is_some() && repeat_end.is_none() {
                repeat_end = Some(i);
            }
            _ => {}
        }
    }
    let repeat = match repeat_index {
        Some(start) => match repeat_end {
            Some(end) => u64::from_str_radix(&line[start..end], 10)?,
            None => u64::from_str_radix(&line[start..].trim(), 10)?
        }
        None => 1
    };
    Ok(Some((input, repeat)))
}