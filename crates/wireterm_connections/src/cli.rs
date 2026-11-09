use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use clap::{Args, Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::codec::{TextDecoder, encode_text, hex_line, parse_hex, safe_text, timestamped_text};
use crate::profile::{self, ConnectionProfile, DisplayMode, LineEnding, SerialProfile, SshProfile};
use crate::serial::{Direction, SerialCommand, SerialSession, SessionState};

#[derive(Parser)]
#[command(
    name = "wireterm-connect",
    about = "Local Serial and direct OpenSSH connections"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Connection profile and known-host directory (portable installations)"
    )]
    config_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Discover serial ports without opening or writing to them.
    Ports,
    /// Configure a connection interactively in this terminal.
    Wizard {
        #[arg(value_parser = ["serial", "ssh"])]
        transport: String,
    },
    Serial(SerialArgs),
    Ssh(SshArgs),
    /// Open a previously saved connection profile.
    Open {
        name: String,
    },
    /// List local connection profiles (no credentials are stored).
    Profiles,
    /// Delete a saved profile after confirmation; never deletes keys or recordings.
    Delete {
        name: String,
    },
    /// Save a serial send template. Saving never connects or transmits.
    Template {
        profile: String,
        name: String,
        body: String,
        #[arg(long)]
        hex: bool,
    },
}

#[derive(Args)]
struct SerialArgs {
    #[arg(long)]
    port: String,
    #[arg(long, default_value_t = 115200)]
    baud: u32,
    #[arg(long, default_value_t = 8)]
    data_bits: u8,
    #[arg(long, default_value = "none", value_parser = ["none", "odd", "even"])]
    parity: String,
    #[arg(long, default_value_t = 1)]
    stop_bits: u8,
    #[arg(long, default_value = "none", value_parser = ["none", "hardware", "software"])]
    flow_control: String,
    #[arg(long)]
    dtr: Option<bool>,
    #[arg(long)]
    rts: Option<bool>,
    #[arg(long, default_value = "utf-8")]
    encoding: String,
    #[arg(long, value_enum, default_value = "ansi")]
    display: DisplayMode,
    #[arg(long)]
    hex_send: bool,
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    timestamps: bool,
    #[arg(long, value_enum, default_value = "crlf")]
    line_ending: LineEnding,
    #[arg(long)]
    reconnect: bool,
    #[arg(long)]
    record: Option<PathBuf>,
    #[arg(long)]
    save: Option<String>,
}

#[derive(Args)]
struct SshArgs {
    host: String,
    #[arg(short = 'l', long)]
    user: Option<String>,
    #[arg(short = 'p', long, default_value_t = 22)]
    port: u16,
    #[arg(short = 'i', long)]
    identity: Option<PathBuf>,
    #[arg(long)]
    save: Option<String>,
}

fn prompt(label: &str, default: &str) -> Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    ensure!(
        io::stdin().read_line(&mut input)? != 0,
        "Connection setup cancelled"
    );
    let input = input.trim();
    Ok(if input.is_empty() {
        default.into()
    } else {
        input.into()
    })
}

fn list_ports() -> Result<()> {
    for port in crate::serial::ports()? {
        println!("{}  {:?}", port.port_name, port.port_type);
    }
    Ok(())
}

fn wizard(transport: &str, directory: &std::path::Path) -> Result<ConnectionProfile> {
    println!("WireTerm {transport} connection — no connection is opened until confirmation.");
    let names = profile::list(directory)?;
    if !names.is_empty() {
        println!("Saved profiles: {}", names.join(", "));
        let name = prompt("Existing profile (blank creates a new connection)", "")?;
        if !name.is_empty() {
            let profile = profile::load(directory, &name)?;
            ensure!(
                matches!(
                    (&profile, transport),
                    (ConnectionProfile::Serial(_), "serial") | (ConnectionProfile::Ssh(_), "ssh")
                ),
                "Profile uses a different transport"
            );
            ensure!(
                prompt("Connect now? Type yes to confirm", "no")? == "yes",
                "Connection cancelled"
            );
            return Ok(profile);
        }
    }
    let mut profile = match transport {
        "serial" => {
            list_ports()?;
            let port = prompt("Port path (e.g. COM3 or /dev/ttyUSB0)", "")?;
            let baud_rate = prompt("Baud rate", "115200")?
                .parse()
                .context("Invalid baud rate")?;
            let data_bits = prompt("Data bits (5..8)", "8")?.parse()?;
            let parity = prompt("Parity (none/odd/even)", "none")?;
            let stop_bits = prompt("Stop bits (1/2)", "1")?.parse()?;
            let flow_control = prompt("Flow control (none/hardware/software)", "none")?;
            let encoding = prompt("Text encoding (utf-8/gbk)", "utf-8")?;
            let display =
                match prompt("Display (ansi=interactive terminal/text/hex)", "ansi")?.as_str() {
                    "text" => DisplayMode::Text,
                    "hex" => DisplayMode::Hex,
                    "ansi" => DisplayMode::Ansi,
                    _ => anyhow::bail!("Invalid display mode"),
                };
            let line_ending =
                match prompt("Text send line ending (none/lf/crlf/cr)", "crlf")?.as_str() {
                    "none" => LineEnding::None,
                    "lf" => LineEnding::Lf,
                    "crlf" => LineEnding::Crlf,
                    "cr" => LineEnding::Cr,
                    _ => anyhow::bail!("Invalid line ending"),
                };
            let send_hex = match prompt("Send mode (text/hex)", "text")?.as_str() {
                "text" => false,
                "hex" => true,
                _ => anyhow::bail!("Invalid send mode"),
            };
            let timestamps = prompt("Line timestamps? (yes/no)", "yes")? == "yes";
            let auto_reconnect =
                prompt("Reconnect same USB identity automatically? (yes/no)", "no")? == "yes";
            let recording = prompt(
                "Raw JSONL recording path (blank disables; existing files are never overwritten)",
                "",
            )?;
            ConnectionProfile::Serial(SerialProfile {
                port,
                baud_rate,
                data_bits,
                parity,
                stop_bits,
                flow_control,
                encoding,
                display,
                send_hex,
                timestamps,
                line_ending,
                auto_reconnect,
                recording: (!recording.is_empty()).then(|| recording.into()),
                ..Default::default()
            })
        }
        "ssh" => ConnectionProfile::Ssh(SshProfile {
            host: prompt("Host or SSH config alias", "")?,
            user: {
                let user = prompt("Username (blank uses OpenSSH config)", "")?;
                (!user.is_empty()).then_some(user)
            },
            port: prompt("Port", "22")?.parse()?,
            identity_file: {
                let file = prompt("Private key path (blank uses agent/default keys)", "")?;
                (!file.is_empty()).then(|| file.into())
            },
        }),
        _ => anyhow::bail!("Unknown transport"),
    };
    profile.validate()?;
    bind_identity(&mut profile);
    let name = prompt("Save as profile (blank skips)", "")?;
    if !name.is_empty() {
        profile::save(directory, &name, &profile)?;
    }
    ensure!(
        prompt("Connect now? Type yes to confirm", "no")? == "yes",
        "Connection cancelled"
    );
    Ok(profile)
}

pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<i32> {
    let cli = match Cli::try_parse_from(
        std::iter::once(OsString::from("wireterm-connect")).chain(args),
    ) {
        Ok(cli) => cli,
        Err(error) => {
            let code = error.exit_code();
            error.print()?;
            return Ok(code);
        }
    };
    let directory = match cli.config_dir {
        Some(directory) => directory,
        None => profile::config_directory()?,
    };
    let profile = match cli.command {
        Action::Ports => {
            list_ports()?;
            return Ok(0);
        }
        Action::Profiles => {
            for name in profile::list(&directory)? {
                println!("{name}");
            }
            return Ok(0);
        }
        Action::Delete { name } => {
            ensure!(
                prompt(&format!("Delete profile {name}? Type yes to confirm"), "no")? == "yes",
                "Deletion cancelled"
            );
            profile::delete(&directory, &name)?;
            return Ok(0);
        }
        Action::Template {
            profile: name,
            name: template_name,
            body,
            hex,
        } => {
            let mut connection = profile::load(&directory, &name)?;
            let ConnectionProfile::Serial(serial) = &mut connection else {
                anyhow::bail!("Send templates require a serial profile");
            };
            serial
                .templates
                .insert(template_name, profile::SendTemplate { body, hex });
            profile::update(&directory, &name, &connection)?;
            println!("Template saved. No connection opened and nothing transmitted.");
            return Ok(0);
        }
        Action::Wizard { transport } => wizard(&transport, &directory)?,
        Action::Open { name } => profile::load(&directory, &name)?,
        Action::Serial(args) => {
            let mut profile = ConnectionProfile::Serial(SerialProfile {
                port: args.port,
                baud_rate: args.baud,
                data_bits: args.data_bits,
                parity: args.parity,
                stop_bits: args.stop_bits,
                flow_control: args.flow_control,
                dtr: args.dtr,
                rts: args.rts,
                encoding: args.encoding,
                display: args.display,
                line_ending: args.line_ending,
                send_hex: args.hex_send,
                timestamps: args.timestamps,
                auto_reconnect: args.reconnect,
                recording: args.record,
                identity: None,
                templates: Default::default(),
            });
            profile.validate()?;
            bind_identity(&mut profile);
            if let Some(name) = args.save {
                profile::save(&directory, &name, &profile)?;
            }
            profile
        }
        Action::Ssh(args) => {
            let profile = ConnectionProfile::Ssh(SshProfile {
                host: args.host,
                user: args.user,
                port: args.port,
                identity_file: args.identity,
            });
            profile.validate()?;
            if let Some(name) = args.save {
                profile::save(&directory, &name, &profile)?;
            }
            profile
        }
    };
    match profile {
        ConnectionProfile::Serial(profile) => serial_console(profile),
        ConnectionProfile::Ssh(profile) => crate::ssh::connect(&profile, &directory),
    }
}

fn bind_identity(profile: &mut ConnectionProfile) {
    if let ConnectionProfile::Serial(profile) = profile
        && profile.identity.is_none()
    {
        profile.identity = crate::serial::ports()
            .ok()
            .and_then(|ports| crate::serial::capture_unique_identity(&profile.port, &ports));
    }
}

struct RawMode;
impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

fn serial_console(profile: SerialProfile) -> Result<i32> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let _entered = runtime.enter();
    let mut decoder = TextDecoder::new(&profile.encoding)?;
    let mut session = SerialSession::spawn(profile.clone())?;
    println!(
        "Serial console: ANSI forwards keys directly; TEXT/HEX sends a line on Enter. Opening sends no data.\nCtrl+Q: close | Ctrl+X: TEXT/HEX send | Ctrl+V: ANSI/TEXT/HEX display | Ctrl+T: timestamps | Ctrl+]: control command\nControl commands: dtr on/off, rts on/off, break — type then Enter to confirm.\nIn ANSI mode keys except the local shortcuts above are forwarded immediately."
    );
    crossterm::terminal::enable_raw_mode()?;
    let raw = RawMode;
    let result = console_loop(&profile, &mut decoder, &mut session);
    let final_status = session.status.clone();
    runtime.block_on(session.close())?;
    drop(raw);
    let final_status = final_status.borrow();
    println!(
        "\nSerial port released. RX: {} bytes / TX: {} bytes / display skipped: {} bytes",
        final_status.received, final_status.sent, final_status.skipped_display_bytes
    );
    result?;
    ensure!(
        final_status.state != SessionState::Failed,
        "{}",
        final_status.message
    );
    Ok(0)
}

fn console_loop(
    profile: &SerialProfile,
    decoder: &mut TextDecoder,
    session: &mut SerialSession,
) -> Result<()> {
    let mut display = profile.display;
    let mut hex_send = profile.send_hex;
    let mut timestamps = profile.timestamps;
    let mut at_line_start = true;
    let mut control = false;
    let mut input = String::new();
    let mut history = VecDeque::<String>::new();
    let mut history_index = None;
    let mut next_receive_offset = 0u64;
    let mut last_status = String::new();
    let mut skipped = 0;
    let mut output = io::stdout().lock();
    loop {
        let status = session.status.borrow().clone();
        let message = format!("{:?}: {}", status.state, status.message);
        if message != last_status {
            write!(output, "\r\n[{}]\r\n", safe_text(&message))?;
            last_status = message;
        }
        if status.skipped_display_bytes != skipped {
            write!(
                output,
                "\r\n[Display skipped {} bytes; inspect raw recording if enabled]\r\n",
                status.skipped_display_bytes - skipped
            )?;
            skipped = status.skipped_display_bytes;
        }
        for _ in 0..64 {
            let Ok(data) = session.events.try_recv() else {
                break;
            };
            if data.direction == Direction::Control {
                if data.bytes.starts_with(b"CONNECTED ") {
                    *decoder = TextDecoder::new(&profile.encoding)?;
                    at_line_start = true;
                }
                write!(
                    output,
                    "\r\n[Control: {}]\r\n",
                    safe_text(&String::from_utf8_lossy(&data.bytes))
                )?;
                continue;
            }
            if data.direction == Direction::Sent {
                if display == DisplayMode::Ansi {
                    continue;
                }
                write!(
                    output,
                    "\r\n[TX {} bytes; RX {} / TX {}]\r\n",
                    data.bytes.len(),
                    status.received,
                    status.sent
                )?;
                continue;
            }
            if data.offset != next_receive_offset {
                *decoder = TextDecoder::new(&profile.encoding)?;
                at_line_start = true;
            }
            next_receive_offset = data.offset + data.bytes.len() as u64;
            match display {
                DisplayMode::Ansi => write!(output, "{}", decoder.decode(&data.bytes, false))?,
                DisplayMode::Text => {
                    let decoded = decoder.decode(&data.bytes, false);
                    let text = if timestamps {
                        timestamped_text(&decoded, data.at_ms, &mut at_line_start)
                    } else {
                        safe_text(&decoded)
                    };
                    write!(output, "{text}")?;
                }
                DisplayMode::Hex => {
                    let mut offset = data.offset;
                    for bytes in data.bytes.chunks(16) {
                        write!(output, "{}", hex_line(offset, bytes))?;
                        offset += bytes.len() as u64;
                    }
                }
            }
        }
        output.flush()?;
        if matches!(status.state, SessionState::Failed | SessionState::Closed) {
            if !session.events.is_empty() {
                continue;
            }
            let tail = decoder.decode(&[], true);
            write!(output, "{}", safe_text(&tail))?;
            if status.state == SessionState::Failed {
                anyhow::bail!("{}", status.message);
            }
            return Ok(());
        }
        if !event::poll(Duration::from_millis(16))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('t') => {
                    timestamps = !timestamps;
                    at_line_start = true;
                    write!(output, "\r\n[Timestamps: {timestamps}]\r\n")?;
                    continue;
                }
                KeyCode::Char('x') => {
                    hex_send = !hex_send;
                    write!(
                        output,
                        "\r\n[Send mode: {}]\r\n",
                        if hex_send {
                            "HEX (no line ending added)"
                        } else {
                            "TEXT"
                        }
                    )?;
                    continue;
                }
                KeyCode::Char('v') => {
                    display = next_display(display);
                    *decoder = TextDecoder::new(&profile.encoding)?;
                    at_line_start = true;
                    input.clear();
                    control = false;
                    history_index = None;
                    write!(
                        output,
                        "\r\n[Display: {display:?}; keyboard: {}]\r\n",
                        if display == DisplayMode::Ansi && !hex_send {
                            "direct"
                        } else {
                            "line input; Enter sends"
                        }
                    )?;
                    continue;
                }
                KeyCode::Char(']') => {
                    input.clear();
                    control = true;
                    write!(
                        output,
                        "\r\n[Control: type dtr on/off, rts on/off or break; Enter confirms, Escape cancels]\r\n"
                    )?;
                    continue;
                }
                KeyCode::Char('c') if display != DisplayMode::Ansi || hex_send || control => {
                    input.clear();
                    control = false;
                    write!(output, "^C\r\n")?;
                    continue;
                }
                _ => {}
            }
        }
        if display == DisplayMode::Ansi && !control && !hex_send {
            let bytes = if let KeyCode::Char(c) = key.code
                && (!key.modifiers.contains(KeyModifiers::CONTROL) || !c.is_ascii())
            {
                encode_text(&c.to_string(), &profile.encoding, LineEnding::None)
            } else {
                Ok(ansi_key(key))
            };
            match bytes {
                Ok(mut bytes) if !bytes.is_empty() => {
                    if key.modifiers.contains(KeyModifiers::ALT)
                        && !key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        bytes.insert(0, 27);
                    }
                    queue(session, SerialCommand::Send(bytes), &mut output)?;
                }
                Ok(_) => {}
                Err(error) => write!(output, "\r\n[Not sent: {error}]\r\n")?,
            }
            continue;
        }
        match key.code {
            KeyCode::Esc => {
                input.clear();
                control = false;
                write!(output, "\r\n[Input cancelled]\r\n")?;
            }
            KeyCode::Backspace => {
                if input.pop().is_some() {
                    write!(output, "\x08 \x08")?;
                }
            }
            KeyCode::Up if !control && !history.is_empty() => {
                let index =
                    history_index.map_or(history.len() - 1, |index: usize| index.saturating_sub(1));
                input = history[index].clone();
                history_index = Some(index);
                write!(output, "\r\n[History input] {}", safe_text(&input))?;
            }
            KeyCode::Down if !control => {
                let index = history_index.map_or(history.len(), |index| index + 1);
                input = history.get(index).cloned().unwrap_or_default();
                history_index = (index < history.len()).then_some(index);
                write!(output, "\r\n[History input] {}", safe_text(&input))?;
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL) && input.len() < 8192 =>
            {
                input.push(c);
                history_index = None;
                write!(output, "{c}")?;
            }
            KeyCode::Enter => {
                write!(output, "\r\n")?;
                if control && input.trim() == "stats" {
                    let status = session.status.borrow().clone();
                    write!(
                        output,
                        "[RX {} / TX {} / display skipped {} bytes]\r\n",
                        status.received, status.sent, status.skipped_display_bytes
                    )?;
                    input.clear();
                    control = false;
                    continue;
                }
                if control && input.trim() == "templates" {
                    let names = profile
                        .templates
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                    write!(
                        output,
                        "[Send templates: {names}; type template NAME and Enter to send]\r\n"
                    )?;
                    input.clear();
                    control = false;
                    continue;
                }
                let command = if control {
                    match input.trim() {
                        "dtr on" => Ok(SerialCommand::Dtr(true)),
                        "dtr off" => Ok(SerialCommand::Dtr(false)),
                        "rts on" => Ok(SerialCommand::Rts(true)),
                        "rts off" => Ok(SerialCommand::Rts(false)),
                        "break" => Ok(SerialCommand::Break),
                        text if text.starts_with("template ") => {
                            match profile.templates.get(text.trim_start_matches("template ")) {
                                Some(template) => {
                                    profile.template_bytes(template).map(SerialCommand::Send)
                                }
                                None => Err(anyhow::anyhow!("Unknown send template; nothing sent")),
                            }
                        }
                        _ => Err(anyhow::anyhow!("Unknown control command; nothing sent")),
                    }
                } else if hex_send {
                    parse_hex(&input).map(SerialCommand::Send)
                } else {
                    encode_text(&input, &profile.encoding, profile.line_ending)
                        .map(SerialCommand::Send)
                };
                match command {
                    Ok(command) => {
                        if queue(session, command, &mut output)? && !control && !input.is_empty() {
                            if history.back() != Some(&input) {
                                history.push_back(input.clone());
                            }
                            if history.len() > 128 {
                                history.pop_front();
                            }
                        }
                    }
                    Err(error) => writeln!(output, "[Not sent: {error}]\r")?,
                }
                input.clear();
                control = false;
                history_index = None;
            }
            _ => {}
        }
    }
}

fn queue(session: &SerialSession, command: SerialCommand, output: &mut impl Write) -> Result<bool> {
    let active = session.status.borrow().state == SessionState::Active;
    if !active {
        writeln!(output, "[Not connected; nothing queued or replayed]\r")?;
        return Ok(false);
    } else if let Err(error) = session.commands.try_send(command) {
        writeln!(output, "[Not sent: command queue unavailable ({error})]\r")?;
        return Ok(false);
    }
    Ok(true)
}

fn ansi_key(key: event::KeyEvent) -> Vec<u8> {
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && let KeyCode::Char(c) = key.code
        && c.is_ascii()
    {
        return vec![(c as u8) & 0x1f];
    }
    match key.code {
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Backspace => vec![127],
        KeyCode::Tab => vec![9],
        KeyCode::Esc => vec![27],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        _ => Vec::new(),
    }
}

fn next_display(display: DisplayMode) -> DisplayMode {
    match display {
        DisplayMode::Ansi => DisplayMode::Text,
        DisplayMode::Text => DisplayMode::Hex,
        DisplayMode::Hex => DisplayMode::Ansi,
    }
}

#[cfg(test)]
#[path = "cli_unit_tests.rs"]
mod tests;
