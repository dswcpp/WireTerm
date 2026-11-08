use cynic::QueryBuilder;

use super::{GetRequestLimitInfo, GetRequestLimitInfoVariables};
use crate::queries::get_workspaces_metadata_for_user::{
    GetWorkspacesMetadataForUser, GetWorkspacesMetadataForUserVariables,
};
use crate::request_context::{ClientContext, OsContext, RequestContext};

fn request_context() -> RequestContext {
    RequestContext {
        client_context: ClientContext { version: None },
        os_context: OsContext {
            category: None,
            linux_kernel_version: None,
            name: None,
            version: None,
        },
    }
}

#[test]
fn request_limits_do_not_select_retired_voice_fields() {
    let query = GetRequestLimitInfo::build(GetRequestLimitInfoVariables {
        request_context: request_context(),
    })
    .query;
    assert!(query.contains("requestLimitInfo"));
    assert!(query.contains("requestsUsedSinceLastRefresh"));
    for field in [
        "isUnlimitedVoice",
        "voiceRequestLimit",
        "voiceRequestsUsedSinceLastRefresh",
    ] {
        assert!(
            !query.contains(field),
            "retired field {field} remains in query"
        );
    }
}

#[test]
fn workspace_policy_does_not_select_retired_voice_enablement() {
    let query = GetWorkspacesMetadataForUser::build(GetWorkspacesMetadataForUserVariables {
        request_context: request_context(),
    })
    .query;
    assert!(query.contains("warpAiPolicy"));
    assert!(query.contains("isNextCommandEnabled"));
    assert!(!query.contains("isVoiceEnabled"));
}
