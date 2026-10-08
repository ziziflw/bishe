# V0.1 实际验证记录

## 环境

2026-10-08：Windows x64，内核 10.0.26200；Node.js 22.14.0；pnpm 10.12.4；Rust/Cargo 1.95.0 MSVC；Visual Studio 18 Build Tools（包含 C++ 工具）；WebView2 153.0.4234.48。

初始工作目录无源码、无 Git 仓库。用户要求后，工程迁移到 D:\PersonalFileTags，后续依赖缓存、构建与临时文件均使用 D 盘。

## 已通过

- `pnpm build`：TypeScript 严格检查 + Vite 生产打包成功。
- `cargo ... test --locked --manifest-path src-tauri/Cargo.toml`：12 passed / 0 failed / 0 ignored。
- `cargo fmt --manifest-path src-tauri/Cargo.toml`：已格式化。

测试对应关系：

| 要求 | 自动化证据 |
|---|---|
| 元数据登记、重复导入 | 名称、扩展名、绝对路径、大小、时间、状态断言；重复返回同一 ID、总数为 1 |
| 标签创建/修改/移除/多个标签 | 增删关联、重命名、重复关联幂等、两个标签共存 |
| 重启持久化 | 关闭 SQLite 连接后重新打开同一数据库，验证索引及标签 |
| 文件移走与重新关联 | 在临时目录真实移动文件，MISSING 保留标签，身份匹配后恢复 READY |
| 撤销授权 | 有文件和文件缺失两种情形均返回 UNAUTHORIZED；明确重选后恢复 |
| 目录越权和跳转 | 父目录跳转、前缀相似的相邻目录、实际 Windows junction、被替换的授权目录均拒绝 |
| 同名文件替换 | 状态 CHANGED，重复登记返回冲突，不继承原标签 |
| 数据库写入失败 | SQLite RAISE(ABORT) 触发器与 query_only 模拟失败，事务回滚、原索引/标签保留 |
| 原文件不变 | SHA-256、长度、创建/修改时间与源目录清单前后严格相等 |
| 文件占用与竞争 | 独占写句柄报错；读取句柄存活期间禁止重命名及写入打开 |
| 输入与数据库版本 | 空标签、重名、无效 ID、外键错误、SQL 字符串输入、高版本 schema 拒绝 |

全部测试使用 tempfile 创建的专用目录与合成文本。未对个人文件进行破坏性测试。

## 环境故障与修复

- npm 官方源在本环境超时：使用 `pnpm install --frozen-lockfile --registry=https://registry.npmmirror.com` 成功安装，锁文件保留完整性哈希。
- Cargo Schannel 报 SEC_E_NO_CREDENTIALS：仅构建期间通过 `.work/cargo-transport.mjs` 提供本机临时传输，由 Node 校验 HTTPS 后获取 crates.io 公共依赖；Cargo 仍验证锁文件 checksum。发布应用不包含、不启动这个服务。
- Vite 默认配置打包器读取受限祖先目录：改用 `--configLoader runner`。
- 初始旧 Tauri 主包与新运行时不兼容：统一 API/CLI/主包至 2.12.1，tauri-build 2.7.1，锁定 Cargo.lock。
- 初次 C 盘测试因沙箱禁止祖先元数据读取而失败：没有跳过安全检查；迁移到 D 盘并用 D 盘临时目录后全部通过。
- `pnpm exec` 在此会话未能定位本地 CLI；使用 `node node_modules/@tauri-apps/cli/tauri.js` 显式运行。

构建环境命令前缀：

```powershell
$env:CARGO_HOME='D:\PersonalFileTags\.cache\cargo'
$env:CARGO_HTTP_PROXY=''
$env:NO_PROXY='127.0.0.1,localhost'
$env:TEMP='D:\PersonalFileTags\.work\tmp'
$env:TMP=$env:TEMP
cargo --config .work/cargo-transport.toml test --locked --manifest-path src-tauri/Cargo.toml
node node_modules/@tauri-apps/cli/tauri.js build --no-bundle -- --config D:\PersonalFileTags\.work\cargo-transport.toml --locked
```

## 发布版及桌面验证

- Tauri release 构建成功，生成 `src-tauri/target/release/personal-file-tags.exe`，本地便携副本位于 `bin/personal-file-tags.exe`。
- `cargo clippy --config D:\PersonalFileTags\.work\cargo-transport.toml --locked --offline --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` 通过，无警告。
- 通过 Windows Computer Use 启动真实发布版，确认窗口、空状态、主窗口 IPC 和原生多选文件对话框可用。
- 在原生对话框选择 D 盘两个合成测试文件，实际登记成功，界面显示 2 条文件记录、正确路径/大小/扩展名及 2 项授权。
- GUI 标签编辑、GUI 授权撤销和整个桌面进程关闭后重启的人工端到端验证尚未完成；相应 Repository 行为及数据库重开持久化已通过上述自动化测试。未将未执行的 GUI 步骤记为通过。
- 数据库、个人数据、合成测试文件、依赖缓存和可执行构建产物均不纳入 Git 提交。

`.work/cargo-transport.*` 是本机构建环境的临时故障处理文件，不属于应用源码或运行依赖，未提交。普通 Windows 开发环境按 README 的标准命令构建。
