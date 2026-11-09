use super::*;

#[test]
fn new_serial_connection_defaults_to_interactive_terminal() {
    let cli = Cli::try_parse_from(["wireterm-connect", "serial", "--port", "COM4"]).unwrap();
    let Action::Serial(args) = cli.command else {
        panic!("Expected serial connection");
    };
    assert_eq!(args.display, DisplayMode::Ansi);
    assert!(!args.hex_send);
}

#[test]
fn display_switch_can_return_to_interactive_terminal() {
    let text = next_display(DisplayMode::Ansi);
    assert_eq!(text, DisplayMode::Text);
    let hex = next_display(text);
    assert_eq!(hex, DisplayMode::Hex);
    assert_eq!(next_display(hex), DisplayMode::Ansi);
}

#[test]
fn interactive_keys_encode_device_terminal_controls() {
    for (code, modifiers, expected) in [
        (KeyCode::Enter, KeyModifiers::NONE, &b"\r"[..]),
        (KeyCode::Char('c'), KeyModifiers::CONTROL, &b"\x03"[..]),
        (KeyCode::Up, KeyModifiers::NONE, &b"\x1b[A"[..]),
        (KeyCode::Backspace, KeyModifiers::NONE, &b"\x7f"[..]),
    ] {
        assert_eq!(ansi_key(event::KeyEvent::new(code, modifiers)), expected);
    }
}
