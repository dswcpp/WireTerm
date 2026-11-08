use super::*;

fn serial() -> SerialProfile {
    SerialProfile {
        port: "COM10".into(),
        ..Default::default()
    }
}

#[test]
fn serial_defaults_preserve_control_lines_and_do_not_reconnect() {
    let profile = serial();
    profile.validate().unwrap();
    assert_eq!(profile.dtr, None);
    assert_eq!(profile.rts, None);
    assert!(!profile.auto_reconnect);
}

#[test]
fn invalid_serial_settings_are_rejected_before_opening_a_port() {
    for profile in [
        SerialProfile {
            baud_rate: 0,
            ..serial()
        },
        SerialProfile {
            data_bits: 9,
            ..serial()
        },
        SerialProfile {
            stop_bits: 3,
            ..serial()
        },
        SerialProfile {
            encoding: "invalid".into(),
            ..serial()
        },
        SerialProfile {
            parity: "invalid".into(),
            ..serial()
        },
    ] {
        assert!(profile.validate().is_err());
    }
}

#[test]
fn profiles_round_trip_without_overwriting_an_existing_profile() {
    let dir = tempfile::tempdir().unwrap();
    let profile = ConnectionProfile::Serial(serial());
    save(dir.path(), "board-1", &profile).unwrap();
    let ConnectionProfile::Serial(loaded) = load(dir.path(), "board-1").unwrap() else {
        panic!("Serial profile expected");
    };
    assert_eq!(loaded.port, "COM10");
    assert!(save(dir.path(), "board-1", &profile).is_err());
}

#[test]
fn profile_names_cannot_escape_the_configuration_directory() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["../other", "..", "a/b", "a\\b", "", "a:b", "C:\\pwn"] {
        assert!(save(dir.path(), name, &ConnectionProfile::Serial(serial())).is_err());
    }
}

#[test]
fn plaintext_passwords_are_not_accepted_as_ssh_profile_fields() {
    assert!(serde_json::from_str::<ConnectionProfile>(r#"{"transport":"Ssh","settings":{"host":"host","user":"user","port":22,"identity_file":null,"password":"secret"}}"#).is_err());
}

#[test]
fn deleting_a_profile_leaves_other_profiles_intact() {
    let dir = tempfile::tempdir().unwrap();
    let profile = ConnectionProfile::Serial(serial());
    save(dir.path(), "a", &profile).unwrap();
    save(dir.path(), "b", &profile).unwrap();
    assert_eq!(list(dir.path()).unwrap(), ["a", "b"]);
    delete(dir.path(), "a").unwrap();
    assert_eq!(list(dir.path()).unwrap(), ["b"]);
    assert!(load(dir.path(), "b").is_ok());
    assert!(delete(dir.path(), "../b").is_err());
}

#[test]
fn send_templates_are_validated_and_encoded_without_execution() {
    let mut profile = serial();
    profile.templates.insert(
        "reset".into(),
        SendTemplate {
            body: "AT+RST".into(),
            hex: false,
        },
    );
    profile.templates.insert(
        "binary".into(),
        SendTemplate {
            body: "00 FF".into(),
            hex: true,
        },
    );
    profile.validate().unwrap();
    assert_eq!(
        profile.template_bytes(&profile.templates["reset"]).unwrap(),
        b"AT+RST\r\n"
    );
    assert_eq!(
        profile
            .template_bytes(&profile.templates["binary"])
            .unwrap(),
        [0, 255]
    );
    profile.templates.insert(
        "broken".into(),
        SendTemplate {
            body: "F".into(),
            hex: true,
        },
    );
    assert!(profile.validate().is_err());
}

#[test]
fn oversized_templates_cannot_replace_a_readable_profile() {
    let directory = tempfile::tempdir().unwrap();
    let original = ConnectionProfile::Serial(serial());
    save(directory.path(), "board", &original).unwrap();
    let mut large = serial();
    for index in 0..9 {
        large.templates.insert(
            format!("command-{index}"),
            SendTemplate {
                body: "a".repeat(8192),
                hex: false,
            },
        );
    }
    assert!(update(directory.path(), "board", &ConnectionProfile::Serial(large)).is_err());
    let ConnectionProfile::Serial(loaded) = load(directory.path(), "board").unwrap() else {
        panic!("Serial profile expected");
    };
    assert!(loaded.templates.is_empty());
}
