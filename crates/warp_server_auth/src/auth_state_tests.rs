use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use warp_core::settings::PrivatePreferences;
use warpui_core::App;
use warpui_extras::secure_storage;
use warpui_extras::user_preferences::in_memory::InMemoryPreferences;

use super::AuthState;

struct RecordingStorage(Arc<AtomicUsize>);

impl secure_storage::SecureStorage for RecordingStorage {
    fn read_value(&self, _: &str) -> Result<String, secure_storage::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok("previous-account-credential-fixture".into())
    }

    fn write_value(&self, _: &str, _: &str) -> Result<(), secure_storage::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn remove_value(&self, _: &str) -> Result<(), secure_storage::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn local_session_has_no_account_or_credentials_and_keeps_local_identity() {
    App::test((), |app| async move {
        app.add_singleton_model(|_| PrivatePreferences::new(Box::<InMemoryPreferences>::default()));
        app.read(|ctx| {
            let first = AuthState::initialize_local(ctx);
            let second = AuthState::initialize_local(ctx);
            assert!(!first.is_logged_in());
            assert!(first.user_id().is_none());
            assert!(first.credentials().is_none());
            assert_eq!(first.anonymous_id(), second.anonymous_id());
        });
    });
}

#[test]
fn local_session_does_not_consume_or_remove_stored_warp_credentials() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| PrivatePreferences::new(Box::<InMemoryPreferences>::default()));
        app.update(|ctx| {
            let accesses = Arc::new(AtomicUsize::new(0));
            ctx.add_singleton_model(|_| -> secure_storage::Model {
                Box::new(RecordingStorage(accesses.clone()))
            });
            let state = AuthState::initialize_local(ctx);
            assert!(!state.is_logged_in());
            assert!(state.credentials().is_none());
            assert_eq!(accesses.load(Ordering::SeqCst), 0);
        });
    });
}
