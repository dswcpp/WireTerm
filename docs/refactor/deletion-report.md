# P0 首批裁剪报告

## 当前结论

本分支完成已确认边界的实际源码删除：远程遥测发送链与独立 TUI 的构建、发布和运行入口。
**这不是完整 P0 验收交付。账号、Drive、云 AI、团队/计费、GUI 自动更新和语音仍有运行时依赖，
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
| 账号 / 认证 | `lib.rs::initialize_app`、`auth/`、`root_view.rs::AuthOnboardingState`、`warp_server_auth`；UserUid 参与 SQLite/对象键。需先定义本地身份及迁移契约，再删除认证 singleton 和引导状态机 |
| Drive / 云对象 | `cloud_object/model/persistence.rs` 同时恢复本地对象和云对象；`PersistenceWriter`、notebook/workflow/session state 共用模型。需先区分本地存储、待同步队列和远程所有权 |
| 云 AI / Oz | `ai/`、`ai_assistant/`、`warp_multi_agent_client`、`warp_server_client` 与输入、block list、Pane 视图共用类型。`terminal/view/ambient_agent/view_impl.rs` 仍读取 ServerApi，不能整体删目录 |
| 团队 / 计费 | `workspaces/`、`billing/`、`pricing/`、Settings 权限枚举和 LLM 可用性互相引用；需先分离本地功能可用性，不能将权限查询伪装为永久成功 |
| 同步设置 | `settings/cloud_preferences_syncer.rs` 与本地 TOML 写入/迁移共享入口，不能为删除同步而破坏本地配置 |
| 自动更新 / 语音 | GUI 菜单、退出/重启回调、编辑器、热键和 Settings 持有状态；本批仅删除其中遥测专用路径 |
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
