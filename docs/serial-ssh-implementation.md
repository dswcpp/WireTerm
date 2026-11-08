# 串口与普通 SSH 会话实现

本次实现对应《WireTerm 系统详细设计方案》的串口核心及普通远程 PTY 部分。首发验证平台为
Windows x64。GUI 复用工作区标签页、分屏和现有终端渲染；连接参数通过标签页内的交互向导设置。

## 使用入口

- 新建会话菜单：`Serial Connection`、`SSH Connection`。
- 命令面板：`New Serial Connection`、`New SSH Connection`。
- 向导先配置、可选保存，再明确确认连接；新开串口不发送初始化字符串或重放历史输入。
- 已保存配置会在向导开头列出，可按名称重新打开。基础连接不要求配置 AI 或登录 Warp 云服务。

开发构建还支持以下 PowerShell 调用：

```powershell
& .\target\debug\warp-oss.exe --wireterm ports
& .\target\debug\warp-oss.exe --wireterm wizard serial
& .\target\debug\warp-oss.exe --wireterm wizard ssh
& .\target\debug\warp-oss.exe --wireterm profiles
& .\target\debug\warp-oss.exe --wireterm open board-a
& .\target\debug\warp-oss.exe --wireterm delete board-a
```

独立测试连接器可以通过 `cargo build -p wireterm_connections --bin wireterm-connect` 构建，参数与
`warp-oss --wireterm` 相同。全局 `--config-dir <目录>` 支持便携配置和隔离测试。

## 串口

支持端口枚举、USB VID/PID/序列号、波特率、5–8 数据位、奇偶校验、1/2 停止位、软硬件流控、
UTF-8/GBK 等编码、增量解码、TEXT/HEX/ANSI 显示、TEXT/HEX 发送、可选行结束符和 UTC 行时间戳。
HEX 输入必须为完整字节，HEX 发送不追加行结束符。不能编码的文本会被拒绝，不静默替换。

```powershell
& .\target\debug\warp-oss.exe --wireterm serial --port COM3 --baud 115200 --save board-a
& .\target\debug\warp-oss.exe --wireterm serial --port COM3 --display hex --hex-send --record D:\captures\board.jsonl
& .\target\debug\warp-oss.exe --wireterm serial --port COM3 --encoding gbk --line-ending crlf
```

`--dtr true/false`、`--rts true/false` 只在用户明确指定时设置开口状态；默认保留 DTR、不主动设置 RTS。
操作系统和设备驱动仍可能在打开/关闭串口时改变控制线，需要用真实适配器确认是否引起板卡复位。
硬件流控与手动 RTS 互斥。

控制台操作：

| 操作 | 行为 |
|---|---|
| 输入文字，Enter | 按当前发送模式提交一条输入 |
| ↑ / ↓ | 浏览当前会话最近 128 条发送输入；不会自动重发 |
| Ctrl+Q | 关闭并释放端口 |
| Ctrl+C | TEXT/HEX 模式取消输入；ANSI 模式发送中断字节 |
| Ctrl+X | 切换 TEXT/HEX 发送 |
| Ctrl+V | 切换 TEXT/HEX 显示；也可在连接时指定 `--display` |
| Ctrl+T | 切换行时间戳 |
| Ctrl+] | 进入本地控制输入；Escape 取消，Enter 确认 |

本地控制输入支持 `dtr on/off`、`rts on/off`、`break`、`stats`、`templates` 和 `template 名称`。
BREAK 持续 100 ms；期间仍轮询接收。控制操作和状态变化作为 `Control` 记录写入可选日志。
TEXT 显示会转义设备控制字符，避免把原始 MCU 输出当作终端控制指令执行；只有显式选择 ANSI
模式时才保留控制序列交给终端解析。ANSI 与 TEXT 都使用增量字符解码，避免 UTF-8/GBK 字符在跨包时损坏。
ANSI 模式即时转发键盘，保留上表中的本地快捷键，不额外输出每键 TX 提示；切到 HEX 发送时仍需 Enter 提交。

发送模板只保存数据，不是可执行脚本：

```powershell
& .\target\debug\warp-oss.exe --wireterm template board-a reset 'AT+RST'
& .\target\debug\warp-oss.exe --wireterm template board-a probe '01 03 00 00 00 01' --hex
```

连接中按 Ctrl+]，输入 `template reset` 并按 Enter 才发送。模板不会在连接、重连、保存配置时执行。
模板最多 128 条，单条最多 8 KiB，整个配置 JSON 最多 64 KiB；更新超限会保留原文件。

串口写队列有界，单次发送最多 64 KiB。发送阻塞时继续轮询接收与取消；5 秒写超时会报错，
已经写出的部分按实际字节数记录。断线后丢弃未发送部分及排队命令，不自动重放。

自动重连默认关闭。`--reconnect` 只允许具有稳定 USB 序列号的设备：按 VID/PID/序列号重新匹配，
允许同一设备的 COM 号变化；缺失、身份不符或重复身份不会选取其他端口。失败等待 1、2、4、8 秒，
上限 30 秒。手动关闭会取消等待并释放句柄。

没有序列号或序列号重复时，可禁用自动重连并按明确的端口手动连接；这类配置不保存用于跨端口跟随的指纹。
波特率参数范围目前为 1–4,000,000，实际可用速率由适配器和驱动决定。

## 原始日志及背压

`--record` 创建新的 JSONL 文件，已有文件不会被覆盖。每行包含会话 UUID、毫秒时间戳、方向和原始
字节数组及各方向的字节偏移。`Received` 是已读取字节，`Sent` 是成功写入串口驱动的字节（不代表设备
协议应答），`Control` 是连接状态和控制操作。日志失败不抹掉已实际写入的统计；关闭时也会显示最终收发计数。
HEX 显示使用原始接收偏移；显示丢批后按偏移重置字符解码，不把断开的字符片段拼成新字符。

每段最多 64 MiB，后续分段命名为 `原文件名.part-0001` 等，总日志配额 256 MiB。日志按接收/发送顺序
写入后才进入显示队列；显示过载可丢弃显示批次并报告跳过字节数，不能静默丢日志。分段冲突、磁盘写入
失败或总配额耗尽会明确停止连接，保留已有数据，不覆盖旧段，也不声称日志仍完整。

## 普通 SSH

SSH 使用系统 OpenSSH，而非 Warp 的远端代理安装链路。在现有本地 PTY 中启动 SSH，继承原生输入、
ANSI/VT 渲染、窗口缩放和密码/私钥口令交互。Windows 使用系统目录下的 OpenSSH，避免从当前项目目录
误启动同名程序；未安装时显示安装提示。

```powershell
& .\target\debug\warp-oss.exe --wireterm ssh 192.168.1.20 -l developer -p 22 --save lab
& .\target\debug\warp-oss.exe --wireterm ssh 192.168.1.20 -l developer -i C:\Keys\lab_key
```

首次未受信主机必须确认 SHA256 指纹；已保存主机密钥变更会阻断认证。WireTerm 使用独立的
`known_hosts`，强制 `StrictHostKeyChecking=ask`，不自动接受新密钥或更新变更密钥。首次指纹确认在
终端内完成。指定私钥时启用 IdentitiesOnly，避免自动尝试无关的 agent 身份；未指定时可使用 OpenSSH 的默认密钥、agent 或密码认证。

连接配置只保存 host、user、port 和私钥路径，不接收密码/口令字段，不保存密码，不把凭据放进进程参数。
连接超时 15 秒，keepalive 间隔 15 秒、连续失败 3 次后断开。远端正常退出码原样返回；SSH 的认证、
主机密钥和网络错误由原生诊断显示。退出或断线后由用户显式重连，不自动重复认证或重新执行远端命令。

## 模块边界

- `crates/wireterm_connections`：配置、编码、串口 Tokio IO、原始录制、OpenSSH 参数与交互入口。
- `app/src/lib.rs`：`--wireterm` 在 GUI/网络初始化前分派。
- `app/src/tab.rs`、`workspace`：连接菜单、命令面板和标签页启动。
- 串口 IO 运行于连接进程的 Tokio 任务中，GUI 仅处理既有 PTY 输出，不同步读写设备。
- 传输模块不引用 UI、Agent、Skill 或 Warp 云 API。Agent 没有获得绕过审批的设备写入口。

这一版采用“现有终端 + 连接向导”，没有另造终端渲染器。设计中的独立设备树和原生 SerialToolbar
仍属于后续界面扩展，不影响当前连接、参数配置和收发入口。

## 验证与剩余验收

```powershell
cargo nextest run -p wireterm_connections
cargo clippy -p wireterm_connections --all-targets -- -D warnings
cargo check -p warp --lib --features gui
cargo build -p warp --bin warp-oss --features gui
```

`script/verify-wireterm-ssh.py` 使用 loopback SSH fixture 和真实系统 OpenSSH，覆盖首次指纹接受/拒绝、
主机密钥变更阻断、密钥/密码认证、认证失败、远端 PTY、缩放、交互收发、Ctrl+C 和退出码。
它只创建临时测试密钥和 known-host 文件；Windows 会将临时私钥 ACL 限制为当前用户。
运行需要 `paramiko`，Windows 另需 `pywinpty`，其他平台另需 `ptyprocess`。

```powershell
python script/verify-wireterm-ssh.py --binary target/debug/wireterm-connect.exe --output target/connection-verification/ssh-results.json
```

Unix 另有真实 pseudo-terminal 串口测试，覆盖不隐式发送、收发和关闭后重新打开。Windows 本机未枚举到
物理或虚拟 COM 端口，因此尚未完成 MCU 实机回环、设备拔插、控制线电平和持续吞吐验收；不能把编码、
队列或模拟流测试当作这些验收已通过。Linux/macOS 构建与运行仍需相应平台验证。
