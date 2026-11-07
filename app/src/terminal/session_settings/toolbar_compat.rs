use serde::{Deserialize, Deserializer};
use serde_json::Value;
use settings_value::SettingsValue;

use super::{AgentToolbarChipSelection, CLIAgentToolbarChipSelection};
use crate::ai::blocklist::agent_view::toolbar_item::AgentToolbarItemKind;

fn read_items(value: &Value, file_format: bool) -> Option<Vec<AgentToolbarItemKind>> {
    value
        .as_array()?
        .iter()
        .filter(|item| !matches!(item.as_str(), Some("VoiceInput" | "voice_input")))
        .map(|item| {
            if file_format {
                AgentToolbarItemKind::from_file_value(item)
            } else {
                serde_json::from_value(item.clone()).ok()
            }
        })
        .collect()
}

macro_rules! impl_toolbar_compat {
    ($selection:ty) => {
        impl $selection {
            fn from_stored_value(value: &Value, file_format: bool) -> Option<Self> {
                let (default, custom) = if file_format {
                    ("default", "custom")
                } else {
                    ("Default", "Custom")
                };
                if value.as_str() == Some(default) {
                    return Some(Self::Default);
                }
                let custom = value.as_object()?.get(custom)?.as_object()?;
                Some(Self::Custom {
                    left: read_items(custom.get("left")?, file_format)?,
                    right: read_items(custom.get("right")?, file_format)?,
                })
            }
        }

        impl<'de> Deserialize<'de> for $selection {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = Value::deserialize(deserializer)?;
                Self::from_stored_value(&value, false)
                    .ok_or_else(|| serde::de::Error::custom("invalid toolbar selection"))
            }
        }

        impl SettingsValue for $selection {
            fn to_file_value(&self) -> Value {
                match self {
                    Self::Default => Value::String("default".to_owned()),
                    Self::Custom { left, right } => serde_json::json!({
                        "custom": {
                            "left": left.to_file_value(),
                            "right": right.to_file_value(),
                        }
                    }),
                }
            }

            fn from_file_value(value: &Value) -> Option<Self> {
                Self::from_stored_value(value, true)
            }
        }
    };
}

// Retired recording controls must not invalidate the rest of a user's saved toolbar.
impl_toolbar_compat!(AgentToolbarChipSelection);
impl_toolbar_compat!(CLIAgentToolbarChipSelection);

#[cfg(test)]
#[path = "toolbar_compat_tests.rs"]
mod tests;
