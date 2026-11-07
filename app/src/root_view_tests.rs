use warp_core::features::FeatureFlag;
use warpui::platform::WindowStyle;
use warpui::{App, SingletonEntity};

use super::{NewWorkspaceSource, RootView};
use crate::GlobalResourceHandles;
use crate::auth::AuthStateProvider;

#[test]
fn logged_out_window_opens_local_terminal() {
    let _force_login = FeatureFlag::ForceLogin.override_enabled(true);
    let _onboarding = FeatureFlag::AgentOnboarding.override_enabled(true);

    App::test((), |mut app| async move {
        crate::workspace::view::tests::initialize_app(&mut app);
        app.update(|ctx| {
            let state = AuthStateProvider::as_ref(ctx).get();
            state.set_user(None);
            state.set_credentials(None);
        });
        let resources = GlobalResourceHandles::mock(&mut app);
        let (_, root) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            RootView::new(
                resources,
                NewWorkspaceSource::Empty {
                    previous_active_window: None,
                    shell: None,
                },
                ctx,
            )
        });
        root.read(&app, |root, ctx| {
            let workspace = root.workspace_view().expect("local workspace").as_ref(ctx);
            let panes = workspace.active_tab_pane_group().as_ref(ctx);
            assert!(panes.active_session_view(ctx).is_some());
            assert!(!AuthStateProvider::as_ref(ctx).get().is_logged_in());
        });
    });
}
