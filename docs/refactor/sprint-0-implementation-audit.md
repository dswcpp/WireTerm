# WireTerm Sprint 0 实施审计

日期：2026-11-07。依据：`docs/WireTerm 系统详细设计方案.md` 第 16、20 节。
本轮先审计，不改产品代码；以下状态不能替代 v0.1 或完整 P0 验收。

## 1. 当前候选与历史基线

- 本轮开始时 HEAD：`decfd77a9a115c3d25e382ac0bbe790b17cc381a`。
- 工作树已有 52 个跟踪文件改动及其他未跟踪文件，涉及认证、工作区、云对象、测试和设计文档。
  本轮保留这些改动，没有提交、切换分支或回退文件。HEAD 不能单独标识本轮检查的源码候选。
- 历史报告记录原始源码基线 `f7fc2f2a08a3aa446baa23960710e630c5defd6e`；尚未独立证明该值
  就是官方上游提交，后续应核实其来源和祖先关系，不能将本地报告中的 SHA 直接称为上游版本。
- 已有成果：遥测发送链、独立 TUI、语音输入裁剪；桌面本地身份初始化；登录界面和 Drive
  侧栏移除。证据见 `deletion-report.md`、`validation-report.md`，并与当前入口代码交叉核对。
- 这些历史成果意味着不应再次从原版开始删除；当前也不能认定整个云依赖已清除。

## 2. 构建环境与依赖证据

本轮实测：Rust/Cargo 1.92.0，host `x86_64-pc-windows-msvc`，LLVM 21.1.3，protoc 36.2。
`rust-toolchain.toml` 固定 1.92.0。应用 package 是 `warp`，默认二进制是 `warp-oss`，
真实 GUI 检查须指定 `--features gui`。

已执行并成功：

```powershell
cargo metadata --offline --locked --no-deps --format-version 1
cargo tree -p warp --features gui --target x86_64-pc-windows-msvc --edges normal,build --depth 2 --offline --locked
```

- [声明依赖图](sprint-0-workspace-graph.json)：79 个 workspace package，记录工作区内部依赖、
  dependency kind、optional 和 target 条件；这是声明图，不等于 GUI 的全部激活依赖。
- [Windows GUI 依赖树](sprint-0-gui-dependencies.txt)：normal/build 深度 2，按 Windows target
  和 GUI feature 解析；不包含完整深层依赖闭包，也不包含测试依赖。
- 仍直接依赖 `ai`、`cloud_objects`、`cloud_object_client`、`warp_server_auth`、
  `warp_server_client`、`warp_graphql`、`warp_multi_agent_client`、`mcp`。
- 保留候选是 `warp_terminal`、`warpui`、`warpui_core`、`warp_core`、`persistence`、
  `settings`、`ipc`、`secret_redaction`；其中部分仍有业务耦合，不能视为已完成独立化。

Windows 前置条件的仓库证据在 `script/windows/bootstrap.ps1:210`：MSVC 编译器、链接器及
Windows SDK；`:239` 的 protoc、`:241` 的 LLVM/libclang 支持生成代码和 bindgen。
脚本还涉及 CMake、Git/Bash 和打包工具。Inno Setup 属于安装包阶段；本轮未运行 bootstrap，
未安装全局依赖、修改环境变量或引入 gcloud。历史完整构建成功记录不替代当前工作树检查。

## 3. 启动与云服务边界

| 边界 | 当前证据 | 判定和实施方向 |
|---|---|---|
| OSS 入口 | `app/src/bin/oss.rs:10` 的 `main`，`:14` 的 `AppId`，`:16`/`:17` 的 production 配置 | 仍是 WarpOss 品牌和生产服务配置；需要独立产品配置 |
| 桌面初始化 | `app/src/lib.rs:1449` 的 `initialize_app`，`:1502` 调用 `AuthState::initialize_local` | 已有本地启动路径，复用而非重复重写 |
| 本地身份 | `crates/warp_server_auth/src/auth_state.rs:54`，内部 `new` 将 user/credentials 设为 None，复用匿名 ID | 不读旧凭据；不是已登录账号，也不是所有网络能力的总开关 |
| 网络客户端注册 | `app/src/lib.rs:1540` → `ServerApiProvider::new` → `ServerApi::new` → `new_with_parts` → `BaseClient::new` | 内部及 external HTTP 客户端均存在；构造客户端本身不能证明已发送请求 |
| 认证管理 | `app/src/lib.rs:1584` 的 `AuthManager::new` | 本地终端仍通过共享认证类型耦合业务；需逐消费者解除 |
| 云对象/团队 | `app/src/lib.rs:2283`/`:2291` 注册 `TeamUpdateManager`/`UpdateManager` | 云模型仍参与启动及本地对象持久化；不可整包删除 |
| 云对象轮询 | `app/src/server/cloud_objects/update_manager.rs:620`、`:673` | `poll_for_updated_objects` 在未登录时返回；登录路径通过 `fetch_changed_objects` 发起请求 |
| 强制刷新 | 同文件 `refresh_updated_objects`，`:630` 起 | 仍有独立调用路径；不能仅凭周期轮询保护推断全部对象请求被阻断 |
| 自动更新 | `app/src/root_view.rs:1517` → `AutoupdateState::start_polling`；`app/src/autoupdate/mod.rs:141` 检查 flag/执行模式 | OSS 入口的 autoupdate_config 为 None；保留代码不等于 OSS 已启用更新，后续需覆盖显式检查入口 |
| 崩溃/遥测 | OSS 入口 `telemetry_config`、`crash_reporting_config` 为 None | 当前入口关闭配置；其他 channel、错误报告和外部下载入口仍需逐项核查 |

生产目的地定义在 `crates/warp_core/src/channel/config.rs:60`、`:84`：
`app.warp.dev`、`rtc.app.warp.dev`、`sessions.app.warp.dev`、`oz.warp.dev`。
这是静态配置清单，不是抓包结果，也不是全部外联域名清单。

当前调用关系：

```text
oss::main → warp::run → run_internal → initialize_app
  ├─ AuthState::initialize_local → 无账号/无凭据
  ├─ ServerApiProvider → ServerApi → BaseClient → HTTP/GraphQL 客户端
  ├─ TeamUpdateManager / UpdateManager → 对象与团队业务
  └─ RootView → Workspace → Pane / 本地终端
```

## 4. 终端、SSH、Skill 复用入口

- GUI 工作区：`app/src/workspace/view.rs`、`app/src/pane_group/`；终端适配在
  `app/src/terminal/local_tty/`。保留标签/分屏与现有实体句柄模式。
- Windows PTY：`crates/warp_terminal/src/local_tty/windows/mod.rs:164` 的 `spawn`、
  `:386` 的 `Pty::new`，以及相邻 `conpty_api.rs`。串口不能直接冒充带 shell 语义的 PTY。
- SSH：`app/src/remote_server/ssh_transport.rs:33` 的 `SshTransport` 使用 ControlMaster，
  `:93` 生成 `remote-server-proxy` 命令。这是远端服务传输，不满足独立远程 PTY 的完整目标。
- Skill：`crates/ai/src/skills/` 是解析/加载复用候选，不等于最终用户的导入、校验、授权和卸载服务。
- 持久化：`crates/persistence/src/schema.rs` 和 `migrations/` 已存在，继续评估 Diesel 复用；
  不提前切换数据库栈，也不把旧云对象无条件归属给本地用户。

## 5. CI、许可与设计待决项

- `.github/workflows/ci.yml:57` 附近限制 `github.repository_owner == 'warpdotdev'`。
  原 CI 文件存在不代表 Fork 可运行；需建立不依赖内部 runner、密钥和发布服务的 Windows CI。
- `Cargo.toml:27` 声明 AGPL-3.0-only，WarpUI 两个 manifest 声明 MIT，根目录保留
  `LICENSE-AGPL`、`LICENSE-MIT`。本轮仅核实文件与声明，未完成第三方许可证扫描或法律审核。
  发布前核对对应源码、第三方 notices 和品牌资产；不要通过重命名删除原作者声明。
- 方案 §5.1 要求区分 Disconnected/Closed，但 §5.2 枚举没有 Disconnected；实现状态机前统一。
- 方案 §6.3 在连接成功后分配 session_id，与 Created/Connecting 事件需要 ID 冲突；建议
  创建逻辑会话时分配，重连另有 generation，审批绑定 session_id + generation + 参数。
- 方案强调消费者独立，但多个 bounded channel 的串行 await 仍可能相互阻塞；实现前明确
  Recorder 满队列/磁盘配额失败的可见处理，以及 UI 可丢显示数据的策略。
- Sprint 1 的“原有 SSH 回归”与独立 SSH Transport 是两项验收，不能以代理 SSH 通过替代后者。

## 6. 最小实施切片及验收顺序

1. **固定当前可构建候选与 Fork CI**：记录现有工作树构建结果，建立 Windows GUI check；
   完成无账号的新配置启动、PTY 回显、退出释放。先解决基线失败，再新增功能。
2. **独立本地启动边界**：逐调用点拆除必要性不成立的云初始化/请求，保留本地持久化和
   PTY；独立 AppId/配置目录必须有旧数据处理策略。验收基础终端无隐式外联。
3. **统一 Session 最小接口 + Serial**：只增加当前需要的领域类型、取消和收发通道，
   先用回环验证断开/关闭/部分写，再接一个 Serial 标签；文本/HEX 与原始日志先闭环。
4. **独立 SSH 与多会话恢复**：Host Key 校验、取消、resize、会话归属和恢复策略各自验证。
5. **运行时 Skill 管理**：导入预览/校验/原子安装/禁用默认值；覆盖 ZIP Slip、链接、
   大小限制与引用越界。无需 Agent 即可使用管理界面。
6. **可选 Agent**：模型配置与只读日志分析先行；所有写入经权限服务，不提前开放脚本执行。

按 KISS/YAGNI 逐切片新增，不预建整套空 crate；按 DRY 复用终端和持久化；以单一职责保持
Transport、数据管线、权限与 UI 的边界。每轮依次完成定向测试、lint、最终格式化。

## 7. 本轮验证状态

- metadata、Windows GUI 深度 2 dependency tree：通过，锁文件未更新。
- 当前工作树 `cargo check -p warp --bin warp-oss --features gui --offline --locked`：通过，
  exit 0，3m06s，无编译诊断。第一次尝试等待已有 `cargo run` 的构建锁后被本轮取消；
  上述结果来自随后完整运行的第二次尝试，没有中断已有应用。check 不等于链接或运行验收。
- 原版及历史运行记录：已阅读，未重新验证；本轮没有启动第二个 GUI 或操作已有窗口。
- 全新配置、真实断网/流量采集、SSH、实体串口、跨平台、第三方许可扫描：未执行。
- Sprint 0 尚不满足完整验收：官方上游来源锁定、完整网络入口审计及可运行 Fork CI 仍待完成。

本轮仅新增这份审计和两份依赖证据，未改动产品源码。后续应从第 6 节首个切片继续，
不能将本报告当作完成全部 Sprint 的声明。
