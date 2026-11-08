use super::*;
#[cfg(unix)]
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio_serial::{SerialPortInfo, SerialPortType, UsbPortInfo};

fn usb(name: &str, serial: &str) -> SerialPortInfo {
    SerialPortInfo {
        port_name: name.into(),
        port_type: SerialPortType::UsbPort(UsbPortInfo {
            vid: 0x1234,
            pid: 0x5678,
            serial_number: Some(serial.into()),
            manufacturer: None,
            product: None,
        }),
    }
}

#[test]
fn reconnect_follows_identity_not_an_arbitrary_new_com_port() {
    let profile = SerialProfile {
        port: "COM3".into(),
        identity: Some(UsbIdentity {
            vid: 0x1234,
            pid: 0x5678,
            serial_number: "board-a".into(),
        }),
        ..Default::default()
    };
    let devices = [usb("COM3", "board-b"), usb("COM7", "board-a")];
    assert_eq!(resolve_port(&profile, &devices).unwrap(), "COM7");
    assert!(resolve_port(&profile, &[usb("COM3", "board-b")]).is_err());
    assert!(resolve_port(&profile, &[usb("COM7", "board-a"), usb("COM8", "board-a")]).is_err());
}

#[test]
fn duplicate_usb_serial_numbers_do_not_prevent_explicit_manual_connections() {
    let devices = [usb("COM3", "duplicate"), usb("COM7", "duplicate")];
    assert!(capture_unique_identity("COM3", &devices).is_none());
    let manual = SerialProfile {
        port: "COM3".into(),
        ..Default::default()
    };
    assert_eq!(resolve_port(&manual, &devices).unwrap(), "COM3");
    let unique = [usb("COM3", "unique")];
    assert!(capture_unique_identity("COM3", &unique).is_some());
    assert_eq!(
        capture_unique_identity("com3", &unique).is_some(),
        cfg!(windows)
    );
}

#[tokio::test]
async fn a_blocked_transmitter_does_not_prevent_receiving_data() {
    let (mut port, mut peer) = tokio::io::duplex(1);
    port.write_all(&[0]).await.unwrap();
    peer.write_all(&[0x41]).await.unwrap();
    let mut buffer = [0u8; 16];
    let event = tokio::time::timeout(
        Duration::from_secs(1),
        std::future::poll_fn(|ctx| poll_io(&mut port, Some(&[1, 2, 3]), &mut buffer, ctx)),
    )
    .await
    .unwrap();
    assert!(matches!(event, IoEvent::Read(Ok(1))));
    assert_eq!(buffer[0], 0x41);
}

#[tokio::test]
async fn closing_before_connection_finishes_cannot_reopen_the_port() {
    let session = SerialSession::spawn(SerialProfile {
        port: "wireterm-port-that-does-not-exist".into(),
        auto_reconnect: true,
        ..Default::default()
    })
    .unwrap();
    let status = session.status.clone();
    session.close().await.unwrap();
    assert_ne!(status.borrow().state, SessionState::Active);
    assert_ne!(status.borrow().state, SessionState::Reconnecting);
}

#[tokio::test]
async fn recorder_preserves_raw_bytes_and_refuses_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.jsonl");
    let mut recorder = Recorder::create(&path).await.unwrap();
    let event = DataEvent {
        offset: 0,
        session: Uuid::new_v4(),
        direction: Direction::Received,
        at_ms: 42,
        bytes: vec![0, 255, 13, 10],
    };
    recorder.record(&event).await.unwrap();
    recorder.file.sync_all().await.unwrap();
    let recorded: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(&path).await.unwrap()).unwrap();
    assert_eq!(recorded["bytes"], serde_json::json!([0, 255, 13, 10]));
    assert!(Recorder::create(&path).await.is_err());
    recorder.total = RECORDING_QUOTA;
    assert!(recorder.record(&event).await.is_err());
}

#[tokio::test]
async fn slow_display_does_not_drop_recorded_bytes_or_block_close() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.jsonl");
    let mut recorder = Some(Recorder::create(&path).await.unwrap());
    let (sender, _receiver) = mpsc::channel(1);
    let (status, _) = watch::channel(Status {
        state: SessionState::Active,
        message: String::new(),
        received: 0,
        sent: 0,
        skipped_display_bytes: 0,
    });
    for _ in 0..3 {
        publish(
            Uuid::nil(),
            Direction::Received,
            &[1, 2],
            &mut recorder,
            &sender,
            &status,
        )
        .await
        .unwrap();
    }
    recorder.as_mut().unwrap().file.sync_all().await.unwrap();
    let records = tokio::fs::read_to_string(path).await.unwrap();
    assert_eq!(records.lines().count(), 3);
    let offsets: Vec<_> = records
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).unwrap()["offset"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(offsets, [0, 2, 4]);
    assert_eq!(status.borrow().received, 6);
    assert_eq!(status.borrow().skipped_display_bytes, 4);
}

#[tokio::test]
async fn rotating_a_recording_preserves_previous_segments_and_never_overwrites() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("capture.jsonl");
    let event = DataEvent {
        offset: 0,
        session: Uuid::nil(),
        direction: Direction::Received,
        at_ms: 1,
        bytes: vec![0x41],
    };
    let mut recorder = Recorder::create(&path).await.unwrap();
    recorder.record(&event).await.unwrap();
    recorder.size = RECORDING_SEGMENT;
    recorder.record(&event).await.unwrap();
    recorder.file.sync_all().await.unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 1);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("capture.jsonl.part-0001"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    recorder.size = RECORDING_SEGMENT;
    let next = directory.path().join("capture.jsonl.part-0002");
    std::fs::write(&next, b"existing recording").unwrap();
    assert!(recorder.record(&event).await.is_err());
    assert_eq!(std::fs::read(next).unwrap(), b"existing recording");
}

#[tokio::test]
async fn a_recording_failure_does_not_hide_bytes_already_written_to_the_device() {
    let directory = tempfile::tempdir().unwrap();
    let mut recorder = Some(
        Recorder::create(&directory.path().join("capture.jsonl"))
            .await
            .unwrap(),
    );
    recorder.as_mut().unwrap().total = RECORDING_QUOTA;
    let (events, mut receiver) = mpsc::channel(1);
    let (status, _) = watch::channel(Status {
        state: SessionState::Active,
        message: String::new(),
        received: 0,
        sent: 0,
        skipped_display_bytes: 0,
    });
    assert!(
        publish(
            Uuid::nil(),
            Direction::Sent,
            &[0x41, 0x42],
            &mut recorder,
            &events,
            &status
        )
        .await
        .is_err()
    );
    assert_eq!(status.borrow().sent, 2);
    assert!(receiver.try_recv().is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn virtual_serial_receives_sends_and_releases_the_port_without_implicit_writes() {
    let (mut peer, device) = tokio_serial::SerialStream::pair().unwrap();
    let path = device.name().unwrap();
    drop(device);
    let profile = SerialProfile {
        port: path.clone(),
        ..Default::default()
    };
    let mut session = SerialSession::spawn(profile.clone()).unwrap();
    let mut status = session.status.clone();
    tokio::time::timeout(Duration::from_secs(3), async {
        while status.borrow().state != SessionState::Active {
            status.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    let mut byte = [0u8];
    assert!(
        tokio::time::timeout(Duration::from_millis(50), peer.read(&mut byte))
            .await
            .is_err()
    );
    peer.write_all(&[0, 0xff, 0x41]).await.unwrap();
    let received = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let event = session.events.recv().await.unwrap();
            if event.direction == Direction::Received {
                break event;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(received.direction, Direction::Received);
    assert_eq!(received.bytes, [0, 0xff, 0x41]);
    session
        .commands
        .send(SerialCommand::Send(vec![0x12, 0x34]))
        .await
        .unwrap();
    let mut sent = [0u8; 2];
    tokio::time::timeout(Duration::from_secs(3), peer.read_exact(&mut sent))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sent, [0x12, 0x34]);
    tokio::time::timeout(Duration::from_secs(3), session.close())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.borrow().state, SessionState::Closed);
    assert!(open(&profile, &path).is_ok());
}
