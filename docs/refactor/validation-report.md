# P0 首批验证报告

## 验证对象与判定

分支 `refactor/p0-local-terminal`，原始 SHA `f7fc2f2a08a3aa446baa23960710e630c5defd6e`。
本报告覆盖遥测发送链和独立 TUI 分发的裁剪，**完整 P0 验收尚未通过**。
剩余功能和风险边界见 [deletion-report.md](deletion-report.md)。
验证源码提交：`a1dacae0bd26ce34f22a062e622cadd010820f11`。规模对比及提交记录见该删除报告。

## 基线

- 平台：Windows x86_64 / MSVC 2022；Rust / Cargo 1.92.0。
- 原版命令：`cargo build -p warp --bin warp-oss --features gui`，成功。
- 原版启动成功，观察到 “Welcome to Warp” 欢迎界面；不能将欢迎页等同于完整终端离线验收。
- 原版 debug exe：405,002,240 字节。81 个 workspace crate，1,574 个 lock 包，270 个 workspace 依赖声明，
  4,161 个 Rust 文件，1,808,237 行 Rust 源码。
- 首次环境缺失：Rust 工具链和 protoc；已安装 Rust 1.92.0、protoc 25.1 后构建成功。
  这些是基线环境问题，不是裁剪回归。

## 已完成检查

| 检查 | 结果 |
|---|---|
| Windows GUI `cargo check -p warp --bin warp-oss --features gui --offline` | 已通过一次；随后清理未使用项并进行 Clippy 验证 |
| `cargo nextest run -p http_client --lib` | 8/8 通过，0 skipped；run ID `a86076f3-f95d-4acd-8aba-4e3f090799ea` |
| `cargo nextest run -p secret_redaction --lib --offline` | 27/27 通过，含迁移的 14 项脱敏行为测试；run ID `85b12cd1-e1d1-43bc-8aeb-ef1416e42063` |
| HTTP 请求回归 | 保留外部签名请求行为，确认没有自动添加 `x-warp-traceparent` / `traceparent` |
| HTTP client Clippy | `cargo clippy -p http_client --all-targets --tests --offline -- -D warnings` 通过 |
| 脱敏 crate Clippy | `cargo clippy -p secret_redaction --all-targets --tests --offline -- -D warnings` 通过 |
| GUI Clippy | `cargo clippy -p warp --bin warp-oss --features gui --offline -- -D warnings` 通过 |
| 最终 Windows GUI build | `cargo build -p warp --bin warp-oss --features gui --offline` 成功，dev profile，23m 50s |
| 格式化 | 所有变更 Rust 文件执行一次 rustfmt，edition 2024，使用仓库 import 分组策略；随后 `git diff --check` 通过，按 AGENTS 要求未在格式化后重复编译检查 |
| GUI 启动 | 新 exe 启动成功，窗口可响应，恢复本机已有终端会话；没有验证全新配置首次启动 |
| 终端输入输出 | PowerShell 执行 `echo WIRETERM_P0_IO_OK`，两个 Pane 均观察到命令及 `WIRETERM_P0_IO_OK` 输出 |
| 多标签 / 分屏 | Ctrl+Shift+T 创建新会话，Ctrl+Shift+D 创建右侧分屏，观察到多个会话及三列 Pane；进行了窗口恢复/激活 |
| 本地 run 入口 | shim Cargo 探针确认 `--profile dev -- --help` 参数完整转发，渠道为 OSS，默认 feature 为 gui；未执行外部配置/技能安装 |
| workspace metadata | 80 个 crate；不再包含独立 `warp_tui` |
| Bash syntax | Linux / macOS bundle 的 `bash -n` 通过 |
| PowerShell syntax | Windows bundle 经 PowerShell AST 解析通过 |
| Release workflow | YAML 可解析；19 个任务，没有悬空 `needs` 引用 |
| Lock 包差异 | 删除 7 个包，新增/升级 0 个 |

测试首次 `--offline` 因 nextest 的全 feature metadata 缓存缺失而无法开始；联网下载测试依赖后通过。
这不代表测试时产品必须联网。源码检查不等于“无网络调用”，仍有明确保留的 Warp 网络服务。

## 运行证据与限制

实际运行使用现有本机用户数据，恢复了此前的 `ls` 结果；没有清空数据库、修改用户账号或上传内容。
初次自动键入受到中文 IME 组合输入及 PowerShell 初始化延迟影响，不能计作产品故障或成功回显；
改用粘贴后观察到上述精确哨兵输出。新 Shell 曾显示启动较慢提示，没有原版同条件耗时数据，
因此没有认定性能通过，也没有将该提示归因于裁剪。

窗口级截图和原始构建日志存于本地 `.git/wireterm-p0/`，没有将包含用户目录的截图纳入提交。
最终可执行文件为 `target/debug/warp-oss.exe`，402,686,464 字节；验证后程序保持运行。

## 未验证 / 未达标

- 本地账号替代和完整登录删除尚未实施；没有宣称无需 Warp 服务运行的最终目标达成。
- 没有验证物理断网后的完整终端流程。当前进程没有管理员 token，不能添加应用级 Windows 防火墙规则；
  未修改整机网络配置。
- 多标签、分屏、基本输入输出和已有会话恢复仅做了上述 smoke 验证；没有完整的前后对照、
  所有窗口操作覆盖、ANSI/VT conformance 或终端性能基准。
- Linux/macOS 未编译或运行。跨平台 skill 所需的 `oz-dev` 不可用，没有虚构云 runner 或平台结论。
- 没有运行全量 presubmit、全 workspace 测试、发布签名、安装器实装或性能基准。
