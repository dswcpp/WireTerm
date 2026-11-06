use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

struct OptInProvider {
    enabled: AtomicBool,
    token: Option<&'static str>,
}

impl IapTokenProvider for OptInProvider {
    fn cached_token(&self) -> Option<String> {
        self.token.map(str::to_string)
    }

    fn routing_opt_in_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

#[test]
fn routing_opt_in_is_captured_at_builder_creation() {
    let provider = Arc::new(OptInProvider {
        enabled: AtomicBool::new(false),
        token: Some("test-iap-token"),
    });
    let mut client = Client::new();
    client.set_iap_token_provider(provider.clone());
    let url = "https://rtc.staging.warp.dev/api/v1/agent/events";

    let request = client.post(url).bearer_auth("app-token").build().unwrap();
    assert!(
        !request
            .wrapped
            .headers()
            .contains_key(iap::IAP_OPT_IN_HEADER)
    );
    provider.enabled.store(true, Ordering::Relaxed);
    let builder = client.post(url).bearer_auth("app-token");
    provider.enabled.store(false, Ordering::Relaxed);
    let request = builder.build().unwrap();
    assert_eq!(request.wrapped.headers()["x-warp-enable-iap"], "1");
    assert_eq!(request.wrapped.headers()[AUTHORIZATION], "Bearer app-token");

    let request = client.post(url).bearer_auth("app-token").build().unwrap();
    assert!(
        !request
            .wrapped
            .headers()
            .contains_key(iap::IAP_OPT_IN_HEADER)
    );
    assert_eq!(request.wrapped.headers()[AUTHORIZATION], "Bearer app-token");
    provider.enabled.store(true, Ordering::Relaxed);
    let request = client
        .get("https://example.com/stripe/webhook")
        .build()
        .unwrap();
    assert_eq!(request.wrapped.headers()[iap::IAP_OPT_IN_HEADER], "1");
    assert!(
        !request
            .wrapped
            .headers()
            .contains_key(iap::IAP_PROXY_AUTH_HEADER)
    );
}

#[test]
fn routing_opt_in_does_not_require_a_cached_token() {
    let mut client = Client::new();
    client.set_iap_token_provider(Arc::new(OptInProvider {
        enabled: AtomicBool::new(true),
        token: None,
    }));

    let request = client
        .get(ChannelState::server_root_url().into_owned())
        .build()
        .unwrap();

    assert_eq!(request.wrapped.headers()[iap::IAP_OPT_IN_HEADER], "1");
    assert!(
        !request
            .wrapped
            .headers()
            .contains_key(iap::IAP_PROXY_AUTH_HEADER)
    );
}

struct TestIapTokenProvider;

impl IapTokenProvider for TestIapTokenProvider {
    fn cached_token(&self) -> Option<String> {
        Some("test-iap-token".into())
    }
}

#[test]
fn external_client_omits_automatic_headers_and_preserves_signing_headers() {
    let mut client = Client::new();
    client.set_iap_token_provider(Arc::new(TestIapTokenProvider));
    let url = format!(
        "{}/object?X-Amz-Signature=test%2Bsignature&x-id=PutObject",
        ChannelState::server_root_url()
    );
    let api_request = client.get(&url).build().unwrap().wrapped;
    assert!(!api_request.headers().contains_key("x-warp-traceparent"));
    assert!(!api_request.headers().contains_key("traceparent"));
    assert!(
        api_request
            .headers()
            .contains_key(headers::WARP_OS_CATEGORY)
    );
    assert!(
        api_request
            .headers()
            .contains_key(iap::IAP_PROXY_AUTH_HEADER)
    );

    let external_client = Client::new_external();
    let request = external_client
        .put(&url)
        .header("x-amz-checksum-crc32c", "EjRWeA==")
        .header(AUTHORIZATION, "AWS4-HMAC-SHA256 test-signature")
        .build()
        .unwrap()
        .wrapped;
    assert_eq!(request.url().as_str(), url);
    assert_eq!(request.headers().len(), 2);
    assert_eq!(request.headers()["x-amz-checksum-crc32c"], "EjRWeA==");
    assert_eq!(
        request.headers()[AUTHORIZATION],
        "AWS4-HMAC-SHA256 test-signature"
    );
}
