use super::*;

#[test]
fn hex_accepts_whitespace_and_rejects_partial_or_non_hex_input() {
    assert_eq!(parse_hex("00 ff\n7A\t80").unwrap(), [0, 255, 122, 128]);
    for input in ["", "0", "abc", "0x12", "GG", "12;34", "设备"] {
        assert!(parse_hex(input).is_err(), "{input}");
    }
}

#[test]
fn utf8_and_gbk_characters_survive_packet_boundaries() {
    for label in ["utf-8", "gbk"] {
        let bytes = encode_text("设备 ready", label, LineEnding::None).unwrap();
        let mut decoder = TextDecoder::new(label).unwrap();
        let mut text = String::new();
        for byte in bytes {
            text.push_str(&decoder.decode(&[byte], false));
        }
        text.push_str(&decoder.decode(&[], true));
        assert_eq!(text, "设备 ready");
    }
}

#[test]
fn unrepresentable_text_is_not_silently_substituted() {
    assert!(encode_text("😀", "gbk", LineEnding::None).is_err());
}

#[test]
fn line_endings_are_explicit_and_hex_never_adds_one() {
    for (ending, expected) in [
        (LineEnding::None, "x"),
        (LineEnding::Lf, "x\n"),
        (LineEnding::Crlf, "x\r\n"),
        (LineEnding::Cr, "x\r"),
    ] {
        assert_eq!(
            encode_text("x", "utf-8", ending).unwrap(),
            expected.as_bytes()
        );
    }
    assert_eq!(parse_hex("00 0D 0A").unwrap(), [0, 13, 10]);
}

#[test]
fn default_text_display_cannot_emit_terminal_control_sequences() {
    let text = safe_text("hello\x1b]52;c;payload\x07\n");
    assert!(!text.contains('\x1b'));
    assert!(!text.contains('\x07'));
    assert!(text.ends_with("\r\n"));
}

#[test]
fn hex_display_keeps_binary_bytes_and_offsets_visible() {
    let line = hex_line(32, &[0, 0x1b, 0x41, 0xff]);
    assert!(line.starts_with("00000020  00 1B 41 FF"));
    assert!(line.contains("|..A.|"));
}

#[test]
fn timestamps_follow_lines_rather_than_network_packet_boundaries() {
    let mut start = true;
    let first = timestamped_text("hel", 3723004, &mut start);
    let rest = timestamped_text("lo\nworld", 3723005, &mut start);
    assert_eq!(
        format!("{first}{rest}"),
        "[01:02:03.004 UTC] hello\r\n[01:02:03.005 UTC] world"
    );
}
