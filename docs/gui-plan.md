# 桌面应用计划与可行性

## 目标与边界

通过 Apple ID 登录（含双重认证），选择 IPA 和设备，预览账户变更后签名、安装并可导出 IPA。先验证 macOS，后续支持 Windows 和 Linux；免费和付费账户均为目标。首版只处理单主 App，对扩展和不支持的 entitlement 明确报错。

密码与验证码不持久化；会话仅在用户选择后保存在系统安全存储。优先本机认证数据，使用第三方服务前展示地址与涉及的数据并征得选择。不得自动撤销证书；Bundle ID 改动需用户确认。

## 初步调查（2026-10-31）

- zsign 已支持 P12、描述文件、IPA 签名和调用 `ideviceinstaller`，但不实现账户认证、设备发现或签名材料申请。
- [apple-private-apis](https://github.com/SideStore/apple-private-apis) 提供 Rust 认证、Anisette 和开发者接口模块，项目自述仍处于早期阶段；许可证为 MPL-2.0。其本机 macOS 支持及双重认证实际可用性尚需验证，集成时需检查依赖与许可证义务。
- [SideSign](https://github.com/SideStore/SideSign) 是另一候选，声明采用 GPL-3.0，不能在未评估发行和许可证影响的情况下直接引入。
- Tauri 是候选桌面框架；[官方环境要求](https://tauri.app/start/prerequisites/)和各平台打包条件需逐一验证。本机有 Rust、Node 和 OpenSSL。默认开发目录指向 Command Line Tools，但通过 `DEVELOPER_DIR=/Applications/Xcode-27.1.0.app/Contents/Developer` 可使用 Xcode 27.1；`idevice_id` 和 `ideviceinstaller` 未发现。
- 在候选库提交 `03beb1aa42991ccdad6214dee77e72282bef461f` 中，源码包含 macOS 本地 Anisette、受信任设备及短信验证码路径；这只是静态代码证据，未验证服务端实际行为。其默认提供者可能在本地失败时静默回退远程，远程实现还包含关闭 TLS 证书校验的代码；集成时必须显式选择提供者并保持证书校验，不能直接沿用这些默认行为。
- 实际 `cargo check -p icloud_auth -p apple-dev-apis` 在 macOS 因 `omnisette` 的 AOSKit trait 签名不匹配失败；`apple-dev-apis` 的 `XcodeSession::with` 仍为 `todo!()`，因此该版本不能直接满足认证及开发者接口需求。
- [Impactor](https://github.com/claration/impactor) 的 `plume_core` 提供认证和开发者接口，包声明 MPL-2.0；其 Anisette 默认回退远程且远程实现关闭 TLS 校验，必须修改并验证后才可接入。源码静态检查不能证明 Apple 服务当前可用。本机运行 macOS 27.2；本机 Anisette 可用性尚待实测。
- 非公开 Apple 接口、免费账户权限、本机 Anisette 和真实设备安装均未验证。不能将登录或签名功能视为已可用。

## 实施顺序

1. 验证候选认证库、设备依赖与 macOS 构建环境，记录结果。
2. 建立桌面基础，围绕已确认的公开行为逐个采用红—绿循环：认证状态、签名输入和结果、设备与安装状态、界面呈现。
3. 接入认证与双重认证，建立会话安全存储。
4. 接入设备发现、账户资源预览与签名材料准备。
5. 接入 zsign、安装和导出，验证错误处理。
6. 由用户在应用中输入凭据和验证码，在 macOS 真实账户和设备上验收。
7. 适配 Windows 和 Linux，并分别验证打包与设备流程。

每个完成的步骤运行适当检查并单独提交。真实 Apple 服务和设备的结果不能由测试替身代替。
