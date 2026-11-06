# 依赖分析

## 基线和启动调用链

- SHA：`f7fc2f2a08a3aa446baa23960710e630c5defd6e`，Windows x86_64，MSVC 2022，Rust 1.92.0。
- `script/run` → `cargo run --bin warp-oss --features gui` → `app/src/bin/oss.rs::main`
  → `app/src/lib.rs::run` → `run_internal` → PTY spawner → `initialize_app` → RootView / Workspace。
- `initialize_app` 读取本地 Settings → 注册 secure storage → AuthState → ServerApiProvider /
  NetworkLogModel / IAP → AuthManager → SQLite / PersistenceWriter → 云对象与本地会话恢复 →
  云 AI、团队、Drive、遥测和语音 singleton → `RootView::new` 的认证/引导状态机。
- `app/src/root_view.rs` 同时持有本地 Workspace 和 AuthOnboardingState；跳过一张登录页不能解除依赖。

## 已确认的网络入口与调用边界

| 入口 | 调用/依赖 | 处置 |
|---|---|---|
| `app/src/server/server_api.rs` | BaseClient、AuthClient、AIClient、ObjectClient、TeamClient、FactoryClient | 删除候选；先拆除 UI、持久化和 CLI 调用 |
| `crates/warp_server_client/src/base_client.rs` | 认证 HTTP/GraphQL、IAP、团队路由 | 删除候选；被主应用和 AI 客户端调用 |
| `crates/warp_server_auth/`、`app/src/auth/` | 账号、刷新令牌、SSO、认证窗口 | 删除候选；UserUid 用于持久化键和对象模型 |
| `app/src/server/cloud_objects/`、`crates/cloud_object_*/` | Drive 增量同步和本地对象恢复 | 替换；先区分本地对象存储与远程同步 |
| `app/src/settings/cloud_preferences_syncer.rs` | 本地 TOML 与账号云偏好同步 | 删除候选；不能移除本地 TOML 配置 |
| `crates/websocket/` | 共享会话、GraphQL subscription、远程 Agent | 待确认；保护后续 SSH 的通用传输边界 |
| `app/src/terminal/shared_session/` | Session sharing protocol / 远程协作 | 删除候选；与本地会话导航和 Pane 状态耦合 |
| `app/src/server/telemetry/` | collector → TelemetryApi → Rudder HTTP、事件磁盘队列 | 删除候选；事件枚举也被 UI 业务参数使用 |
| `app/src/tracing/` | OTLP exporter → cloud-agent-auth → managed-secrets，子进程凭据交接 | 可独立删除；保留通用 `tracing` instrument API |
| `crates/http_client/src/lib.rs` | 自动添加 `X-Warp-Traceparent` | 随 OTLP 删除；保留通用 HTTP wrapper |
| `app/src/autoupdate/` | ServerApi/channel_versions → 下载 → OS installer/relaunch | 删除候选；退出回调与菜单持有状态 |
| `app/src/voice/`、`crates/voice_input/` | 录音 → ServerVoiceTranscriber → Warp AI | 删除候选；设置、编辑器、全局热键均有入口 |
| `app/src/billing/`、`pricing/`、`settings_view/billing_and_usage*` | 配额、计划、团队权限 | 删除候选；权限枚举被 AI 和 Workspace 共用 |
| `crates/warp_tui/`、`app/src/tui*` | 独立前端 → `warp/tui` → 全部 initialize_app | 独立前端删除候选；共享 cell-grid 框架须逐项检查 |

## 资源和构建依赖

- `app/build.rs`：Windows conpty、dxcompiler、dxil、图标资源；macOS Objective-C、DockTilePlugin、Sentry；保留终端/GPU 资产和版权。
- `resources/bundled/`：包含通用 shell integration、语法/主题及 AI/Skill 资源，不能整个删除。
- `script/{linux,macos,windows}/bundle*`：GUI 与 TUI 分支混合；删除前需同时修改渠道配置、包名和 CI 调用。
- `.github/workflows/ci.yml`、`script/presubmit`：workspace feature unification 与 TUI test-util 关联。
- `Cargo.lock` 包含 1,574 个包；workspace 声明依赖 270 项，81 个本地 crate。
- 第一轮删除选择 OTLP：其唯一实际消费者是主应用追踪导出和 HTTP trace-link；不需要修改 PTY、终端模型锁、数据库或渲染框架。

## 验证边界

原版 Windows GUI 编译成功并显示欢迎页，debug exe 为 405,002,240 字节。
原版曾因本机缺 `protoc` 失败，安装仓库脚本同版本 protoc 25.1 后成功；这是环境问题。
`cargo-nextest` 尚未安装。跨平台验证需要 Oz runner；本机没有 `oz-dev`，不能声称 Linux/macOS 已验证。

## Workspace 正向和反向依赖

以下为 Cargo metadata 声明图，包含 target、optional、dev/build 依赖，不等于 Windows GUI 激活图。

| Crate | Workspace 直接依赖 | 被引用方 |
|---|---|---|
| ai | computer_use, http_client, languages, persistence, repo_metadata, settings_value, string-offset, syntax_tree, virtual-fs, warp_cli, warp_core, warp_errors, warp_graphql, warp_terminal, warp_util, warpui_core, warpui_extras, watcher | cloud_object_models, onboarding, warp, warp_tui |
| ai_types | — | persistence, warp |
| app-installation-detection | command | integration, warp |
| asset_cache | warp_errors, warpui_core | ui_components, warp, warp_editor, warpui |
| asset_macro | warp_util | warp |
| build_cache | command, warp_core, warp_errors | warp |
| channel_versions | — | warp, warp_terminal, warp_tui |
| cloud_object_client | cloud_object_models, cloud_objects, warp_core, warp_graphql | warp, warp_server_client |
| cloud_object_models | ai, cloud_object_persistence, cloud_objects, handlebars, persistence, settings, settings_value, warp_cli, warp_core, warp_errors, warp_graphql, warp_util | cloud_object_client, mcp, warp, warp_server_client |
| cloud_object_persistence | cloud_objects, persistence, warp_core, warp_graphql | cloud_object_models, warp |
| cloud_objects | settings_value, warp_core, warp_graphql, warp_server_auth, warpui_core | cloud_object_client, cloud_object_models, cloud_object_persistence, warp, warp_server_client |
| command | warp_errors | app-installation-detection, build_cache, computer_use, integration, local_control, lsp, node_runtime, remote_server, warp, warp_channel_config, warp_completer, warp_isolation_platform, warp_ripgrep, warp_server_client, warp_terminal, warp_tui, warp_util, warpui, warpui_core |
| computer_use | command, computer_use, warp_errors, warpui_core | ai, computer_use, warp |
| field_mask | — | warp |
| firebase | — | warp, warp_server_client |
| fuzzy_match | — | warp, warp_completer |
| handlebars | — | cloud_object_models, mcp, warp |
| http_client | prevent_sleep, warp_core, warp_errors | ai, lsp, node_runtime, warp, warp_graphql, warp_server_client, warp_tui |
| http_server | warp_core, warp_errors, warpui_core | warp |
| input_classifier | natural_language_detection, warp_completer, warp_errors, warp_features | warp |
| integration | app-installation-detection, command, settings, sum_tree, warp, warp_cli, warp_core, warpui, warpui_core, warpui_extras | — |
| ipc | warpui_core | warp |
| ipynb_parser | markdown_parser | warp_editor |
| jsonrpc | warp_errors, warpui_core | lsp |
| languages | warp_editor, warp_util | ai, syntax_tree, warp |
| local_control | command | warp, warp_cli |
| lsp | command, http_client, jsonrpc, node_runtime, repo_metadata, simple_logger, warp_core, warp_errors, warp_util, warpui_core | warp |
| managed_secrets_wasm | warp_managed_secrets | — |
| markdown_parser | — | ipynb_parser, warp, warp_editor, warp_tui, warpui, warpui_core |
| mcp | cloud_object_models, handlebars, simple_logger, warp_core, warp_errors, warpui, warpui_extras | warp |
| natural_language_detection | — | input_classifier, warp |
| node_runtime | command, http_client, warp_core | lsp |
| onboarding | ai, ui_components, warp_core, warp_errors, warp_logging, warpui, warpui_core | warp |
| persistence | ai_types | ai, cloud_object_models, cloud_object_persistence, warp, warp_graphql |
| prevent_sleep | — | http_client |
| regex_dfas | — | secret_redaction, warp_terminal |
| remote_server | command, repo_metadata, warp_core, warp_errors, warp_util, warpui_core | warp, warp_files |
| repo_metadata | virtual-fs, warp_core, warp_errors, warp_util, warpui, warpui_core, watcher | ai, lsp, remote_server, warp, warp_files |
| secret_redaction | regex_dfas, string-offset, warp_core, warp_errors | warp, warp_terminal |
| serve-wasm | — | — |
| settings | settings_value, warp_errors, warp_features, warpui_core, warpui_extras | cloud_object_models, integration, warp, warp_core |
| settings_value | settings_value_derive | ai, cloud_object_models, cloud_objects, settings, warp, warp_core, warp_server_client, warpui_core |
| settings_value_derive | — | settings_value |
| simple_logger | warp_core, warpui_core | lsp, mcp, warp |
| string-offset | — | ai, secret_redaction, syntax_tree, vim, warp, warp_completer, warp_core, warp_editor, warp_ripgrep, warp_search_core, warp_terminal, warp_tui, warpui_core |
| sum_tree | sum_tree, warp_errors | integration, sum_tree, warp, warp_editor, warp_tui, warpui, warpui_core |
| syntax_tree | languages, string-offset, warp_editor, warp_util, warpui, warpui_core | ai, warp |
| ui_components | asset_cache, warp_core, warpui, warpui_core | onboarding, warp |
| vim | string-offset, warp_core, warpui_core | warp, warp_editor, warp_tui |
| virtual-fs | — | ai, repo_metadata, warp, warpui |
| voice_input | warp_errors, warpui_core | warp |
| warp | ai, ai_types, app-installation-detection, asset_cache, asset_macro, build_cache, channel_versions, cloud_object_client, cloud_object_models, cloud_object_persistence, cloud_objects, command, computer_use, field_mask, firebase, fuzzy_match, handlebars, http_client, http_server, input_classifier, ipc, languages, local_control, lsp, markdown_parser, mcp, natural_language_detection, onboarding, persistence, remote_server, repo_metadata, secret_redaction, settings, settings_value, simple_logger, string-offset, sum_tree, syntax_tree, ui_components, vim, virtual-fs, voice_input, warp_assets, warp_channel_config, warp_cli, warp_completer, warp_core, warp_editor, warp_errors, warp_files, warp_graphql, warp_harness_usage, warp_isolation_platform, warp_logging, warp_managed_secrets, warp_multi_agent_client, warp_ripgrep, warp_search_core, warp_server_auth, warp_server_client, warp_terminal, warp_util, warp_web_event_bus, warpui, warpui_core, warpui_extras, watcher, websocket | integration, warp_tui |
| warp_assets | warpui_core | warp, warp_terminal |
| warp_channel_config | command, warp_core | warp, warp_tui |
| warp_cli | local_control, warp_core, warp_util | ai, cloud_object_models, integration, warp, warp_completer, warp_ripgrep, warp_terminal |
| warp_completer | command, fuzzy_match, string-offset, warp_cli, warp_completer, warp_core, warp_util, warpui_core | input_classifier, warp, warp_completer, warp_terminal, warp_tui |
| warp_core | settings, settings_value, string-offset, warp_errors, warp_features, warp_util, warpui_core, warpui_extras, websocket | ai, build_cache, cloud_object_client, cloud_object_models, cloud_object_persistence, cloud_objects, http_client, http_server, integration, lsp, mcp, node_runtime, onboarding, remote_server, repo_metadata, secret_redaction, simple_logger, ui_components, vim, warp, warp_channel_config, warp_cli, warp_completer, warp_editor, warp_files, warp_graphql, warp_isolation_platform, warp_logging, warp_managed_secrets, warp_multi_agent_client, warp_search_core, warp_server_auth, warp_server_client, warp_terminal, warp_tui |
| warp_editor | asset_cache, ipynb_parser, markdown_parser, string-offset, sum_tree, vim, warp_core, warp_errors, warp_util, warpui, warpui_core | languages, syntax_tree, warp, warp_tui |
| warp_errors | websocket | ai, asset_cache, build_cache, cloud_object_models, command, computer_use, http_client, http_server, input_classifier, jsonrpc, lsp, mcp, onboarding, remote_server, repo_metadata, secret_redaction, settings, sum_tree, voice_input, warp, warp_core, warp_editor, warp_graphql, warp_logging, warp_search_core, warp_server_auth, warp_server_client, warp_terminal, warp_tui, warp_util, warpui, warpui_core, warpui_extras, watcher |
| warp_features | — | input_classifier, settings, warp_core |
| warp_files | remote_server, repo_metadata, warp_core, warp_util, warpui_core, watcher | warp, warp_tui |
| warp_graphql | http_client, persistence, warp_core, warp_errors, warp_graphql_schema, websocket | ai, cloud_object_client, cloud_object_models, cloud_object_persistence, cloud_objects, warp, warp_managed_secrets, warp_server_auth, warp_server_client |
| warp_graphql_schema | — | warp_graphql |
| warp_harness_usage | — | warp |
| warp_isolation_platform | command, warp_core | warp, warp_managed_secrets, warp_server_client, warp_terminal |
| warp_logging | warp_core, warp_errors, warp_web_event_bus | onboarding, warp, warp_tui |
| warp_managed_secrets | warp_core, warp_graphql, warp_isolation_platform, warpui_core | managed_secrets_wasm, warp, warp_server_auth |
| warp_multi_agent_client | warp_core, warp_server_client | warp |
| warp_ripgrep | command, string-offset, warp_cli | warp |
| warp_search_core | string-offset, warp_core, warp_errors, warpui_core | warp, warp_tui |
| warp_server_auth | warp_core, warp_errors, warp_graphql, warp_managed_secrets, warpui_core, warpui_extras | cloud_objects, warp, warp_server_client |
| warp_server_client | cloud_object_client, cloud_object_models, cloud_objects, command, firebase, http_client, settings_value, warp_core, warp_errors, warp_graphql, warp_isolation_platform, warp_server_auth, warpui_core, websocket | warp, warp_multi_agent_client |
| warp_terminal | channel_versions, command, regex_dfas, secret_redaction, string-offset, warp_assets, warp_cli, warp_completer, warp_core, warp_errors, warp_isolation_platform, warp_util, warpui_core | ai, warp, warp_tui |
| warp_tui | ai, channel_versions, command, http_client, markdown_parser, string-offset, sum_tree, vim, warp, warp_channel_config, warp_completer, warp_core, warp_editor, warp_errors, warp_files, warp_logging, warp_search_core, warp_terminal, warp_util, warpui, warpui_core | — |
| warp_util | command, warp_errors, warpui_core | ai, asset_macro, cloud_object_models, languages, lsp, remote_server, repo_metadata, syntax_tree, warp, warp_cli, warp_completer, warp_core, warp_editor, warp_files, warp_terminal, warp_tui, warpui_core |
| warp_web_event_bus | — | warp, warp_logging |
| warpui | asset_cache, command, markdown_parser, sum_tree, virtual-fs, warp_errors, warpui, warpui_core | integration, mcp, onboarding, repo_metadata, syntax_tree, ui_components, warp, warp_editor, warp_tui, warpui |
| warpui_core | command, markdown_parser, settings_value, string-offset, sum_tree, warp_errors, warp_util, warpui_core | ai, asset_cache, cloud_objects, computer_use, http_server, integration, ipc, jsonrpc, lsp, onboarding, remote_server, repo_metadata, settings, simple_logger, syntax_tree, ui_components, vim, voice_input, warp, warp_assets, warp_completer, warp_core, warp_editor, warp_files, warp_managed_secrets, warp_search_core, warp_server_auth, warp_server_client, warp_terminal, warp_tui, warp_util, warpui, warpui_core, warpui_extras, watcher |
| warpui_extras | warp_errors, warpui_core, warpui_extras | ai, integration, mcp, settings, warp, warp_core, warp_server_auth, warpui_extras |
| watcher | warp_errors, warpui_core | ai, repo_metadata, warp, warp_files |
| websocket | — | warp, warp_core, warp_errors, warp_graphql, warp_server_client |
