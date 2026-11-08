use std::io::Write;
use std::process::{Output, Stdio};

#[expect(
    clippy::disallowed_types,
    reason = "Tests pipe every stream and explicitly suppress Windows console creation"
)]
fn run(root: &std::path::Path, args: &[&str], input: &str) -> Output {
    use std::process::Command;
    let mut command = Command::new(env!("CARGO_BIN_EXE_wireterm-connect"));
    command
        .arg("--config-dir")
        .arg(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn serial_wizard_can_save_a_profile_without_opening_the_port() {
    let directory = tempfile::tempdir().unwrap();
    let mut answers = vec!["wireterm-test-port-that-does-not-exist"];
    answers.extend([""; 12]);
    answers.extend(["board-a", "no"]);
    let output = run(
        directory.path(),
        &["wizard", "serial"],
        &format!("{}\n", answers.join("\n")),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Connection cancelled"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Cannot open"));
    let wireterm_connections::profile::ConnectionProfile::Serial(profile) =
        wireterm_connections::profile::load(directory.path(), "board-a").unwrap()
    else {
        panic!("Serial profile expected");
    };
    assert!(!profile.auto_reconnect);
    assert_eq!(profile.dtr, None);
    assert_eq!(profile.port, "wireterm-test-port-that-does-not-exist");
}

#[test]
fn deleting_a_profile_requires_confirmation_and_preserves_recordings() {
    use wireterm_connections::profile::{ConnectionProfile, SshProfile, save};
    let directory = tempfile::tempdir().unwrap();
    save(
        directory.path(),
        "local-test",
        &ConnectionProfile::Ssh(SshProfile {
            host: "127.0.0.1".into(),
            user: None,
            port: 22,
            identity_file: None,
        }),
    )
    .unwrap();
    let recording = directory.path().join("recording.jsonl");
    std::fs::write(&recording, b"raw data").unwrap();
    let output = run(directory.path(), &["delete", "local-test"], "no\n");
    assert!(!output.status.success());
    assert!(directory.path().join("local-test.json").exists());
    let output = run(directory.path(), &["delete", "local-test"], "yes\n");
    assert!(output.status.success());
    assert!(!directory.path().join("local-test.json").exists());
    assert_eq!(std::fs::read(recording).unwrap(), b"raw data");
}

#[test]
fn creating_a_template_never_opens_its_serial_port() {
    use wireterm_connections::profile::{ConnectionProfile, SerialProfile, load, save};
    let directory = tempfile::tempdir().unwrap();
    save(
        directory.path(),
        "board-a",
        &ConnectionProfile::Serial(SerialProfile {
            port: "wireterm-port-does-not-exist".into(),
            ..Default::default()
        }),
    )
    .unwrap();
    let output = run(
        directory.path(),
        &["template", "board-a", "reset", "AT+RST"],
        "",
    );
    assert!(output.status.success(), "{:?}", output.stderr);
    let ConnectionProfile::Serial(profile) = load(directory.path(), "board-a").unwrap() else {
        panic!("Serial profile expected");
    };
    assert_eq!(profile.templates["reset"].body, "AT+RST");
}
