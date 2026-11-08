use anyhow::{Result, ensure};
use encoding_rs::{Decoder, Encoding};

use crate::profile::LineEnding;

pub fn parse_hex(input: &str) -> Result<Vec<u8>> {
    let digits: String = input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    ensure!(
        !digits.is_empty()
            && digits.len().is_multiple_of(2)
            && digits.bytes().all(|c| c.is_ascii_hexdigit()),
        "HEX requires complete pairs of hexadecimal digits"
    );
    digits
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair)?, 16).map_err(Into::into))
        .collect()
}

pub fn encode_text(input: &str, label: &str, ending: LineEnding) -> Result<Vec<u8>> {
    let encoding =
        Encoding::for_label(label.as_bytes()).ok_or_else(|| anyhow::anyhow!("Unknown encoding"))?;
    let (bytes, _, errors) = encoding.encode(input);
    ensure!(
        !errors,
        "Text cannot be represented in the selected encoding"
    );
    let mut bytes = bytes.into_owned();
    bytes.extend_from_slice(ending.bytes());
    Ok(bytes)
}

pub struct TextDecoder(Decoder);

impl TextDecoder {
    pub fn new(label: &str) -> Result<Self> {
        Ok(Self(
            Encoding::for_label(label.as_bytes())
                .ok_or_else(|| anyhow::anyhow!("Unknown encoding"))?
                .new_decoder_without_bom_handling(),
        ))
    }

    pub fn decode(&mut self, bytes: &[u8], last: bool) -> String {
        let mut result = String::with_capacity(
            self.0
                .max_utf8_buffer_length(bytes.len())
                .unwrap_or(bytes.len() * 4 + 16),
        );
        let mut consumed = 0;
        loop {
            let (outcome, read, _) = self
                .0
                .decode_to_string(&bytes[consumed..], &mut result, last);
            consumed += read;
            match outcome {
                encoding_rs::CoderResult::InputEmpty => break,
                encoding_rs::CoderResult::OutputFull => result.reserve(bytes.len() * 4 + 16),
            }
        }
        result
    }
}

pub fn safe_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => result.push_str("\r\n"),
            '\r' | '\t' => result.push(c),
            c if c.is_control() => result.extend(c.escape_default()),
            c => result.push(c),
        }
    }
    result
}

pub fn hex_line(offset: u64, bytes: &[u8]) -> String {
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    let ascii: String = bytes
        .iter()
        .map(|byte| {
            if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            }
        })
        .collect();
    format!("{offset:08X}  {hex:<47}  |{ascii}|\r\n")
}

pub fn timestamped_text(text: &str, at_ms: u128, at_line_start: &mut bool) -> String {
    let mut result = String::new();
    for line in text.split_inclusive('\n') {
        if *at_line_start && line.chars().any(|c| c != '\r' && c != '\n') {
            let seconds = at_ms / 1000;
            result.push_str(&format!(
                "[{:02}:{:02}:{:02}.{:03} UTC] ",
                seconds / 3600 % 24,
                seconds / 60 % 60,
                seconds % 60,
                at_ms % 1000
            ));
            *at_line_start = false;
        }
        result.push_str(&safe_text(line));
        if line.ends_with('\n') {
            *at_line_start = true;
        }
    }
    result
}

#[cfg(test)]
#[path = "codec_tests.rs"]
mod tests;
