use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsbIdentity {
    pub vid: u16,
    pub pid: u16,
    pub serial_number: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, clap::ValueEnum, PartialEq, Eq)]
pub enum DisplayMode {
    #[default]
    Text,
    Hex,
    Ansi,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, clap::ValueEnum, PartialEq, Eq)]
pub enum LineEnding {
    None,
    Lf,
    #[default]
    Crlf,
    Cr,
}

impl LineEnding {
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Self::None => b"",
            Self::Lf => b"\n",
            Self::Crlf => b"\r\n",
            Self::Cr => b"\r",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SendTemplate {
    pub body: String,
    pub hex: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SerialProfile {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: u8,
    pub flow_control: String,
    pub dtr: Option<bool>,
    pub rts: Option<bool>,
    pub encoding: String,
    pub display: DisplayMode,
    pub send_hex: bool,
    pub timestamps: bool,
    pub line_ending: LineEnding,
    pub auto_reconnect: bool,
    pub identity: Option<UsbIdentity>,
    pub recording: Option<PathBuf>,
    pub templates: BTreeMap<String, SendTemplate>,
}

impl Default for SerialProfile {
    fn default() -> Self {
        Self {
            port: String::new(),
            baud_rate: 115200,
            data_bits: 8,
            parity: "none".into(),
            stop_bits: 1,
            flow_control: "none".into(),
            dtr: None,
            rts: None,
            encoding: "utf-8".into(),
            display: DisplayMode::Text,
            send_hex: false,
            timestamps: true,
            line_ending: LineEnding::Crlf,
            auto_reconnect: false,
            identity: None,
            recording: None,
            templates: BTreeMap::new(),
        }
    }
}

impl SerialProfile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.port.trim().is_empty() && !self.port.contains('\0'),
            "A serial port is required"
        );
        ensure!(
            (1..=4_000_000).contains(&self.baud_rate),
            "Baud rate must be 1..4000000"
        );
        ensure!((5..=8).contains(&self.data_bits), "Data bits must be 5..8");
        ensure!(matches!(self.stop_bits, 1 | 2), "Stop bits must be 1 or 2");
        ensure!(
            matches!(self.parity.as_str(), "none" | "odd" | "even"),
            "Parity must be none, odd or even"
        );
        ensure!(
            matches!(self.flow_control.as_str(), "none" | "hardware" | "software"),
            "Flow control must be none, hardware or software"
        );
        ensure!(
            self.flow_control != "hardware" || self.rts.is_none(),
            "Manual RTS cannot be combined with hardware flow control"
        );
        ensure!(
            encoding_rs::Encoding::for_label(self.encoding.as_bytes()).is_some(),
            "Unknown text encoding"
        );
        ensure!(
            self.templates.len() <= 128,
            "A profile supports at most 128 send templates"
        );
        for (name, template) in &self.templates {
            ensure!(
                !name.is_empty()
                    && name.len() <= 80
                    && name
                        .chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_')),
                "Invalid template name"
            );
            ensure!(template.body.len() <= 8192, "Template body exceeds 8 KiB");
            self.template_bytes(template)
                .with_context(|| format!("Invalid send template {name}"))?;
        }
        Ok(())
    }

    pub fn template_bytes(&self, template: &SendTemplate) -> Result<Vec<u8>> {
        if template.hex {
            crate::codec::parse_hex(&template.body)
        } else {
            crate::codec::encode_text(&template.body, &self.encoding, self.line_ending)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SshProfile {
    pub host: String,
    pub user: Option<String>,
    pub port: u16,
    pub identity_file: Option<PathBuf>,
}

impl SshProfile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.host.is_empty()
                && !self.host.starts_with('-')
                && !self
                    .host
                    .chars()
                    .any(|c| c.is_control() || c.is_whitespace())
                && !self.host.contains(['@', '/', '\\']),
            "Invalid SSH host"
        );
        ensure!(self.port != 0, "SSH port must be 1..65535");
        if let Some(user) = &self.user {
            ensure!(
                !user.is_empty()
                    && !user.starts_with('-')
                    && !user.chars().any(|c| c.is_control() || c.is_whitespace()),
                "Invalid SSH username"
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "transport", content = "settings", deny_unknown_fields)]
pub enum ConnectionProfile {
    Serial(SerialProfile),
    Ssh(SshProfile),
}

impl ConnectionProfile {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Serial(profile) => profile.validate(),
            Self::Ssh(profile) => profile.validate(),
        }
    }
}

pub fn config_directory() -> Result<PathBuf> {
    dirs::config_local_dir()
        .map(|dir| dir.join("WireTerm").join("connections"))
        .context("Cannot find the local configuration directory")
}

fn profile_path(root: &Path, name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty()
            && name.len() <= 80
            && name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')),
        "Profile names use letters, digits, '-' and '_' only"
    );
    Ok(root.join(format!("{name}.json")))
}

pub fn save(root: &Path, name: &str, profile: &ConnectionProfile) -> Result<()> {
    use std::io::Write;
    profile.validate()?;
    let bytes = serde_json::to_vec_pretty(profile)?;
    ensure!(
        bytes.len() <= 64 * 1024,
        "Combined profile and templates exceed 64 KiB"
    );
    let path = profile_path(root, name)?;
    std::fs::create_dir_all(root)?;
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    file.write_all(&bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path)
        .context("Cannot create profile (choose a new name to avoid overwriting)")?;
    Ok(())
}

pub fn load(root: &Path, name: &str) -> Result<ConnectionProfile> {
    let path = profile_path(root, name)?;
    ensure!(
        std::fs::metadata(&path)?.len() <= 64 * 1024,
        "Profile is too large"
    );
    let profile: ConnectionProfile = serde_json::from_slice(&std::fs::read(path)?)?;
    profile.validate()?;
    Ok(profile)
}

pub fn list(root: &Path) -> Result<Vec<String>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
            && let Some(name) = path.file_stem().and_then(|name| name.to_str())
            && profile_path(root, name).is_ok()
        {
            names.push(name.to_owned());
        }
    }
    names.sort();
    Ok(names)
}

pub fn delete(root: &Path, name: &str) -> Result<()> {
    std::fs::remove_file(profile_path(root, name)?).context("Cannot delete connection profile")
}

pub fn update(root: &Path, name: &str, profile: &ConnectionProfile) -> Result<()> {
    use std::io::Write;
    profile.validate()?;
    let bytes = serde_json::to_vec_pretty(profile)?;
    ensure!(
        bytes.len() <= 64 * 1024,
        "Combined profile and templates exceed 64 KiB"
    );
    let path = profile_path(root, name)?;
    ensure!(path.is_file(), "Profile does not exist");
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    file.write_all(&bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)
        .context("Cannot update connection profile")?;
    Ok(())
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
