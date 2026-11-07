use std::default::Default;

use warpui::fonts::Weight;
use warpui::keymap::FixedBinding;
use warpui::presenter::ChildView;
use warpui::ui_components::components::{Coords, UiComponentStyles};
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use super::shared_objects_creation_denied_body::{
    SharedObjectsCreationDeniedBody, SharedObjectsCreationDeniedBodyEvent,
};
use crate::drive::DriveObjectType;
use crate::modal::{Modal, ModalEvent};
use crate::server::ids::ServerId;
use crate::workspaces::user_workspaces::UserWorkspaces;

const DEFAULT_LIMIT_REACHED_MODAL_HEADER: &str = "Shared object limit reached";

pub struct SharedObjectsCreationDeniedModal {
    shared_objects_creation_denied_modal: ViewHandle<Modal<SharedObjectsCreationDeniedBody>>,
    team_uid: Option<ServerId>,
}

#[derive(Debug)]
pub enum SharedObjectsCreationDeniedModalAction {
    Close,
}

pub enum SharedObjectsCreationDeniedModalEvent {
    Close,
    TeamSettings,
}

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_fixed_bindings([FixedBinding::new(
        "escape",
        SharedObjectsCreationDeniedModalAction::Close,
        id!("SharedObjectsCreationDeniedModal"),
    )]);
}

impl SharedObjectsCreationDeniedModal {
    pub fn new(object_type: Option<DriveObjectType>, ctx: &mut ViewContext<Self>) -> Self {
        let shared_objects_creation_denied_body = ctx.add_typed_action_view(
            |_ctx: &mut ViewContext<'_, SharedObjectsCreationDeniedBody>| {
                SharedObjectsCreationDeniedBody::new(object_type)
            },
        );

        ctx.subscribe_to_view(
            &shared_objects_creation_denied_body,
            move |me, _, event, ctx| {
                me.handle_shared_objects_creation_denied_body_event(event, ctx);
            },
        );

        let shared_objects_creation_denied_modal = ctx.add_typed_action_view(|ctx| {
            Modal::new(
                Some(DEFAULT_LIMIT_REACHED_MODAL_HEADER.into()),
                shared_objects_creation_denied_body,
                ctx,
            )
            .with_modal_style(UiComponentStyles {
                width: Some(355.),
                ..Default::default()
            })
            .with_header_style(UiComponentStyles {
                font_size: Some(16.),
                font_weight: Some(Weight::Bold),
                padding: Some(Coords {
                    top: 24.,
                    bottom: 16.,
                    left: 24.,
                    right: 24.,
                }),
                ..Default::default()
            })
            .with_body_style(UiComponentStyles {
                padding: Some(Coords {
                    top: 0.,
                    bottom: 24.,
                    left: 24.,
                    right: 24.,
                }),
                ..Default::default()
            })
            .with_background_opacity(100)
            .with_dismiss_on_click()
        });
        ctx.subscribe_to_view(
            &shared_objects_creation_denied_modal,
            |me, _, event, ctx| match event {
                ModalEvent::Close => me.close(ctx),
            },
        );

        Self {
            shared_objects_creation_denied_modal,
            team_uid: None,
        }
    }

    pub fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(SharedObjectsCreationDeniedModalEvent::Close);
    }

    

    fn handle_shared_objects_creation_denied_body_event(
        &mut self,
        event: &SharedObjectsCreationDeniedBodyEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            SharedObjectsCreationDeniedBodyEvent::Upgrade => match self.team_uid {
                // If team_uid is set, then open up the upgrade page for the team
                // directly.
                Some(team_uid) => {
                    ctx.open_url(UserWorkspaces::upgrade_link_for_team(team_uid).as_str());
                }
                // Otherwise redirect them to the team settings page.
                None => ctx.emit(SharedObjectsCreationDeniedModalEvent::TeamSettings),
            },
            SharedObjectsCreationDeniedBodyEvent::ManageBilling => match self.team_uid {
                // If team_uid is set, then open up the manage billing page for the team
                // directly. The actual logic that opens the billing portal url in the
                // browser is in the handle_model_event method of TeamsPageView.
                Some(team_uid) => {
                    UserWorkspaces::handle(ctx).update(ctx, move |user_workspaces, ctx| {
                        user_workspaces.generate_stripe_billing_portal_link(team_uid, ctx);
                    });
                }
                // Otherwise redirect them to the team settings page.
                None => ctx.emit(SharedObjectsCreationDeniedModalEvent::TeamSettings),
            },
        }
    }
}

impl Entity for SharedObjectsCreationDeniedModal {
    type Event = SharedObjectsCreationDeniedModalEvent;
}

impl View for SharedObjectsCreationDeniedModal {
    fn ui_name() -> &'static str {
        "SharedObjectsCreationDeniedModal"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.shared_objects_creation_denied_modal).finish()
    }
}

impl TypedActionView for SharedObjectsCreationDeniedModal {
    type Action = SharedObjectsCreationDeniedModalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SharedObjectsCreationDeniedModalAction::Close => self.close(ctx),
        }
    }
}
