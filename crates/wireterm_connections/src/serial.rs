use std::path::Path;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::sync::{mpsc, watch};
use tokio_serial::{DataBits, FlowControl, Parity, SerialPort, SerialPortBuilderExt, StopBits};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::profile::{SerialProfile, UsbIdentity};

const RECORDING_QUOTA: u64 = 256 * 1024 * 1024;
const RECORDING_SEGMENT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    Connecting,
    Active,
    Disconnected,
    Reconnecting,
    Closing,
    Closed,
    Failed,
}

#[derive(Clone, Debug)]
pub struct Status {
    pub state: SessionState,
    pub message: String,
    pub received: u64,
    pub sent: u64,
    pub skipped_display_bytes: u64,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum Direction {
    Received,
    Sent,
    Control,
}

#[derive(Debug, Serialize)]
pub struct DataEvent {
    pub session: Uuid,
    pub at_ms: u128,
    pub direction: Direction,
    pub offset: u64,
    pub bytes: Vec<u8>,
}

pub enum SerialCommand {
    Send(Vec<u8>),
    Dtr(bool),
    Rts(bool),
    Break,
}

pub struct SerialSession {
    pub commands: mpsc::Sender<SerialCommand>,
    pub events: mpsc::Receiver<DataEvent>,
    pub status: watch::Receiver<Status>,
    cancel: CancellationToken,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl SerialSession {
    pub fn spawn(profile: SerialProfile) -> Result<Self> {
        profile.validate()?;
        let (commands, receiver) = mpsc::channel(64);
        let (events_tx, events) = mpsc::channel(256);
        let status = Status {
            state: SessionState::Connecting,
            message: "Connecting".into(),
            received: 0,
            sent: 0,
            skipped_display_bytes: 0,
        };
        let (status_tx, status) = watch::channel(status);
        let cancel = CancellationToken::new();
        let cancellation = cancel.clone();
        let task = tokio::spawn(async move {
            if let Err(error) = run(profile, receiver, events_tx, &status_tx, &cancellation).await {
                status_tx.send_modify(|status| {
                    status.state = SessionState::Failed;
                    status.message = format!("{error:#}");
                });
            }
        });
        Ok(Self {
            commands,
            events,
            status,
            cancel,
            task: Some(task),
        })
    }

    pub async fn close(mut self) -> Result<()> {
        self.cancel.cancel();
        if let Some(task) = self.task.take() {
            task.await?;
        }
        Ok(())
    }
}

impl Drop for SerialSession {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

pub fn ports() -> Result<Vec<tokio_serial::SerialPortInfo>> {
    tokio_serial::available_ports().context("Cannot enumerate serial ports")
}

pub fn identity(info: &tokio_serial::SerialPortInfo) -> Option<UsbIdentity> {
    if let tokio_serial::SerialPortType::UsbPort(usb) = &info.port_type {
        usb.serial_number
            .as_ref()
            .filter(|number| !number.is_empty())
            .map(|number| UsbIdentity {
                vid: usb.vid,
                pid: usb.pid,
                serial_number: number.clone(),
            })
    } else {
        None
    }
}

pub fn capture_unique_identity(
    port: &str,
    available: &[tokio_serial::SerialPortInfo],
) -> Option<UsbIdentity> {
    let selected = available.iter().find(|info| {
        if cfg!(windows) {
            info.port_name.eq_ignore_ascii_case(port)
        } else {
            info.port_name == port
        }
    })?;
    let selected_identity = identity(selected)?;
    (available
        .iter()
        .filter(|info| identity(info).as_ref() == Some(&selected_identity))
        .count()
        == 1)
        .then_some(selected_identity)
}

pub fn resolve_port(
    profile: &SerialProfile,
    available: &[tokio_serial::SerialPortInfo],
) -> Result<String> {
    let Some(expected) = &profile.identity else {
        return Ok(profile.port.clone());
    };
    let matches: Vec<_> = available
        .iter()
        .filter(|port| identity(port).as_ref() == Some(expected))
        .collect();
    ensure!(
        matches.len() == 1,
        "The saved USB device is missing or its identity is ambiguous"
    );
    Ok(matches[0].port_name.clone())
}

fn open(profile: &SerialProfile, port: &str) -> Result<tokio_serial::SerialStream> {
    let data_bits = match profile.data_bits {
        5 => DataBits::Five,
        6 => DataBits::Six,
        7 => DataBits::Seven,
        8 => DataBits::Eight,
        _ => anyhow::bail!("Invalid data bits"),
    };
    let parity = match profile.parity.as_str() {
        "none" => Parity::None,
        "odd" => Parity::Odd,
        "even" => Parity::Even,
        _ => anyhow::bail!("Invalid parity"),
    };
    let stop_bits = match profile.stop_bits {
        1 => StopBits::One,
        2 => StopBits::Two,
        _ => anyhow::bail!("Invalid stop bits"),
    };
    let flow = match profile.flow_control.as_str() {
        "none" => FlowControl::None,
        "hardware" => FlowControl::Hardware,
        "software" => FlowControl::Software,
        _ => anyhow::bail!("Invalid flow control"),
    };
    let mut builder = tokio_serial::new(port, profile.baud_rate)
        .data_bits(data_bits)
        .parity(parity)
        .stop_bits(stop_bits)
        .flow_control(flow);
    builder = if let Some(dtr) = profile.dtr {
        builder.dtr_on_open(dtr)
    } else {
        builder.preserve_dtr_on_open()
    };
    let mut stream = builder
        .open_native_async()
        .with_context(|| format!("Cannot open {port} (check device, driver and port ownership)"))?;
    if let Some(rts) = profile.rts {
        stream.write_request_to_send(rts)?;
    }
    Ok(stream)
}

struct Recorder {
    file: tokio::fs::File,
    path: std::path::PathBuf,
    part: u16,
    size: u64,
    total: u64,
}

impl Recorder {
    async fn create(path: &Path) -> Result<Self> {
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        Ok(Self {
            file: options
                .open(path)
                .await
                .context("Cannot create recording; select a new filename")?,
            path: path.to_owned(),
            part: 0,
            size: 0,
            total: 0,
        })
    }

    async fn record(&mut self, event: &DataEvent) -> Result<()> {
        let mut bytes = serde_json::to_vec(event)?;
        bytes.push(b'\n');
        ensure!(
            self.total + bytes.len() as u64 <= RECORDING_QUOTA,
            "Recording quota reached (256 MiB); connection stopped to preserve recording integrity"
        );
        if self.size + bytes.len() as u64 > RECORDING_SEGMENT {
            self.file.sync_all().await?;
            let mut path = self.path.as_os_str().to_owned();
            path.push(format!(".part-{:04}", self.part + 1));
            let next = Self::create(Path::new(&path)).await?;
            self.file = next.file;
            self.part += 1;
            self.size = 0;
        }
        self.file
            .write_all(&bytes)
            .await
            .context("Recording failed; connection stopped")?;
        self.size += bytes.len() as u64;
        self.total += bytes.len() as u64;
        Ok(())
    }
}

async fn publish(
    session: Uuid,
    direction: Direction,
    bytes: &[u8],
    recorder: &mut Option<Recorder>,
    events: &mpsc::Sender<DataEvent>,
    status: &watch::Sender<Status>,
) -> Result<()> {
    let offset = match direction {
        Direction::Received => status.borrow().received,
        Direction::Sent => status.borrow().sent,
        Direction::Control => 0,
    };
    let event = DataEvent {
        session,
        direction,
        offset,
        at_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        bytes: bytes.to_vec(),
    };
    let count = bytes.len() as u64;
    status.send_modify(|status| match direction {
        Direction::Received => status.received += count,
        Direction::Sent => status.sent += count,
        Direction::Control => {}
    });
    if let Some(recorder) = recorder {
        recorder.record(&event).await?;
    }
    if events.try_send(event).is_err() {
        status.send_modify(|status| status.skipped_display_bytes += count);
    }
    Ok(())
}

fn state(status: &watch::Sender<Status>, value: SessionState, message: impl Into<String>) {
    status.send_modify(|status| {
        status.state = value;
        status.message = message.into();
    });
}

async fn run(
    mut profile: SerialProfile,
    mut commands: mpsc::Receiver<SerialCommand>,
    events: mpsc::Sender<DataEvent>,
    status: &watch::Sender<Status>,
    cancel: &CancellationToken,
) -> Result<()> {
    let session = Uuid::new_v4();
    let mut recorder = match &profile.recording {
        Some(path) => Some(Recorder::create(path).await?),
        None => None,
    };
    if profile.identity.is_none() {
        profile.identity = capture_unique_identity(&profile.port, &ports()?);
    }
    ensure!(
        !profile.auto_reconnect || profile.identity.is_some(),
        "Automatic reconnect requires a unique USB serial number; disable it when identity is missing or ambiguous"
    );
    let mut backoff = 1u64;
    let mut connected_once = false;
    'connection: loop {
        if cancel.is_cancelled() {
            break;
        }
        let port = if profile.identity.is_some() {
            ports().and_then(|available| resolve_port(&profile, &available))
        } else {
            Ok(profile.port.clone())
        };
        let opened = port.and_then(|port| open(&profile, &port).map(|stream| (port, stream)));
        let (port, mut stream) = match opened {
            Ok(value) => value,
            Err(error) if connected_once && profile.auto_reconnect => {
                state(
                    status,
                    SessionState::Reconnecting,
                    format!("Retry in {backoff}s: {error:#}"),
                );
                tokio::select! { _ = cancel.cancelled() => break, _ = tokio::time::sleep(Duration::from_secs(backoff)) => {} }
                backoff = (backoff * 2).min(30);
                while commands.try_recv().is_ok() {}
                continue;
            }
            Err(error) => return Err(error),
        };
        connected_once = true;
        backoff = 1;
        while commands.try_recv().is_ok() {}
        state(
            status,
            SessionState::Active,
            format!("Connected to {port} at {} baud", profile.baud_rate),
        );
        publish(
            session,
            Direction::Control,
            format!("CONNECTED {port}").as_bytes(),
            &mut recorder,
            &events,
            status,
        )
        .await?;
        let mut buffer = [0u8; 4096];
        let mut pending: Option<(Vec<u8>, usize, tokio::time::Instant)> = None;
        let mut break_until = None;
        loop {
            let write_deadline = pending
                .as_ref()
                .map(|(_, _, deadline)| *deadline)
                .unwrap_or_else(|| tokio::time::Instant::now() + Duration::from_secs(3600));
            let break_deadline = break_until
                .unwrap_or_else(|| tokio::time::Instant::now() + Duration::from_secs(3600));
            let outcome: Result<()> = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    if break_until.is_some() { stream.clear_break()?; }
                    break 'connection;
                }
                _ = tokio::time::sleep_until(write_deadline), if pending.is_some() => Err(anyhow::anyhow!("Serial write timed out; unsent bytes will not be replayed")),
                _ = tokio::time::sleep_until(break_deadline), if break_until.is_some() => {
                    stream.clear_break()?;
                    break_until = None;
                    publish(session, Direction::Control, b"BREAK cleared", &mut recorder, &events, status).await?;
                    Ok(())
                }
                command = commands.recv(), if pending.is_none() && break_until.is_none() => {
                    match command {
                        None => break 'connection,
                        Some(SerialCommand::Send(bytes)) => {
                            ensure!(bytes.len() <= 64 * 1024, "Send exceeds 64 KiB");
                            if !bytes.is_empty() {
                                pending = Some((bytes, 0, tokio::time::Instant::now() + Duration::from_secs(5)));
                            }
                            Ok(())
                        }
                        Some(SerialCommand::Dtr(value)) => {
                            stream.write_data_terminal_ready(value)?;
                            publish(session, Direction::Control, format!("DTR={value}").as_bytes(), &mut recorder, &events, status).await?;
                            Ok(())
                        }
                        Some(SerialCommand::Rts(value)) => {
                            ensure!(profile.flow_control != "hardware", "Manual RTS is unavailable with hardware flow control");
                            stream.write_request_to_send(value)?;
                            publish(session, Direction::Control, format!("RTS={value}").as_bytes(), &mut recorder, &events, status).await?;
                            Ok(())
                        }
                        Some(SerialCommand::Break) => {
                            stream.set_break()?;
                            break_until = Some(tokio::time::Instant::now() + Duration::from_millis(100));
                            publish(session, Direction::Control, b"BREAK asserted (100ms)", &mut recorder, &events, status).await?;
                            Ok(())
                        }
                    }
                }
                io = std::future::poll_fn(|ctx| poll_io(&mut stream, pending.as_ref().map(|(bytes, offset, _)| &bytes[*offset..]), &mut buffer, ctx)) => {
                    match io {
                        IoEvent::Written(Ok(0)) => Err(anyhow::anyhow!("Serial write returned zero bytes")),
                        IoEvent::Written(Ok(count)) => {
                            let (bytes, offset, _) = pending.as_mut().expect("write requires a pending send");
                            publish(session, Direction::Sent, &bytes[*offset..*offset + count], &mut recorder, &events, status).await?;
                            *offset += count;
                            if *offset == bytes.len() { pending = None; }
                            Ok(())
                        }
                        IoEvent::Read(Ok(0)) => Err(anyhow::anyhow!("Serial device disconnected")),
                        IoEvent::Read(Ok(count)) => {
                            publish(session, Direction::Received, &buffer[..count], &mut recorder, &events, status).await?;
                            Ok(())
                        }
                        IoEvent::Read(Err(error)) | IoEvent::Written(Err(error)) => Err(error.into()),
                    }
                }
            };
            if let Err(error) = outcome {
                state(status, SessionState::Disconnected, format!("{error:#}"));
                publish(
                    session,
                    Direction::Control,
                    format!("DISCONNECTED: {error:#}").as_bytes(),
                    &mut recorder,
                    &events,
                    status,
                )
                .await?;
                if !profile.auto_reconnect {
                    return Err(error);
                }
                drop(stream);
                while commands.try_recv().is_ok() {}
                state(
                    status,
                    SessionState::Reconnecting,
                    "Disconnected; retrying the same USB device in 1s (no commands replayed)",
                );
                tokio::select! { _ = cancel.cancelled() => break 'connection, _ = tokio::time::sleep(Duration::from_secs(1)) => {} }
                continue 'connection;
            }
        }
    }
    state(status, SessionState::Closing, "Closing");
    publish(
        session,
        Direction::Control,
        b"CLOSED",
        &mut recorder,
        &events,
        status,
    )
    .await?;
    if let Some(recorder) = &mut recorder {
        recorder.file.sync_all().await?;
    }
    state(status, SessionState::Closed, "Closed; port released");
    Ok(())
}

enum IoEvent {
    Read(std::io::Result<usize>),
    Written(std::io::Result<usize>),
}

fn poll_io(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    send: Option<&[u8]>,
    buffer: &mut [u8],
    ctx: &mut TaskContext<'_>,
) -> Poll<IoEvent> {
    if let Some(bytes) = send
        && let Poll::Ready(result) = Pin::new(&mut *stream).poll_write(ctx, bytes)
    {
        return Poll::Ready(IoEvent::Written(result));
    }
    let mut read = ReadBuf::new(buffer);
    match Pin::new(stream).poll_read(ctx, &mut read) {
        Poll::Ready(Ok(())) => Poll::Ready(IoEvent::Read(Ok(read.filled().len()))),
        Poll::Ready(Err(error)) => Poll::Ready(IoEvent::Read(Err(error))),
        Poll::Pending => Poll::Pending,
    }
}

#[cfg(test)]
#[path = "serial_tests.rs"]
mod tests;
