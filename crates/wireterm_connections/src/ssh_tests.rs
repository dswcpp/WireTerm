use super::*;

fn profile() -> SshProfile {
    SshProfile {
        host: "example.test".into(),
        user: Some("user".into()),
        port: 2222,
        identity_file: Some("C:/keys/key with spaces".into()),
    }
}

#[test]
fn ssh_arguments_keep_host_verification_and_interactive_pty_enabled() {
    let args = arguments(&profile(), Path::new("C:/a b/known_hosts")).unwrap();
    let args: Vec<_> = args.iter().map(|arg| arg.to_str().unwrap()).collect();
    for arg in [
        "-tt",
        "StrictHostKeyChecking=ask",
        "UpdateHostKeys=no",
        "ControlMaster=no",
        "ServerAliveInterval=15",
        "UserKnownHostsFile=\"C:/a b/known_hosts\"",
    ] {
        assert!(args.contains(&arg), "{arg}");
    }
    assert!(
        args.iter()
            .any(|arg| arg.starts_with("IdentityFile=") && arg.contains("key with spaces"))
    );
    assert_eq!(args.last(), Some(&"example.test"));
    assert!(
        !args
            .iter()
            .any(|arg| arg.contains("remote-server") || arg.contains("StrictHostKeyChecking=no"))
    );
}

#[test]
fn hosts_and_usernames_cannot_inject_ssh_options() {
    for host in [
        "-oProxyCommand=bad",
        "host\nother",
        "host other",
        "user@host",
        "",
        "host/path",
    ] {
        assert!(
            arguments(
                &SshProfile {
                    host: host.into(),
                    ..profile()
                },
                Path::new("known_hosts")
            )
            .is_err()
        );
    }
    assert!(
        SshProfile {
            user: Some("-oProxyCommand=bad".into()),
            ..profile()
        }
        .validate()
        .is_err()
    );
    assert!(
        SshProfile {
            host: "::1".into(),
            ..profile()
        }
        .validate()
        .is_ok()
    );
    let args = arguments(
        &SshProfile {
            user: Some("developer@example.test".into()),
            ..profile()
        },
        Path::new("known_hosts"),
    )
    .unwrap();
    assert!(args.windows(2).any(|pair| pair
        == [
            OsString::from("-l"),
            OsString::from("developer@example.test")
        ]));
}

#[test]
fn known_host_paths_escape_openssh_tokens_and_reject_control_characters() {
    let args = arguments(&profile(), Path::new("known%hosts")).unwrap();
    assert!(args.contains(&OsString::from("UserKnownHostsFile=\"known%%hosts\"")));
    assert!(arguments(&profile(), Path::new("known\nhosts")).is_err());
    let args = arguments(
        &SshProfile {
            identity_file: Some("keys/100% ready".into()),
            ..profile()
        },
        Path::new("known_hosts"),
    )
    .unwrap();
    assert!(
        args.iter()
            .any(|arg| arg.to_string_lossy().contains("keys/100%% ready"))
    );
}
