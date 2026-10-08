# Personal File Tags V0.1

Windows 10/11 x64 本地文件登记与人工标签工具。不会复制、移动、修改或删除原文件，不扫描硬盘、不自动监控目录、不调用云端 AI。

## 启动

需要 Windows 10/11 x64 和 Microsoft Edge WebView2 Runtime。开发构建需要 Node.js 22.14+、pnpm 10.12.4、Rust 1.95（MSVC x64 工具链）、Visual Studio C++ Build Tools 和 Windows SDK。

在 `D:\PersonalFileTags` 运行；先设置本次 PowerShell 会话的缓存和临时目录，保证后续构建写入 D 盘：

```powershell
$env:CARGO_HOME="D:\PersonalFileTags\.cache\cargo"
$env:TEMP="D:\PersonalFileTags\.work\tmp"
$env:TMP=$env:TEMP
New-Item -ItemType Directory -Force $env:CARGO_HOME,$env:TEMP | Out-Null
```

```powershell
pnpm install --frozen-lockfile
pnpm desktop
```

生产构建（生成独立 EXE，未配置安装包或签名）：

```powershell
pnpm package
# src-tauri/target/release/personal-file-tags.exe
```

验证：

```powershell
pnpm check
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

依赖锁定于 `pnpm-lock.yaml` 和 `src-tauri/Cargo.lock`。Vite 使用 `--configLoader runner`，兼容当前受限 Windows 构建环境。pnpm 若提示忽略 esbuild 安装脚本，当前平台预编译包仍可构建；不要开启无关依赖脚本。

## 使用方式

1. 点击「登记文件」，在 Windows 原生对话框选择一个或多个文件。选择同时授权后续查看详情时读取元数据，最多每批 500 项。
2. 左侧创建标签，选择文件后勾选多个标签；取消勾选移除关联。标签旁的编辑按钮可重命名。
3. 输入文件名后搜索，可叠加标签、扩展名筛选。搜索只查询本地数据库，不读取原文件。
4. 查看详情或点击检查状态时才访问原位置。`MISSING` 保留标签；`CHANGED` 表示路径被其他身份文件占用；`UNAUTHORIZED` 表示没有有效授权。
5. 原文件同卷移动后，点击「重新选择原文件关联」。卷标识、文件索引及创建时间必须一致；副本及跨卷移动保守拒绝。
6. 「设置与授权」可以选择目录或撤销文件/目录授权。本版不遍历目录。重叠授权独立存在，要禁止某文件后续访问，需要撤销覆盖它的全部授权。
7. 「移除此索引」只删除数据库中的索引与关联；不会删除原文件，不会自动撤销授权。撤销授权也不会自动清空索引。

## 数据与文件结构

应用数据库：EXE 旁的 `PersonalFileTags-data\personal-file-tags.sqlite3`。交付版本位于 `D:\PersonalFileTags\bin`，因此应用数据也留在 D 盘。SQLite WAL/SHM 及 WebView2 自身运行数据位于应用数据范围内。程序没有文件正文缓存或扫描日志。备份应先完全退出应用，再备份该应用数据目录。

```text
src/
  api.ts                 类型与固定 IPC API
  App.vue                界面与交互状态
  style.css              本地桌面样式
src-tauri/
  src/access.rs          Windows 路径/句柄/身份/授权检查
  src/repository.rs      SQLite Repository、事务与业务数据操作
  src/commands.rs        校验后的 Tauri 命令、Rust 原生对话框
  src/model.rs           共享数据结构
  src/error.rs           脱敏错误类型
  src/tests.rs           临时目录集成测试
  src/lib.rs             应用初始化与命令注册
  migrations/001_init.sql 版本化数据库结构
  capabilities/main.json 主窗口业务命令白名单
  build.rs               资源与权限生成
  icons/                 本地程序图标
  Cargo.lock             Rust 依赖锁
  tauri.conf.json         窗口与 CSP 配置
docs/
  architecture.md
  security.md
  verification.md        实际执行与验证记录
```

## 已知边界

- Windows 本地盘符路径，最长 240 个 UTF-8 字节；拒绝 UNC、设备路径、ADS、`..`、符号链接、junction 和其他 reparse point（含某些云文件占位符）。
- 应用只读取必要元数据，不解析格式或正文。无扩展名、未知格式也可登记。独占占用或无法安全读取时返回错误。
- 本版不自动追踪移动，文件身份不是密码学永久标识；不支持跨卷自动关联。
- 数据库未加密，Tauri capability 不等于操作系统沙箱；Windows 可自行更新访问时间。详见 `docs/security.md`。
- 文件名搜索使用 SQLite 的大小写处理：英文不区分大小写，中文按原文匹配；尚未实现国际化全文分词、分页或大规模索引优化。
- 本版不含资源管理器定位、安装程序、自动更新、AI、OCR、向量检索、Python 服务和桌宠。

Tauri Windows 构建前置条件：https://v2.tauri.app/start/prerequisites/


如需让当前 PowerShell 会话自动使用项目内缓存和临时目录，可运行：

```powershell
. .\scripts\Enter-ProjectEnvironment.ps1
```

便携 EXE 使用其旁边的独立数据目录；请将 `bin` 文件夹整体保留在 D 盘。开发模式和发布版本的 EXE 位于不同目录，因此默认拥有各自的数据目录。
