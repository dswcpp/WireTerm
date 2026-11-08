use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;

use anyhow::{Context, Result, ensure};

use crate::profile::SshProfile;

pub fn arguments(profile: &SshProfile, known_hosts: &Path) -> Result<Vec<OsString>> {
    profile.validate()?;
    let mut args: Vec<OsString> = [
        "-tt",
        "-o",
        "StrictHostKeyChecking=ask",
        "-o",
        "UpdateHostKeys=no",
        "-o",
        "VerifyHostKeyDNS=no",
        "-o",
        "ControlMaster=no",
        "-o",
        "ControlPath=none",
        "-o",
        "PermitLocalCommand=no",
        "-o",
        "RemoteCommand=none",
        "-o",
        "ServerAliveInterval=15",
        "-o",
        "ServerAliveCountMax=3",
        "-o",
        "ConnectTimeout=15",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    args.extend([
        OsString::from("-o"),
        path_option("UserKnownHostsFile", known_hosts)?,
    ]);
    args.extend([OsString::from("-p"), profile.port.to_string().into()]);
    if let Some(user) = &profile.user {
        args.extend([OsString::from("-l"), user.into()]);
    }
    if let Some(identity) = &profile.identity_file {
        args.extend([
            OsString::from("-o"),
            "IdentitiesOnly=yes".into(),
            "-o".into(),
            path_option("IdentityFile", &std::path::absolute(identity)?)?,
        ]);
    }
    args.push(profile.host.clone().into());
    Ok(args)
}

fn path_option(option: &str, path: &Path) -> Result<OsString> {
    let path = path.to_str().context("SSH file path must be valid UTF-8")?;
    ensure!(
        !path.chars().any(char::is_control),
        "SSH file path contains control characters"
    );
    #[cfg(windows)]
    let path = path.replace('\\', "/");
    let path = path
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    Ok(format!("{option}=\"{path}\"").into())
}

#[expect(
    clippy::disallowed_types,
    reason = "OpenSSH must inherit the active PTY for passwords and resizing; the workspace wrapper forces CREATE_NO_WINDOW"
)]
pub fn connect(profile: &SshProfile, directory: &Path) -> Result<i32> {
    use std::process::Command;
    std::fs::create_dir_all(directory)?;
    let args = arguments(
        profile,
        &std::path::absolute(directory.join("known_hosts"))?,
    )?;
    eprintln!(
        "WireTerm SSH: verify the SHA256 host fingerprint before accepting a new server.\nCredentials are handled by OpenSSH and are not stored in connection profiles."
    );
    #[cfg(windows)]
    let executable = std::path::PathBuf::from(
        std::env::var_os("SystemRoot").context("Cannot find Windows system directory")?,
    )
    .join("System32")
    .join("OpenSSH")
    .join("ssh.exe");
    #[cfg(not(windows))]
    let executable = "ssh";
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context(
            "Cannot start OpenSSH. Install the system OpenSSH client and ensure ssh is on PATH",
        )?;
    let status = child.wait()?;
    if status.code() == Some(255) {
        eprintln!(
            "SSH connection failed. See OpenSSH's message above for host-key, authentication, timeout or network details. Reconnect explicitly after resolving the error."
        );
    }
    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
#[path = "ssh_tests.rs"]
mod tests;
