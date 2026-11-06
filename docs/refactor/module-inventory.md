# 模块清单

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
| voice_input | `crates/voice_input/Cargo.toml` | 删除候选（先解耦） | 13 |
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
| `app/src/voice` | 删除候选（检查调用链） |
| `app/src/voltron.rs` | 替换/待确认 |
| `app/src/warp_managed_paths_watcher.rs` | 替换/待确认 |
| `app/src/wasm_nux_dialog.rs` | 替换/待确认 |
| `app/src/window_settings.rs` | 替换/待确认 |
| `app/src/word_block_editor.rs` | 替换/待确认 |
| `app/src/workflows` | 替换/待确认 |
| `app/src/workspace` | 保留 |
| `app/src/workspaces` | 删除候选（检查调用链） |
