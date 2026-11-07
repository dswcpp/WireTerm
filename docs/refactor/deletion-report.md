# P0 首批裁剪报告

## P0 续作状态（2026-11-07）

本次从 `59289b393215592d71c133511220da29db2def99` 继续，独立分支
`codex/p0-source-pruning`。本批实际删除语音输入的完整运行链路以及独立 TUI 遗留的设置适配，
没有保留 `#[cfg]` 关闭的实现，也没有增加返回空结果的伪服务。

### 语音输入

| 边界 | 删除内容 |
|---|---|
| 采集与转写 crate | `crates/voice_input/`（3 文件）：CPAL 采集、WAV 编码、重采样、状态机；workspace 与 app 的依赖和 `gui` feature 绑定 |
| 应用单例与请求 | `app/src/voice/`、`app/src/ai/voice/`、`app/src/server/voice_transcriber.rs`、`ServerApi::transcribe`、`TranscribeError` 及 `/ai/transcribe` DTO |
| 编辑器 | `app/src/editor/view/voice.rs`、`EditorAction::ToggleVoiceInput`、`EditorEvent::VoiceStateUpdated`、语音光标图标、输入锁定和提示文本 |
| 终端与输入栏 | UDI 麦克风按钮、Agent 输入栏麦克风及 CLI 语音状态机、AltScreen/BlockList 语音按键、RootView 按住说话和全局动作 |
| 设置与入口 | `VoiceInputToggleKey`、`VOICE_INPUT_LANGUAGES`、`voice_input_enabled_internal`、`voice_input_language`、`entered_agent_mode_num_times`、新功能弹窗、`IS_VOICE_INPUT_ENABLED`、命令面板切换项、`/voice` 与 `TuiStatuslineItem::VoiceInput` |
| 配额、策略与遥测 | `RequestLimitInfo` 语音字段、`can_request_voice`、GraphQL/WarpAiPolicy 的 `is_voice_enabled`、三组语音遥测事件及 payload、Web 端语音入口来源枚举 |
| 资源 | `app/assets/bundled/svg/microphone.svg`、`Icon::Microphone` 及其路径映射、macOS 麦克风用途说明 |

语音的最小本地替代：无。录音、转写、按钮、快捷键、配额和请求都不属于本地终端必需能力，
直接删除；普通编辑器、终端输入、PTY 和会话逻辑保持原实现。旧本地 TOML 中的
`agents.voice.*` 键不再注册并由设置层按默认值忽略，没有清空数据库或改写用户配置。

### 独立 TUI 设置适配

删除 `app/src/settings/tui_autoupdate.rs`、`tui_theme.rs`、`tui_voice.rs`、`tui_zero_state.rs`
及三个测试文件，并移除 `settings::init`、测试初始化中的注册和 `pub use`。这些设置只服务于已
删除的独立 TUI 前端；共享的 WarpUI cell-grid 组件、`TuiZeroStateDataSource` 和终端适配保留。

旧 statusline 顺序中的 `voice_input` 序列化值不再合法；TUI 专用设置整体已删除，本地配置由
设置层按缺失字段回退，没有把云账户或同步状态迁移成本地数据。

用户原有设计文档改动与 `.spec-workflow/` 保留，不纳入本批提交。

### 依赖与规模

本批移除 Cargo 依赖：`voice_input`（workspace 与 app 声明）及 `gui = ["voice_input"]` 绑定；
新增依赖 0。锁文件删除 18 个包：`alsa`、`alsa-sys`、`coreaudio-rs`、`cpal`、`dasp_sample`、
`hound`、`mach2`、`objc2-audio-toolbox`、`objc2-avf-audio`、`objc2-core-audio`、
`objc2-core-audio-types`、`primal-check`、`realfft`、`rubato`、`rustfft`、`strength_reduce`、
`transpose`、`voice_input`；没有升级或新增包。

| 指标 | `59289b39` 基线 | 本批后 | 减少 |
|---|---:|---:|---:|
| Workspace crate | 80 | 79 | 1 |
| Cargo.lock 包 | 1,567 | 1,549 | 18 |
| Windows GUI 激活包（不含根 crate） | 1,005 | 994 | 11 |
| Rust 文件（跟踪，含新增兼容文件） | 3,931 | 3,915 | 16 |
| Rust 源码行（跟踪，含新增兼容文件） | 1,683,469 | 1,678,681 | 4,788（0.28%） |

文件统计另含 1 个 `Cargo.toml` 与 1 个 SVG，共删除 20 个跟踪文件；同时新增
`session_settings/toolbar_compat.rs` 及测试（131 行），用于在反序列化时过滤旧工具栏配置中的
`VoiceInput` 项，避免用户本地设置整份失效。净减少 16 个跟踪文件。行数按 Git blob 换行统计；
删除 18 个 `.rs` 文件的同时，保留文件内还删除了约 2,750 行语音分支与设置代码（含 rustfmt
收回的空白行）。Cargo.lock 的 214 行减少与 18 个包一致。
本批提交：`9a5596e1`，100 个文件，4,802 插入、5,423 删除。提交只包含本批路径；
工作树中与本批无关的并行格式化改动未纳入。
机器可读数据见 [p0-continuation-current.json](p0-continuation-current.json)、
[p0-continuation-deleted-files.txt](p0-continuation-deleted-files.txt) 和
[p0-gui-dependencies.txt](p0-gui-dependencies.txt)。本批未重新生成 debug exe，因此不提供
可执行文件大小对比，也不将基线 exe 当作裁剪后结果。

## 第二批实际删除与本地替代

本批处理用户指出的残留 Sign in 入口：删除 RootView 的认证/引导状态机、GUI 登录/SSO/token/
认证错误组件、Account 设置页与原退出登录菜单，并将桌面启动替换为直接打开本地 Workspace。
同时删除左侧 Drive/云会话列表的构造、订阅、视图实现、命名/清空回收站 UI，以及不再使用的
云 leave/empty-trash/rename-folder 操作和专用测试。完整路径清单见
[batch-2-deleted-files.txt](batch-2-deleted-files.txt)；总删除列表仍见 deleted-files.txt。

最小替代实现：

- `AuthState::initialize_local`：无账号、无凭据，复用既有稳定本地 UUID；不读取或删除旧 Warp 凭据。
- `RootView::new`：直接创建 Workspace；保留窗口/Quake/热键路由及 macOS 原有按键重复设置逻辑。
- 原本点击“在 Drive 中查看”改为已有缓存对象编辑器路由；没有新增云同步或伪成功服务。
- 旧 sidebar 序列化值兼容映射到 ProjectExplorer；旧 ToggleWarpDrive 快捷键标识仍供通用侧栏
  开关解析。没有清空 SQLite、设置或会话；旧账号数据未被迁移成伪造本地账户。
- 残余共享编辑器的账号/分享请求在本地边界明确报错；这些生产者和模型尚未全量删除。

本批移除 Cargo 依赖：0；新增依赖：0。云客户端 crate 仍有 AI、共享会话、对象持久化和 CLI
反向依赖，不能因 GUI 登录页已删除就宣称认证及云服务源码/运行时依赖已清零。
onboarding 资源仍被终端 callout/教程和 DTO 使用，保留并列为后续解耦项。
自动更新、团队/计费设置、云 AI/Oz 及可选 Sentry 仍未完成裁剪，完整 P0 尚未通过。

## 当前结论

本分支完成已确认边界的实际源码删除：远程遥测发送链、独立 TUI 与语音输入的构建、发布和运行入口。
**这不是完整 P0 验收交付。账号、Drive、云 AI、团队/计费及 GUI 自动更新仍有运行时依赖，
当前工程不能被描述为已完成的纯离线 WireTerm。**

基线：`f7fc2f2a08a3aa446baa23960710e630c5defd6e`。分支：`refactor/p0-local-terminal`。
用户原有未跟踪文件 `docs/WireTerm 系统详细设计方案.md` 未修改、未纳入提交。

## 实际删除

完整路径列表见 [deleted-files.txt](deleted-files.txt)。此列表由基线与候选源码差异生成，
包含 206 个删除路径；重命名的本地脱敏模块不计作删除。

| 边界 | 删除内容 |
|---|---|
| OTLP | `app/src/tracing.rs`、`app/src/tracing/` 的导出器、认证刷新、子进程凭据交接及专用测试 |
| Rudder | `app/src/server/telemetry/collector.rs`、`rudder_message.rs`、发送/落盘/重传实现和对应测试 |
| 遥测格式转换 | `app/src/server/telemetry_ext.rs` 与专用测试 |
| 即时上报 | 删除同步上报宏及其调用；删除登录通知、Agent 提示曝光 HTTP 接口和仅用于上报的 CLI 事件 |
| HTTP 追踪传播 | 删除自动添加 `X-Warp-Traceparent` 的实现、专用导出测试和依赖 |
| Windows 更新报告 | 删除安装日志远程上报、其解析器及专用测试，保留安装器本身和本地日志文件能力 |
| 独立 TUI | `crates/warp_tui/` 全部 190 个文件，包括 5 个渠道二进制、前端、测试、bench、build.rs 和 manifest |
| 本地运行脚本 | `script/run` 不再安装外部渠道配置或开发技能，直接构建 OSS GUI；保留平台封装和常规 Cargo 参数 |
| TUI 分发 | `script/run-tui`、Windows TUI 安装器及其测试，三个平台的 TUI 打包分支、专用 release profile、6 个发布任务 |

没有新增禁用开关，没有为删除的服务端 API 添加空实现。保留的 WarpUI 本地事件队列是既有框架
接口，有 1,024 条事件容量上限；其业务事件定义、账号上下文和同步设置仍待后续审计清除。

## Cargo 依赖

从 app 移除 `opentelemetry`、`opentelemetry-http`、`opentelemetry-otlp`、`opentelemetry_sdk`、
`tracing-opentelemetry`、`tracing-subscriber`；从 HTTP wrapper 移除 OTEL 传播和专用测试依赖。
同时清理 workspace 的相应声明。通用 `tracing` 仍被业务 instrumentation 和依赖库使用。

锁文件删除 7 个包，没有引入或升级包：`opentelemetry`、`opentelemetry-http`、
`opentelemetry-otlp`、`opentelemetry-proto`、`opentelemetry_sdk`、`tracing-opentelemetry`、`warp_tui`。
精确名称、版本与来源见 [removed-lock-packages.json](removed-lock-packages.json)。

`tracing-subscriber` 仍由 `app-installation-detection`、`serve-wasm` 等工具直接声明，
因此不能把它从锁文件整包删除。TUI 的通用依赖仍被其他保留 crate 使用，没有盲目删除。

## 最小本地提取

- 将遥测目录中被本地错误输出复用的字符串脱敏逻辑与测试迁入
  `crates/secret_redaction/src/diagnostic.rs` / `diagnostic_tests.rs`。
- 更新终端自定义规则、checkout 失败输出及 Unix 终止诊断的引用，删除仅用于遥测 JSON payload 的递归脱敏。
- 移除 TerminalView 仅为销毁时上传保留的后台执行器、计时字段和隐私快照。
  保留仍被云 Agent 子视图引用的 ServerApi 字段；没有修改 TerminalModel 锁、PTY 或 VT 解析器。
- 内存警告仍触发本地 UI 事件，保留既有 heap profiling 功能的按需明细接口。

本批没有改动 SQLite schema、会话序列化或本地 TOML 设置键，因此没有新增数据迁移。
渠道配置中移除了 Rudder 目的地址/写入密钥结构；既有渠道 JSON 中额外的 Rudder 字段由 Serde
按默认规则忽略。本地配置中的旧隐私/遥测设置键暂留，与相关业务模型一起列入后续清理，
没有通过清空本地数据库处理旧数据。

## 尚未完成与风险边界

这些项是未完成的工程工作，不是已删除模块，也不表示已通过离线验收。

| 保留项 | 真实耦合与下一步 |
|---|---|
| 账号 / 认证剩余 | 桌面已使用 Local 初始化并删除 RootView 登录状态机；`auth/` 后端、`warp_server_auth` 和 CLI/device flow 尚存，UserUid 仍参与 SQLite/对象键，需先分离身份/所有权和迁移契约 |
| Drive / 云对象 | `cloud_object/model/persistence.rs` 同时恢复本地对象和云对象；`PersistenceWriter`、notebook/workflow/session state 共用模型。需先区分本地存储、待同步队列和远程所有权 |
| 云 AI / Oz | `ai/`、`ai_assistant/`、`warp_multi_agent_client`、`warp_server_client` 与输入、block list、Pane 视图共用类型。`terminal/view/ambient_agent/view_impl.rs` 仍读取 ServerApi，不能整体删目录 |
| 团队 / 计费 | `workspaces/`、`billing/`、`pricing/`、Settings 权限枚举和 LLM 可用性互相引用；需先分离本地功能可用性，不能将权限查询伪装为永久成功 |
| 同步设置 | `settings/cloud_preferences_syncer.rs` 与本地 TOML 写入/迁移共享入口，不能为删除同步而破坏本地配置 |
| 自动更新 | GUI 菜单、退出/重启回调、热键和 Settings 持有状态尚存；语音输入已在 P0 续作中完整删除，不在此列 |
| 遥测剩余部分 | 框架事件队列、业务事件参数、PrivacySettings、Sentry 可选集成和 session sharing 中的上下文仍在；OTLP/Rudder 发送器已删除，不能据此宣称所有远程上报已清零 |
| TUI 适配残留 | `app/src/tui*`、`LaunchMode::Tui`、部分设置/API-key 桥接及 WarpUI cell-grid 能力尚存。本批只完成独立前端/分发删除，未将整个 TUI 相关符号标记为清零 |
| 共享底层组件 | WarpUI、VT/ANSI、PTY、Shell、异步任务、会话/Pane、本地日志和持久化继续保留；通用 HTTP/WebSocket/MCP 接口在确认未来扩展和当前引用前不整包删除 |

## 规模、提交与验证

原始统计见 [baseline.json](baseline.json)，裁剪源码统计见 [current.json](current.json)，
测试与实际运行细节见 [validation-report.md](validation-report.md)。

| 指标 | 原始基线 | 首批裁剪后 | 减少 |
|---|---:|---:|---:|
| Workspace crate | 81 | 80 | 1 |
| Cargo.lock 包 | 1,574 | 1,567 | 7 |
| Workspace 依赖声明 | 270 | 264 | 6 |
| Windows GUI normal/build 激活包（不含根 crate） | 1,011 | 1,005 | 6 |
| Rust 文件 | 4,161 | 3,959 | 202 |
| Rust 源码行 | 1,808,237 | 1,708,068 | 100,169（5.54%） |
| Rust 原始字节 | 69,820,048 | 65,869,122 | 3,950,926 |
| Windows debug exe 字节 | 405,002,240 | 402,686,464 | 2,315,776（0.57%） |

Rust 行数按跟踪文件实际换行计数，包含测试和生成源码，不是非空逻辑代码行；文件字节会受到
工作树换行影响。两次 exe 使用相同 dev profile 和 GUI feature，大小不代表发布包大小或性能。
current.json 的源码 SHA 是首批源码提交，报告附件随后独立提交，不影响 Rust 统计。

| 提交 | 内容 |
|---|---|
| `5038ea9c` | 基线与依赖审计；原始源码 SHA 保持为 `f7fc2f2a08a3aa446baa23960710e630c5defd6e` |
| `a1dacae0bd26ce34f22a062e622cadd010820f11` | 实删 OTLP/Rudder 发送链、独立 TUI、分发入口，提取本地诊断脱敏 |
| `docs(refactor): record first-batch deletion and validation evidence` | 最终四份报告与机器可读证据，提交记录可用 `git log refactor/p0-local-terminal` 查询 |

Windows GUI 离线 Cargo 构建、三项严格 Clippy 检查及 35 项相关测试通过；实际启动、已有会话恢复、
多会话/分屏和 PowerShell 回显完成 smoke 检查。新配置免登录、物理断网、跨平台及性能未通过验收。
版权及根目录许可证保持原样，Rudder SDK 原始许可证也保留。
