use serde_json::json;

use super::*;

#[test]
fn preserves_custom_items_when_loading_retired_voice_controls() {
    let stored = json!({"Custom": {
        "left": ["FileExplorer", "VoiceInput", "FileAttach"],
        "right": ["VoiceInput", "Settings"]
    }});
    let selection: AgentToolbarChipSelection = serde_json::from_value(stored.clone()).unwrap();
    assert_eq!(
        selection,
        AgentToolbarChipSelection::Custom {
            left: vec![
                AgentToolbarItemKind::FileExplorer,
                AgentToolbarItemKind::FileAttach
            ],
            right: vec![AgentToolbarItemKind::Settings],
        }
    );
    let cli: CLIAgentToolbarChipSelection = serde_json::from_value(stored).unwrap();
    assert_eq!(
        cli,
        CLIAgentToolbarChipSelection::Custom {
            left: vec![
                AgentToolbarItemKind::FileExplorer,
                AgentToolbarItemKind::FileAttach
            ],
            right: vec![AgentToolbarItemKind::Settings],
        }
    );
}

#[test]
fn preserves_file_format_and_rejects_unrecognized_items() {
    let stored = json!({"custom": {
        "left": ["voice_input", "file_explorer"],
        "right": ["file_attach"]
    }});
    let selection = AgentToolbarChipSelection::from_file_value(&stored).unwrap();
    assert_eq!(
        selection.to_file_value(),
        json!({"custom": {
            "left": ["file_explorer"], "right": ["file_attach"]
        }})
    );
    assert_eq!(
        AgentToolbarChipSelection::from_file_value(&selection.to_file_value()),
        Some(selection)
    );
    let invalid = json!({"custom": {"left": ["unrecognized_control"], "right": []}});
    assert!(AgentToolbarChipSelection::from_file_value(&invalid).is_none());
}
