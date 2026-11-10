use warpui::elements::{
    ChildView, ClippedScrollStateHandle, ClippedScrollable, ConstrainedBox, Container,
    CrossAxisAlignment, Fill, Flex, MainAxisAlignment, MainAxisSize, ParentElement, ScrollbarWidth,
    Shrinkable, Text,
};
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::appearance::Appearance;
use crate::view_components::action_button::{
    ActionButton, DisabledTheme, NakedTheme, PrimaryTheme,
};
use crate::view_components::{Dropdown, DropdownItem};

#[derive(Clone, Debug, PartialEq)]
pub enum SerialConnectionAction {
    Select(usize, usize),
    Refresh,
    Connect,
    Cancel,
}

pub enum SerialConnectionEvent {
    Connect(Vec<String>),
    Close,
}

struct Field {
    label: &'static str,
    flag: &'static str,
    values: Vec<String>,
    selected: usize,
    dropdown: ViewHandle<Dropdown<SerialConnectionAction>>,
}

pub struct SerialConnectionModal {
    fields: Vec<Field>,
    scroll_state: ClippedScrollStateHandle,
    error: Option<String>,
    refresh: ViewHandle<ActionButton>,
    cancel: ViewHandle<ActionButton>,
    connect: ViewHandle<ActionButton>,
    disabled_connect: ViewHandle<ActionButton>,
}

impl SerialConnectionModal {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let refresh = ctx.add_typed_action_view(|_| {
            ActionButton::new("刷新串口", NakedTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(SerialConnectionAction::Refresh);
            })
        });
        let cancel = ctx.add_typed_action_view(|_| {
            ActionButton::new("取消", NakedTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(SerialConnectionAction::Cancel);
            })
        });
        let connect = ctx.add_typed_action_view(|_| {
            ActionButton::new("连接", PrimaryTheme).on_click(|ctx| {
                ctx.dispatch_typed_action(SerialConnectionAction::Connect);
            })
        });
        let disabled_connect =
            ctx.add_typed_action_view(|_| ActionButton::new("连接", DisabledTheme));
        let mut me = Self {
            fields: Vec::new(),
            scroll_state: Default::default(),
            error: None,
            refresh,
            cancel,
            connect,
            disabled_connect,
        };
        for (label, flag, values) in [
            ("串口", "--port", vec![]),
            (
                "波特率",
                "--baud",
                vec![
                    "115200", "9600", "19200", "38400", "57600", "230400", "460800", "921600",
                    "1200", "2400", "4800",
                ],
            ),
            ("数据位", "--data-bits", vec!["8", "7", "6", "5"]),
            ("校验位", "--parity", vec!["none", "odd", "even"]),
            ("停止位", "--stop-bits", vec!["1", "2"]),
            (
                "流控",
                "--flow-control",
                vec!["none", "hardware", "software"],
            ),
            ("编码", "--encoding", vec!["utf-8", "gbk"]),
            ("显示模式", "--display", vec!["ansi", "text", "hex"]),
            (
                "文本发送换行符",
                "--line-ending",
                vec!["crlf", "lf", "cr", "none"],
            ),
        ] {
            let index = me.fields.len();
            let dropdown = ctx.add_typed_action_view(Dropdown::new);
            let values: Vec<String> = values.into_iter().map(String::from).collect();
            dropdown.update(ctx, |dropdown, ctx| {
                dropdown.set_vertical_margin(0., ctx);
                dropdown.set_items(
                    values
                        .iter()
                        .enumerate()
                        .map(|(choice, value)| {
                            DropdownItem::new(
                                value.clone(),
                                SerialConnectionAction::Select(index, choice),
                            )
                        })
                        .collect(),
                    ctx,
                );
                if !values.is_empty() {
                    dropdown.set_selected_by_index(0, ctx);
                }
            });
            me.fields.push(Field {
                label,
                flag,
                values,
                selected: 0,
                dropdown,
            });
        }
        me.refresh_ports(ctx);
        me
    }

    fn refresh_ports(&mut self, ctx: &mut ViewContext<Self>) {
        let field = &mut self.fields[0];
        let previous = field.values.get(field.selected).cloned();
        let ports = match wireterm_connections::serial::ports() {
            Ok(ports) => {
                self.error = None;
                ports
            }
            Err(error) => {
                self.error = Some(format!("读取串口失败：{error}"));
                Vec::new()
            }
        };
        field.values = ports.iter().map(|port| port.port_name.clone()).collect();
        field.selected = previous
            .and_then(|name| field.values.iter().position(|value| *value == name))
            .unwrap_or(0);
        if field.values.is_empty() && self.error.is_none() {
            self.error = Some("未发现串口，请插入设备后点击刷新串口。".into());
        }
        field.dropdown.update(ctx, |dropdown, ctx| {
            dropdown.set_items(
                ports
                    .iter()
                    .enumerate()
                    .map(|(index, port)| {
                        DropdownItem::new(
                            port.port_name.clone(),
                            SerialConnectionAction::Select(0, index),
                        )
                    })
                    .collect(),
                ctx,
            );
            dropdown.set_selected_by_index(field.selected, ctx);
        });
        ctx.notify();
    }
}

impl Entity for SerialConnectionModal {
    type Event = SerialConnectionEvent;
}

impl View for SerialConnectionModal {
    fn ui_name() -> &'static str {
        "SerialConnectionModal"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let mut form = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);
        for field in &self.fields {
            form.add_child(
                Container::new(
                    Flex::row()
                        .with_main_axis_size(MainAxisSize::Max)
                        .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            Text::new(field.label, appearance.ui_font_family(), 14.)
                                .with_color(appearance.theme().foreground().into())
                                .finish(),
                        )
                        .with_child(ChildView::new(&field.dropdown).finish())
                        .finish(),
                )
                .with_margin_bottom(8.)
                .finish(),
            );
        }
        let theme = appearance.theme();
        let scrollable = ClippedScrollable::vertical(
            self.scroll_state.clone(),
            form.finish(),
            ScrollbarWidth::Auto,
            theme.nonactive_ui_text_color().into(),
            theme.active_ui_text_color().into(),
            Fill::None,
        )
        .with_overlayed_scrollbar()
        .finish();
        let mut column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(
                Shrinkable::new(
                    1.,
                    ConstrainedBox::new(scrollable)
                        .with_max_height(300.)
                        .finish(),
                )
                .finish(),
            );
        if let Some(error) = &self.error {
            column.add_child(
                Text::new(error.clone(), appearance.ui_font_family(), 14.)
                    .with_color(appearance.theme().foreground().into())
                    .soft_wrap(true)
                    .finish(),
            );
        }
        column.add_child(
            Container::new(
                Flex::row()
                    .with_main_axis_size(MainAxisSize::Max)
                    .with_main_axis_alignment(MainAxisAlignment::End)
                    .with_child(ChildView::new(&self.refresh).finish())
                    .with_child(ChildView::new(&self.cancel).finish())
                    .with_child(
                        ChildView::new(if self.fields[0].values.is_empty() {
                            &self.disabled_connect
                        } else {
                            &self.connect
                        })
                        .finish(),
                    )
                    .finish(),
            )
            .with_margin_top(16.)
            .finish(),
        );
        column.finish()
    }
}

impl TypedActionView for SerialConnectionModal {
    type Action = SerialConnectionAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match *action {
            SerialConnectionAction::Select(field, choice) => {
                self.fields[field].selected = choice;
                ctx.notify();
            }
            SerialConnectionAction::Refresh => self.refresh_ports(ctx),
            SerialConnectionAction::Cancel => ctx.emit(SerialConnectionEvent::Close),
            SerialConnectionAction::Connect => {
                let mut args = vec!["serial".into()];
                for field in &self.fields {
                    let Some(value) = field.values.get(field.selected) else {
                        return;
                    };
                    args.extend([field.flag.into(), value.clone()]);
                }
                ctx.emit(SerialConnectionEvent::Connect(args));
            }
        }
    }
}
