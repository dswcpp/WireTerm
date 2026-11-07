# 模块清单

## P0 续作（2026-11-07）

续作基线为 `59289b393215592d71c133511220da29db2def99`，工作分支为
`codex/p0-source-pruning`。这次基线已经包含历史两批裁剪，不能将历史删除再次计入本批收益。
语音输入与独立 TUI 设置删除完成后，workspace 为 79 个 crate；完整的基线 crate、manifest、
声明依赖和反向 workspace 引用见 [p0-workspace-graph.json](p0-workspace-graph.json)。该图包含
optional、target、dev/build 声明，不等于桌面实际激活依赖图。

| 业务边界 | 处置 | 调用与保护边界 |
|---|---|---|
| `crates/voice_input` | 已删除 | 唯一生产消费者为 app GUI；CPAL 采集、WAV 编码与重采样属于语音业务 |
| `app/src/voice`、`app/src/ai/voice` | 已删除 | Editor → VoiceTranscriber → ServerVoiceTranscriber → `/ai/transcribe` 全链移除 |
| 编辑器、终端输入栏、RootView 语音分支 | 已删除 | 保留普通键盘事件、选择、光标、PTY 和终端模型锁的实现 |
| 语音设置、命令面板与工具栏 | 已删除 | 同时移除搜索入口、录音按钮、配额字段、遥测和全局模型注册 |
| 独立 TUI 的设置适配 | 已删除 | 接管并行工作留下的 TUI 专用设置删除；共享 WarpUI cell-grid 组件保留 |
| 自动更新 | 删除候选，尚未实施 | `app/src/autoupdate` 同时负责下载、退出前应用更新、子进程重启及版本比较；需保护退出警告和正常关闭 |
| 云身份和 Drive 模型 | 替换／待确认 | UserUid 同时用于本地 SQLite、对象所有权及云端共享对象，不将云数据自动改归本地用户 |
| 云 AI/Oz、团队、商业配额 | 删除候选，先解耦 | Pane、Blocklist、编辑器、CLI agent 输入和对象持久化共用类型，不能直接整目录删除 |
| WarpUI、VT/ANSI、PTY、本地 Shell/IO | 保留 | 本批不重写共享终端基础设施 |
| 本地配置、日志、SQLite、异步调度 | 保留 | 不清空用户数据，不重建数据库，不升级依赖 |

语音的删除明细、锁文件包和规模对比见 [deletion-report.md](deletion-report.md)；验证结果见
[validation-report.md](validation-report.md)。

## 第二批：桌面本地启动与账号/Drive 界面删除

桌面 `LaunchMode::App` 使用 `AuthInitialization::Local`；`RootView` 只持有本地
`Workspace`，不再构造账号登录、SSO、粘贴令牌或账号优先引导视图。登录页相关文件、
Account 设置页、Drive 侧栏主体/行渲染器/命名及清空回收站对话框、云会话列表已实际删除。
精确路径见 [batch-2-deleted-files.txt](batch-2-deleted-files.txt)。

| 模块 | 当前处置 | 反向依赖与保留边界 |
|---|---|---|
| 桌面 RootView 认证状态机 | 删除，直接构造 Workspace | 保留窗口、热键、多标签/Pane、Quake；语音修饰键处理已在 P0 续作删除 |
| GUI 登录/SSO/token/账号设置 | 删除实现、注册、菜单、回调入口 | CLI/TUI 残留适配和 API-key/device flow 尚存；第三方 MCP/ChatGPT OAuth 未混同为 Warp 账号登录 |
| `warp_server_auth::AuthState` | 最小本地初始化 | 无账户/凭据；保留稳定本地 UUID；既有 UserUid/credentials 类型仍被共享模型使用 |
| `drive/index.rs`、`panel.rs`、`items/item.rs` | 删除视图及专用测试 | sidebar 通用宽度常量移入 Workspace；保留被对象查询使用的 DriveIndexVariant |
| `drive/items` 共享模型适配 | 保留部分，删除侧栏点击/定位接口 | CloudObject、Notebook、Workflow、env-var/MCP 的展示/所有权接口仍引用 WarpDriveItem |
| 本地侧栏 | 保留项目文件树、全局搜索 | 旧存储的 Drive/云会话页签映射至文件树；保留旧快捷键序列化标识以兼容用户绑定 |
| 本地 SQLite/会话恢复 | 保留 | 未清空数据库，未篡改云对象所有者；云同步与本地对象存储仍待解耦 |
| AI onboarding 账号模型转换 | 删除无调用模块 | 通用 onboarding crate 仍被终端 callout/教程数据类型使用，不能整包删除 |
| 自动更新/云 AI/计费及团队设置 | 未完成裁剪 | 保留实际实现和调用，不将永久禁用开关算作删除；语音已在 P0 续作单独删除 |

原始 SHA：`f7fc2f2a08a3aa446baa23960710e630c5defd6e`。完整原始数据见 baseline.json。

保留指保护通用能力；删除候选必须先解除反向引用；替换指混合业务模块；待确认不授权整块删除。

## 首批执行状态

以下表格记录原始声明图；当前 workspace 为 80 个 crate。独立 `warp_tui` 已删除，OTLP/Rudder
发送器及专用依赖已删除。本地脱敏提取至既有 `secret_redaction::diagnostic`，未增加 crate。
其余标为删除候选的模块尚未完成，详情见 deletion-report.md；不能据此把候选当成已经删除。

## Cargo workspace

| Crate | Manifest | 处置 | 直接依赖数 |
|---|---|---|---:|
| ai | `crates/ai/Cargo.toml` | 替换/待确认 | 75 |
| ai_types | `crates/ai_types/Cargo.toml` | 替换/待确认 | 5 |
| app-installation-detection | `crates/app-installation-detection/Cargo.toml` | 替换/待确认 | 11 |
| asset_cache | `crates/asset_cache/Cargo.toml` | 保留 | 12 |
| asset_macro | `crates/asset_macro/Cargo.toml` | 保留 | 5 |
| build_cache | `crates/build_cache/Cargo.toml` | 替换/待确认 | 21 |
| channel_versions | `crates/channel_versions/Cargo.toml` | 保留 | 8 |
| cloud_object_client | `crates/cloud_object_client/Cargo.toml` | 删除候选（先解耦） | 12 |
| cloud_object_models | `crates/cloud_object_models/Cargo.toml` | 删除候选（先解耦） | 27 |
| cloud_object_persistence | `crates/cloud_object_persistence/Cargo.toml` | 删除候选（先解耦） | 12 |
| cloud_objects | `crates/cloud_objects/Cargo.toml` | 删除候选（先解耦） | 17 |
| command | `crates/command/Cargo.toml` | 保留 | 11 |
| computer_use | `crates/computer_use/Cargo.toml` | 替换/待确认 | 33 |
| field_mask | `crates/field_mask/Cargo.toml` | 替换/待确认 | 5 |
| firebase | `crates/firebase/Cargo.toml` | 删除候选（先解耦） | 2 |
| fuzzy_match | `crates/fuzzy_match/Cargo.toml` | 保留 | 1 |
| handlebars | `crates/handlebars/Cargo.toml` | 替换/待确认 | 0 |
| http_client | `crates/http_client/Cargo.toml` | 替换/待确认 | 25 |
| http_server | `crates/http_server/Cargo.toml` | 替换/待确认 | 9 |
| input_classifier | `crates/input_classifier/Cargo.toml` | 替换/待确认 | 25 |
| integration | `crates/integration/Cargo.toml` | 替换/待确认 | 40 |
| ipc | `crates/ipc/Cargo.toml` | 替换/待确认 | 14 |
| ipynb_parser | `crates/ipynb_parser/Cargo.toml` | 替换/待确认 | 4 |
| jsonrpc | `crates/jsonrpc/Cargo.toml` | 替换/待确认 | 9 |
| languages | `crates/languages/Cargo.toml` | 保留 | 7 |
| local_control | `crates/local_control/Cargo.toml` | 替换/待确认 | 11 |
| lsp | `crates/lsp/Cargo.toml` | 替换/待确认 | 32 |
| managed_secrets_wasm | `crates/managed_secrets_wasm/Cargo.toml` | 替换/待确认 | 5 |
| markdown_parser | `crates/markdown_parser/Cargo.toml` | 保留 | 8 |
| mcp | `crates/mcp/Cargo.toml` | 替换/待确认 | 29 |
| natural_language_detection | `crates/natural_language_detection/Cargo.toml` | 替换/待确认 | 3 |
| node_runtime | `crates/node_runtime/Cargo.toml` | 替换/待确认 | 16 |
| onboarding | `crates/onboarding/Cargo.toml` | 删除候选（先解耦） | 19 |
| persistence | `crates/persistence/Cargo.toml` | 保留 | 7 |
| prevent_sleep | `crates/prevent_sleep/Cargo.toml` | 保留 | 10 |
| regex_dfas | `crates/regex_dfas/Cargo.toml` | 保留 | 2 |
| remote_server | `crates/remote_server/Cargo.toml` | 替换/待确认 | 23 |
| repo_metadata | `crates/repo_metadata/Cargo.toml` | 替换/待确认 | 28 |
| secret_redaction | `crates/secret_redaction/Cargo.toml` | 保留（本地脱敏） | 10 |
| serve-wasm | `crates/serve-wasm/Cargo.toml` | 替换/待确认 | 8 |
| settings | `crates/settings/Cargo.toml` | 保留 | 13 |
| settings_value | `crates/settings_value/Cargo.toml` | 保留 | 6 |
| settings_value_derive | `crates/settings_value_derive/Cargo.toml` | 保留 | 4 |
| simple_logger | `crates/simple_logger/Cargo.toml` | 替换/待确认 | 10 |
| string-offset | `crates/string-offset/Cargo.toml` | 保留 | 3 |
| sum_tree | `crates/sum_tree/Cargo.toml` | 保留 | 5 |
| syntax_tree | `crates/syntax_tree/Cargo.toml` | 保留 | 13 |
| ui_components | `crates/ui_components/Cargo.toml` | 保留 | 9 |
| vim | `crates/vim/Cargo.toml` | 保留 | 7 |
| virtual-fs | `crates/virtual_fs/Cargo.toml` | 替换/待确认 | 4 |
| voice_input | `crates/voice_input/Cargo.toml` | 已删除（P0 续作） | 13 |
| warp | `app/Cargo.toml` | 替换/待确认 | 294 |
| warp_assets | `crates/warp_assets/Cargo.toml` | 保留 | 3 |
| warp_channel_config | `crates/warp_channel_config/Cargo.toml` | 替换/待确认 | 3 |
| warp_cli | `crates/warp_cli/Cargo.toml` | 替换/待确认 | 18 |
| warp_completer | `crates/warp_completer/Cargo.toml` | 替换/待确认 | 28 |
| warp_core | `crates/warp_core/Cargo.toml` | 保留 | 50 |
| warp_editor | `crates/editor/Cargo.toml` | 保留 | 49 |
| warp_errors | `crates/warp_errors/Cargo.toml` | 保留 | 8 |
| warp_features | `crates/warp_features/Cargo.toml` | 替换/待确认 | 1 |
| warp_files | `crates/warp_files/Cargo.toml` | 保留 | 15 |
| warp_graphql | `crates/graphql/Cargo.toml` | 删除候选（先解耦） | 22 |
| warp_graphql_schema | `crates/warp_graphql_schema/Cargo.toml` | 删除候选（先解耦） | 3 |
| warp_harness_usage | `crates/warp_harness_usage/Cargo.toml` | 删除候选（先解耦） | 3 |
| warp_isolation_platform | `crates/isolation_platform/Cargo.toml` | 替换/待确认 | 10 |
| warp_logging | `crates/warp_logging/Cargo.toml` | 保留 | 18 |
| warp_managed_secrets | `crates/managed_secrets/Cargo.toml` | 替换/待确认 | 22 |
| warp_multi_agent_client | `crates/warp_multi_agent_client/Cargo.toml` | 删除候选（先解耦） | 12 |
| warp_ripgrep | `crates/warp_ripgrep/Cargo.toml` | 替换/待确认 | 14 |
| warp_search_core | `crates/warp_search_core/Cargo.toml` | 替换/待确认 | 24 |
| warp_server_auth | `crates/warp_server_auth/Cargo.toml` | 删除候选（先解耦） | 16 |
| warp_server_client | `crates/warp_server_client/Cargo.toml` | 删除候选（先解耦） | 49 |
| warp_terminal | `crates/warp_terminal/Cargo.toml` | 保留 | 78 |
| warp_tui | `crates/warp_tui/Cargo.toml` | 删除候选（先解耦） | 55 |
| warp_util | `crates/warp_util/Cargo.toml` | 保留 | 30 |
| warp_web_event_bus | `crates/warp_web_event_bus/Cargo.toml` | 替换/待确认 | 5 |
| warpui | `crates/warpui/Cargo.toml` | 保留 | 96 |
| warpui_core | `crates/warpui_core/Cargo.toml` | 保留 | 77 |
| warpui_extras | `crates/warpui_extras/Cargo.toml` | 保留 | 23 |
| watcher | `crates/watcher/Cargo.toml` | 保留 | 7 |
| websocket | `crates/websocket/Cargo.toml` | 替换/待确认 | 25 |

## 应用业务模块

| 路径 | 处置 |
|---|---|
| `app/src/ai` | 替换/待确认 |
| `app/src/ai_assistant` | 替换/待确认 |
| `app/src/alloc.rs` | 替换/待确认 |
| `app/src/antivirus` | 替换/待确认 |
| `app/src/app_menus.rs` | 替换/待确认 |
| `app/src/app_services` | 替换/待确认 |
| `app/src/app_state.rs` | 保留 |
| `app/src/appearance.rs` | 保留 |
| `app/src/auth` | 删除候选（检查调用链） |
| `app/src/autoupdate` | 删除候选（检查调用链） |
| `app/src/banner` | 替换/待确认 |
| `app/src/billing` | 删除候选（检查调用链） |
| `app/src/bin` | 替换/待确认 |
| `app/src/changelog_model.rs` | 替换/待确认 |
| `app/src/channel.rs` | 替换/待确认 |
| `app/src/chip_configurator` | 替换/待确认 |
| `app/src/cloud_object` | 替换/待确认 |
| `app/src/code` | 替换/待确认 |
| `app/src/code_review` | 替换/待确认 |
| `app/src/coding_entrypoints` | 替换/待确认 |
| `app/src/coding_panel_enablement_state.rs` | 替换/待确认 |
| `app/src/command_palette.rs` | 替换/待确认 |
| `app/src/completer` | 替换/待确认 |
| `app/src/context_chips` | 替换/待确认 |
| `app/src/crash_recovery.rs` | 替换/待确认 |
| `app/src/crash_reporting` | 替换/待确认 |
| `app/src/debug_dump.rs` | 替换/待确认 |
| `app/src/default_terminal` | 替换/待确认 |
| `app/src/download_method.rs` | 替换/待确认 |
| `app/src/drive` | 删除候选（检查调用链） |
| `app/src/dynamic_libraries.rs` | 替换/待确认 |
| `app/src/editor` | 替换/待确认 |
| `app/src/env_vars` | 替换/待确认 |
| `app/src/experiments` | 替换/待确认 |
| `app/src/external_secrets` | 替换/待确认 |
| `app/src/features.rs` | 替换/待确认 |
| `app/src/font_fallback.rs` | 替换/待确认 |
| `app/src/global_resource_handles.rs` | 替换/待确认 |
| `app/src/gpu_state.rs` | 替换/待确认 |
| `app/src/input_classifier.rs` | 替换/待确认 |
| `app/src/input_suggestions.rs` | 替换/待确认 |
| `app/src/integration_testing` | 替换/待确认 |
| `app/src/interval_timer.rs` | 替换/待确认 |
| `app/src/keyboard.rs` | 保留 |
| `app/src/launch_configs` | 保留 |
| `app/src/lib.rs` | 替换/待确认 |
| `app/src/linear.rs` | 替换/待确认 |
| `app/src/local_control` | 替换/待确认 |
| `app/src/login_item` | 替换/待确认 |
| `app/src/menu.rs` | 替换/待确认 |
| `app/src/modal.rs` | 替换/待确认 |
| `app/src/network.rs` | 替换/待确认 |
| `app/src/notebooks` | 替换/待确认 |
| `app/src/notification.rs` | 替换/待确认 |
| `app/src/palette.rs` | 替换/待确认 |
| `app/src/pane_group` | 保留 |
| `app/src/persistence` | 保留 |
| `app/src/platform` | 保留 |
| `app/src/prefix.rs` | 替换/待确认 |
| `app/src/preview_config_migration.rs` | 替换/待确认 |
| `app/src/pricing` | 删除候选（检查调用链） |
| `app/src/profiling.rs` | 替换/待确认 |
| `app/src/projects.rs` | 替换/待确认 |
| `app/src/prompt` | 替换/待确认 |
| `app/src/quit_warning` | 替换/待确认 |
| `app/src/referral_theme_status.rs` | 删除候选（检查调用链） |
| `app/src/remote_server` | 替换/待确认 |
| `app/src/resource_center` | 替换/待确认 |
| `app/src/resource_limits.rs` | 替换/待确认 |
| `app/src/reward_view.rs` | 删除候选（检查调用链） |
| `app/src/root_view.rs` | 替换/待确认 |
| `app/src/safe_triangle.rs` | 替换/待确认 |
| `app/src/search` | 替换/待确认 |
| `app/src/search_bar.rs` | 替换/待确认 |
| `app/src/server` | 删除候选（检查调用链） |
| `app/src/session_management.rs` | 替换/待确认 |
| `app/src/settings` | 替换/待确认 |
| `app/src/settings_view` | 替换/待确认 |
| `app/src/shell_indicator.rs` | 替换/待确认 |
| `app/src/suggestions` | 替换/待确认 |
| `app/src/system` | 替换/待确认 |
| `app/src/system.rs` | 替换/待确认 |
| `app/src/tab.rs` | 保留 |
| `app/src/tab_configs` | 保留 |
| `app/src/terminal` | 保留 |
| `app/src/test_util` | 替换/待确认 |
| `app/src/themes` | 保留 |
| `app/src/throttle.rs` | 替换/待确认 |
| `app/src/tips` | 替换/待确认 |
| `app/src/tracing` | 删除候选（检查调用链） |
| `app/src/tracing.rs` | 删除候选（检查调用链） |
| `app/src/tui` | 删除候选（检查调用链） |
| `app/src/tui_export` | 删除候选（检查调用链） |
| `app/src/tui_export.rs` | 删除候选（检查调用链） |
| `app/src/tui_onboarding_markers.rs` | 删除候选（检查调用链） |
| `app/src/tui_test_support.rs` | 删除候选（检查调用链） |
| `app/src/ui_components` | 替换/待确认 |
| `app/src/undo_close` | 替换/待确认 |
| `app/src/uri` | 替换/待确认 |
| `app/src/usage` | 替换/待确认 |
| `app/src/user_config` | 保留 |
| `app/src/util` | 保留 |
| `app/src/view_components` | 保留 |
| `app/src/vim_registers.rs` | 替换/待确认 |
| `app/src/voice` | 已删除（P0 续作） |
| `app/src/voltron.rs` | 替换/待确认 |
| `app/src/warp_managed_paths_watcher.rs` | 替换/待确认 |
| `app/src/wasm_nux_dialog.rs` | 替换/待确认 |
| `app/src/window_settings.rs` | 替换/待确认 |
| `app/src/word_block_editor.rs` | 替换/待确认 |
| `app/src/workflows` | 替换/待确认 |
| `app/src/workspace` | 保留 |
| `app/src/workspaces` | 删除候选（检查调用链） |
