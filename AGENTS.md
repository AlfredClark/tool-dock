# AGENTS.md — 项目开发规范

> 本文档面向 AI 智能体与新加入开发者，请在开始任何代码修改前通读本文件。
> 基于 Tauri v2 + SvelteKit 2 + Svelte 5 + Tailwind CSS v4 + shadcn-svelte + Paraglide + tauri-specta 的桌面工具箱应用。
> 优先级：安全红线 > 架构规范 > 测试验证 > 代码风格。

---

## 1. 项目概述

- **项目名称：** `tool-dock`（显示名 `ToolDock`）
- **一句话描述：** A cross-platform desktop toolbox.
- **核心功能：**
  1. Tauri 类型安全命令链（tauri-specta 契约 + 前端链式 API）
  2. 明暗主题跟随（`mode-watcher` system 跟随 + Tailwind v4 `.dark` 变体）与配色主题（鲜艳系 ocean/teal/emerald/amber/tangerine/rose/amethyst + 暗色美学 nord/dracula/tokyo-night，`data-color-theme` 属性切换〈刻意避开 mode-watcher 占用的 `data-theme`〉，见 `themes.css`）
  3. 前后端双轨国际化（前端 Paraglide + 后端 rust-i18n，持久化于后端 `config.json`）
  4. 前后端崩溃兜底（前端 `ErrorBoundary` + 后端 panic 钩子写 `%TEMP%/tool-dock_crash.log`）
  5. 完整设置页（通用分组经 `config.json` 落盘 + 外观分组纯前端）
  6. 关于页（应用 / 项目 / 平台分组 + 更新面板）与桌面外壳（无边框窗口 / 托盘 / 单实例 + `tool-dock://` 深链 / 更新器）
  7. 首页工具搜索 + 工具列表页（按分类网格渲染）+ 独立工具路由（`(tools)` 分组自绘标题栏，三者同源于工具注册表 `libs/tools/registry.ts`）
- **用户角色：** Tauri + Svelte 技术栈的桌面应用开发者

---

## 2. 技术总览

> 版本唯一来源是 `package.json`（前端）与 `src-tauri/Cargo.toml`（后端），下表不记录版本号。
> 生成代码时必须使用这两个文件中实际声明版本对应的 API，禁止自行升级 / 降级依赖。

| 层级        | 技术选型                                                                                                                                                               | 说明 / 用途                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 前端框架    | SvelteKit + Svelte                                                                                                                                                     | SPA 模式（`adapter-static` + `fallback: index.html`，`ssr = false`）；`@sveltejs/vite-plugin-svelte` 构建集成                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| UI / 样式   | shadcn-svelte + Tailwind CSS                                                                                                                                           | shadcn-svelte（nova / neutral），Tailwind（`@tailwindcss/vite` + `tailwind-merge` / `tailwind-variants` / `tw-animate-css`）；图标 `@lucide/svelte`，字体 `@fontsource-variable/geist`，主题 `mode-watcher` + `layout.css` 的 `.dark` 变体，配色 `themes.css` 的 `[data-color-theme]` 十色块（`neutral` 为默认回落，暗色须用 `:root.dark[data-color-theme]` 显式组合），`cn()` 合并类名                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 前端插件    | `@tauri-apps/api` + `plugin-log` / `opener` / `updater` / `system-fonts`                                                                                               | Tauri JS 互操作与桌面能力；日志上报走 `plugin-log`，系统字体列表走 `tauri-plugin-system-fonts-api`，CLI 为 `@tauri-apps/cli`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 前后端契约  | tauri-specta + `specta` / `specta-typescript`                                                                                                                          | Rust 侧 `#[tauri::command]` + `#[specta::specta]` 导出类型，生成物 `src/libs/commands/bindings.ts`，禁止手改                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 国际化      | Paraglide（前端）+ rust-i18n（后端）                                                                                                                                   | 前端 `paraglide-js` / `paraglide-js-svelte`（策略 `localStorage` + `baseLocale`，CLI 为 `@inlang/cli`）；后端 `rust-i18n`（`src-tauri/locales/*.yml`，`fallback = "en"`）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 后端框架    | Tauri + Rust                                                                                                                                                           | `edition = "2024"`，工具链由 `rust-toolchain.toml` 锁定；`tauri-build` 构建脚本                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 后端插件    | `opener` / `store` / `log` / `os` / `updater` / `autostart` / `window-state` / `system-fonts` / `single-instance` / `deep-link` / `clipboard-manager` / `http` / `sql` | 分别对应外部打开（capability 仅 `opener:allow-open-url` 且 `allow` 域只放行 `http(s)`，与前端 `openExternal` 网关双层收敛；后端 `open_path` 走 Rust 侧不受约束）、持久化（`config.json` 存 `locale`）、日志（capability 仅 `log:allow-log`）、系统语言探测（已注册 `plugins/os.rs`，纯 Rust 侧调用故 capability 无 `os` 项）、自动更新 / 开机自启 / 窗口状态（后三者仅非移动端启用 `updater` / `autostart` / `window-state`；更新器前端经命令链调用、无插件 JS 直调，故 capability 无 `updater` 项）、系统字体探测（`plugins/system_fonts.rs`，capability 仅 `system-fonts:allow-get-system-fonts`）、单实例（二次启动聚焦 + 深链参数转发，仅桌面端）、深链协议（`plugins.deep-link.desktop.schemes`，capability 仅 `deep-link:allow-get-current`，`onOpenUrl` 走核心事件通道）、剪贴板纯文本写（`copy_system_info` 经 Rust 侧 `write_text`，前端无 JS 直调故 capability 无 `clipboard-manager` 项）、网络请求（前端 `fetch` 出网，capability 域约束 + CSP `connect-src` 双门禁，仅桌面端）、结构化存储（SQLite，`sqlite` 特性 + `Builder` 注册，前端无 JS 直调故 capability 暂无 `sql` 项，仅桌面端） |
| 后端支撑    | `serde` / `serde_json` + `chrono` + `log` + `thiserror` / `anyhow`                                                                                                     | 序列化、UTC 时间、日志门面、统一错误（`CommandError` / `CommandResult`，见第 5 章第 4 条）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 构建 / 质量 | Vite + TypeScript + vitest + pnpm                                                                                                                                      | Vite 固定端口 `1420`；`svelte-check` 严格 TS 检查；ESLint + Prettier；vitest 单测；包管理器 `pnpm`（`engines.node >= 24`，`pnpm-lock.yaml` 锁定；后端 `Cargo.lock` 需提交）                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 部署        | GitHub Actions + Tauri bundler                                                                                                                                         | CI 入口 `.github/workflows/ci.yml`，发布入口 `.github/workflows/release.yml`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

**版本查询命令：**

```bash
node -v && pnpm -v && cat package.json | grep -A 30 '"dependencies"'
rustc --version && cat rust-toolchain.toml && cat src-tauri/Cargo.toml | grep -A 20 '\[dependencies\]'
```

---

## 3. 项目结构

> 目录职责边界：只在对应目录做对应事，禁止跨层乱放。

```text
tool-dock/
├── src/                            # 前端：SvelteKit 单页应用（adapter-static + fallback: index.html）
│   ├── app.html                    # HTML 外壳，SvelteKit 在此注入脚本与样式
│   ├── assets/                     # 需要经构建处理的资源（别名 $assets；内联单色图标组件统一放 icons/ 分组如 icons/github-icon.svelte / icons/app-icon.svelte，见 svelte.config.ts）
│   ├── routes/                     # 页面与全局样式
│   │   ├── +layout.svelte          # 根布局：ModeWatcher（主题）+ Toaster + ErrorBoundary（渲染异常）包裹全部分组，首帧前对齐外观变量（字体/字重/字号）
│   │   ├── +layout.ts              # ssr = false（SPA 模式）；语言对齐在 +layout.svelte 挂载后后台完成，不经 load()（Tauri invoke 走 window.fetch，load 内调用触发 SvelteKit 追踪警告）
│   │   ├── (main)/                 # 分组路由（括号不进 URL）：常规页面分组，独享布局容器
│   │   │   ├── +layout.svelte      # 分组布局：LayoutContainer 包裹，特殊页另起分组即可绕开布局
│   │   │   ├── +page.svelte        # 首页：应用图标 + 工具搜索（注册表过滤，选中跳转），下方留白供后续功能
│   │   │   ├── about/              # 导航标签示例页（应用 / 项目 / 平台分组 + 更新面板）
│   │   │   ├── settings/           # 完整设置页（通用 + 外观两分组）
│   │   │   └── tools/              # 工具列表页（按分类网格渲染，点击进独立工具路由）
│   │   ├── (tools)/                # 工具详情分组（括号不进 URL）：绕开主布局容器，自绘标题栏
│   │   │   ├── +layout.svelte      # 独立布局：左返回（`goto("/tools")`，不用 history.back）+ 中工具名（查注册表）+ 右窗口控制（复用 parts/window-buttons.svelte）
│   │   │   └── text/editor/        # 文本编辑工具路由（URL 即 /text/editor），内部组件在 components/tools/text/editor/
│   │   └── layout.css              # Tailwind v4 入口与 shadcn-svelte 主题令牌（含 .dark 暗色变体与 --app-font-* 外观变量）
│   │   └── themes.css              # 配色主题令牌（鲜艳系 7 色 + 暗色美学 3 色的浅色 + :root.dark 暗色块，neutral 回落 layout.css）
│   ├── components/
│   │   ├── common/                 # 手写共享组件（error-boundary.svelte 全局渲染兜底，card-section/row.svelte 通用卡片分组与行，confirm-dialog.svelte 通用确认弹窗）
│   │   ├── layout/                 # 布局子系统：layout-container.svelte 容器 + tabs（标题栏/标签栏/内容/底边）与 sidebar（侧边栏 + 标题栏/内容/底边）实现，新增布局需在 libs/hooks/appearance.svelte.ts 中映射
│   │   │   └── parts/              # 布局配套部件（title-bar / window-buttons / nav-tabs-bar / side-nav-bar / copyright）
│   │   ├── widget/                 # 手写页面小组件（settings/：设置页通用/外观分组与字体选择器；about/：关于页应用/项目/平台分组与更新面板；home/：首页工具搜索；tools/：工具网格与占位）
│   │   ├── tools/                    # 工具内部组件：按 `<分类>/<工具名>` 分目录，与工具路由一一对应（如 routes/(tools)/text/editor 的内部组件放 tools/text/editor/，路由页只做组装）
│   │   └── shadcn-svelte/          # CLI 生成的 UI 组件（nova / neutral / lucide），新增走 CLI 添加，勿手动重组
│   ├── tests/                      # 前端测试集中目录，按源路径镜像：unit/ 纯逻辑（node）+ component/ 组件（jsdom）
│   └── libs/
│       ├── commands/               # tauri-specta 契约链（bindings.ts 生成物 + 链式 API 封装）
│       ├── http/                   # 网络层：client.ts 通用请求封装，错误统一 HttpError
│       ├── i18n/                   # Paraglide：messages/ 文案、project.inlang/ 配置、paraglide/ 生成物
│       ├── navigation/             # 导航等应用级注册表（nav-tabs.ts：路径 / 图标 / 顺序，顶部标签栏与侧边栏导航同源）
│       ├── tools/                    # 工具注册表（registry.ts：路径 / 分类 / 文案 / 图标，首页搜索 + 工具网格 + 工具标题栏 + 深链四处同源）
│       ├── utils/                  # 前端通用工具（shadcn-svelte.ts 的 cn()、window-controls.ts、toast.ts、opener.ts）
│       └── hooks/                  # 前端共享状态（统一经 $hooks 引用），如 appearance.svelte.ts（布局注册表 + 字体偏好）、config.svelte.ts（后端配置内存态）、updater.svelte.ts（更新状态）、is-mobile.svelte.ts（响应式断点）、deep-link.svelte.ts（三路深链入口）
├── src-tauri/                      # 后端：Rust（edition 2024）
│   ├── src/
│   │   ├── main.rs                 # 二进制入口，仅转发到 lib::run()
│   │   ├── lib.rs                  # 串联插件注册 → 命令处理器 → 核心初始化
│   │   ├── commands/               # 命令封装层：薄封装 + collect_commands! 注册
│   │   ├── cores/                  # 通用能力与跨层共享类型（types / locale / config / deep_link / specta / system / tray / updater）
│   │   ├── features/               # 业务逻辑（纯函数，不依赖 Tauri 运行时）
│   │   └── plugins/                # 各 Tauri 插件的初始化（log / store / opener / os / updater / autostart / window-state / system-fonts / single-instance / deep-link / clipboard / http / sql）
│   ├── locales/                    # rust-i18n 后端文案（*.yml，单文件可内含多 locale）
│   ├── capabilities/               # 权限配置（default.json / plugins.json），外部 URL 需同步更新
│   ├── icons/                      # 应用图标（由 pnpm tauri:icon 生成）
│   ├── build.rs                    # tauri-build 构建脚本
│   ├── tauri.conf.json             # 窗口 / CSP / 打包目标 / updater 配置
│   └── Cargo.toml                  # 后端依赖与 crate 元信息
├── static/                         # 原样拷贝到产物根目录的资源（favicon / 图标 / svg）
├── .github/workflows/              # ci.yml（质量门禁）、release.yml（打包发布）
├── Cargo.toml                      # 虚拟工作区 + 全局 lints / profile
├── package.json                    # 脚本与前端依赖
├── svelte.config.ts                # adapter-static、路径别名（唯一来源）
├── vite.config.ts                  # Vite + SvelteKit + Tailwind + Paraglide 插件，固定端口 1420
├── vitest.config.ts                # 单元测试配置（复用 sveltekit() 插件以继承别名）
└── AGENTS.md                       # 本文件
```

**铁律：**

- 前端组件禁止裸 `invoke`，必须经 `src/libs/commands` 链式 API 调用后端。
- 后端 `commands/` 只做薄封装（参数与结果转换 + 注册），业务逻辑必须放入 `features/`，跨层共享能力放入 `cores/`。
- `libs/utils/` 只放不含业务 / 应用知识的通用工具（`cn()` / `toast` / `window-controls`）；注册表类（路径、图标、顺序等应用知识）放 `libs/<领域>/`，如导航注册表在 `libs/navigation/`。后端通用能力一律进 `cores/`，不另设 `utils/`。
- `bindings.ts` 为生成物，禁止手动编辑；`paraglide/` 生成物禁止编辑、禁止从外部导入其内部文件。
- 工具内部组件一律放 `src/components/tools/<分类>/<工具名>/`，与 `routes/(tools)/<分类>/<工具名>/` 路由一一对应，路由页只做组装不写业务；跨工具复用的展示组件才允许上浮到 `components/widget/tools/`。
- 路径别名（`$assets`、`$components`、`$hooks`、`$libs`）唯一来源是 `svelte.config.ts` 的 `kit.alias`，不写入 `vite.config.ts` 。

---

## 4. 常用命令

> 所有命令以仓库根目录为基准执行。优先使用以下命令，禁止另起等效命令。
> 包管理器为 `pnpm@12.9.1`，Node `>=24`；Rust 工具链锁定 `1.99.0`（见 `rust-toolchain.toml`）。

```bash
# ---- 环境初始化 ----
pnpm install --frozen-lockfile     # 按锁文件安装依赖（不要用 npm / yarn）

# ---- 本地启动 ----
pnpm tauri:dev                     # 完整桌面联调：Vite（固定端口 1420）+ Tauri 窗口
pnpm dev                           # 仅启动前端（浏览器中 Tauri 命令不可用，会自动降级）

# ---- 代码检查与格式化（每次修改后必跑 pnpm format + pnpm validate） ----
pnpm format                        # 自动修复两端：Prettier + ESLint + cargo fmt
pnpm lint                          # 前端 + 后端全量检查
pnpm lint:frontend                 # prettier --check . + eslint .
pnpm lint:backend                  # cargo fmt --check + clippy -D warnings + cargo check
pnpm check                         # svelte-kit sync + svelte-check（严格 TS 检查）
pnpm validate                      # test（含 vitest + cargo test 与绑定重生成）+ lint + check（每次修改后必跑）

# ---- 构建 ----
pnpm build                         # 仅构建前端，adapter-static 输出 SPA 到 build/（需先 i18n:compile）
pnpm tauri:build                   # 完整打包；tauri:build:local 仅编译不打包（--no-bundle）
pnpm clean                         # 清理构建产物；clean:frontend / clean:backend 分侧清理

# ---- 测试 ----
pnpm test                          # vitest + cargo test 单次运行；test:vitest 仅前端单测；test:watch 为监听模式
pnpm test:cargo                    # 后端测试，同时重新生成 src/libs/commands/bindings.ts

# ---- 依赖升级（禁止裸 pnpm update / cargo update，必须经以下脚本以触发对齐检查） ----
pnpm update:frontend                # 仅前端：caret 范围内升级（不跨 major）
pnpm update:backend                 # 仅后端：semver 兼容范围内升级（不跨 major）
pnpm update:all                     # 双端一起升 + check:sync 对齐检查；之后必跑 audit 双端 + validate，单独提交 chore(deps:update)

# ---- 国际化 / 图标 / 发布 ----
pnpm i18n:compile                  # 编译前端国际化文案，改完 messages/ 后必须执行
pnpm i18n:translate                # 走 inlang 机器翻译
pnpm tauri:icon                    # 由 static/icon.png 生成各平台图标
pnpm changelog                     # git-cliff 生成变更日志；changelog:preview 仅预览未发布部分
pnpm release                       # bumpp 联动升级三处版本号（package.json + tauri.conf.json + Cargo.toml）
```

---

## 5. 架构规范

> 优先级高于代码风格。违反以下任一条即视为不合格修改。

1. **分层单向依赖：** `Svelte 组件 → libs/commands（链式 API）→ commands/（Rust 薄封装）→ features/ · cores/`，禁止反向调用与跨层跳跃（如组件裸 `invoke`、命令层写业务逻辑）。纯前端 UI 偏好（如布局 / 字体等外观偏好走 `localStorage`）是有意例外，不经 commands 链。
2. **前后端契约优先：** 以 tauri-specta 为准（见第 10 章第 2 条）。改命令必须先改 `features/` + `commands/`（含 `#[tauri::command]` + `#[specta::specta]` + `collect_commands!`），再 `cargo test` 重生成 `bindings.ts`，最后改前端调用。禁止手改 `bindings.ts`、禁止跳过重生成直接改前端。
3. **状态归属：** Svelte 5 runes（`$state` / `$props` / `$effect`）优先，禁用旧式 store；页面私有状态用局部 state，跨页面共享才考虑提升。主题经 `mode-watcher` 全局管理，语言对齐走第 10 章第 1 条数据流，禁止各页面自建语言状态。
4. **错误处理分层：** `features/` 返回业务错误 → `commands/` 转为 `CommandResult`（错误统一 `CommandError::Internal(...)`，`anyhow::Error` 经已有 `From` 自动转换）→ 前端经 `.result() / .value() / .success() / .failed()` 处理。前端 `.failed()` 回调形状为 `{ kind, message }`；未注册 `.failed()` 的失败会自动经 plugin-log 上报，不会静默丢失。禁止在 `features/` 内直接耦合 Tauri 运行时。
5. **配置外置与同步：** Paraglide 编译选项唯一来源是 `src/libs/i18n/project.inlang/paraglide.config.ts`（CLI 与 vite 插件都读它，禁止往 `package.json` / `vite.config.ts` 各写一份）；路径别名唯一来源是 `svelte.config.ts` 的 `kit.alias`。新增外部 URL 必须同步更新 `capabilities/*.json` 与 `tauri.conf.json` 的 CSP（`updater` 下载端点由 Rust 侧发起，不受 WebView CSP 约束，无需同步 CSP，见第 8 章第 2 条）。新增配置必须检查第 10 章第 4 条成对维护表。路径别名共四个（`$assets` / `$components` / `$hooks` / `$libs`），均由该处定义；其中 hooks 一律经 `$hooks` 引用，禁止写 `$libs/hooks`（两者都能解析）——`components.json` 的 `aliases.hooks` 即 `$hooks`，CLI 生成的组件也走 `$hooks`，保持唯一写法才能避免漂移。新增 / 变更配置项必须同步 `cores/config.rs` 的 `CURRENT_SCHEMA_VERSION` 与 `MIGRATIONS` 迁移链（仅改动已有字段语义时才递增版本），否则老用户配置会静默失效。配置写锁三入口（`update` / `save_locale` / `migrate`，内部函数不加锁，新增写入路径必须走其一）；`update` 内幂等 `migrate`，存储不可用直接失败（不基于回落默认值决策），自启落盘失败回滚操作系统状态。
6. **崩溃边界：** 前端 `ErrorBoundary`（经 `@tauri-apps/plugin-log` 上报，堆栈仅 dev 显示；同步 `console.error` 留底 + 2s 节流防刷屏）+ 后端 `cores/system.rs` panic 钩子（先走常规日志，日志不可用时落盘到 `%TEMP%/tool-dock_crash.log` 兜底，二选一而非双写；超 512KB 轮转为 `.1` 仅保留一份）。两者互不替代，禁止另加并行机制。
7. **语言数据流：** 唯一持久化点是后端 `config.json` 的 `locale` 键（`cores/config.rs`），类型为 `cores/locale.rs` 的 `Locale` 枚举（经 specta 导出为 `"en" | "zh-CN"` 联合类型，禁止裸 `string` 或类型断言；`Locale::parse` 仅支持这两项，繁体一律归 `zh-CN`，加语言时同步改解析、`locales/*.yml` 与前端文案）。启动时后端先读持久化值、缺失则 `tauri_plugin_os::locale()` 探测并落库，再 `rust_i18n::set_locale`；前端在根布局 `onMount` 经 `hydrateAndAlignLocale` 后台水合配置（失败重试一次，不阻断首帧；放 `onMount` 而非 `load()`——Tauri `invoke` 底层走 `window.fetch` 调 `ipc://` 端点，`load` 内调用触发 SvelteKit 追踪警告），再用 `config.locale` 调 Paraglide 的 `setLocale(locale, { reload: false })` 对齐（否则整页重载 + 语言闪烁）；用户切换时 `commands.updateConfig({ locale })` 先落盘再改内存，运行时 `rust_i18n` 由 `config::apply_runtime_effects` 统一同步，前端随后用 Paraglide 默认 `setLocale` 重载。更新命令 `restart` 桌面端永不结算（勿 `await`）、移动端返回错误走 `.failed()`；检查 120s 超时兜底，待重启态不被新检查覆盖。完整流程见第 10 章第 1 条。
8. **初始化与订阅：** `initAppearance` 类“读存储 → 写内存 → 应用到 DOM”的初始化只允许跑一次，禁止进入跟踪型 `$effect`（init 读写同一批状态，被跟踪即形成“改动→重初始化”回路：未落盘的预览会被存量覆盖，字重滑块冻住即此因；必须包 `untrack`）。`app.html` 的首帧内联脚本是 init 在 JS 就绪前的同步镜像，两处键名与分支逻辑必须同步改。`localStorage` 存量键名即契约（改名即视为放弃老用户存量；存量裸键名如 `layout-name` / `color-theme` / `font-*` 冻结不再改，仅新增键统一加 `app.` 前缀），新增键禁止裸通用名。

---

## 6. 代码风格

### 6.1 通用

- 语言：注释用中文（专有名词与包名保留英文），提交信息一律英文小写；变量 / 函数名一律英文。
- 命名：文件 `kebab-case`，类 / 组件 `PascalCase`，函数 / 变量 `camelCase`，常量 `UPPER_SNAKE_CASE`，Rust 模块 `snake_case`。
- 枚举取值：会落盘的取值（布局名等）用语义名，禁用 `default` / `other` 之类只表达"被选中"的占位名。
- 函数：单个函数尽可能不超过 50 行，圈复杂度 < 10；过长的复杂函数按照逻辑进行拆分。
- 注释：只解释 Why（为什么这样做），不复述 What（代码在做什么）。

### 6.2 前端

- 组件：只用 Svelte 5 runes 写法（`$state` / `$props` / `$effect`），新代码禁用旧式 store；Props 必须显式定义类型，禁止 `any` 透传。
- shadcn-svelte 组件在引用时尽可能使用全名引用如： `AlertDialogTrigger` 避免使用 `AlertDialog.Trigger`
- 数据请求：统一经 `src/libs/commands` 链式 API（`.value() / .result() / .success() / .failed()`，回调在 `await / value()` 之前链式注册）；禁止在组件内手写裸 `invoke`。
- 样式：Tailwind v4 + `cn()` 合并类名，优先主题变量（`src/routes/layout.css`，Geist Variable 字体，`.dark` 暗色变体）；禁止散落硬编码色值。Prettier（`double` 双引号、分号、2 空格缩进、行宽 100、LF）+ ESLint（`js recommended`、`typescript-eslint recommended`、`svelte flat/recommended` + `flat/prettier`）。
- 导入顺序强制：内建模块 → 第三方 → 类型 → `$assets` / `$hooks` → `$components` → `$libs` → 相对路径。
- TSDoc（`/** */`，短句保持单行）只写契约与非直观处；**不要在 TSDoc 里写代码示例**（prettier-jsdoc 会重排成散文，示例用 `//` 注释块）；描述别用英文小写标识符开头（`jsdocCapitalizeDescription` 会首字母大写，中文开头可规避）。
- 示例：

```ts
// ✅ 推荐：链式 API + 默认值回落
import commands from "$libs/commands";
const config = await commands.getConfig().value(undefined);
const result = await commands.getConfig().result();
await commands.updateConfig({ locale }).success(onOk).failed(onFail);

// ❌ 禁止：裸 invoke + 手改绑定
import { invoke } from "@tauri-apps/api/core";
const config = await invoke<string>("get_config");
```

### 6.3 后端

- 命令层：薄封装 + `#[tauri::command]` + `#[specta::specta]` + `collect_commands!` 注册，返回 `CommandResult`；`features/` 保持纯函数，不依赖 Tauri 运行时。
- 格式：`cargo fmt`（行宽 100、4 空格、`use_field_init_shorthand`、`use_try_shorthand`）+ Clippy `pedantic` / `nursery` / `cargo` 组记为 `warn`，CI 中 `-D warnings`。`main.rs` 豁免 `clippy::module_name_repetitions`；`commands/mod.rs` 豁免 `unnecessary_wraps`；`lib.rs:55` 的 `.expect()` 为 Tauri 官方模板惯用法（`run()` 已声明 `# Panics`），予以豁免；`lib.rs:run()` 豁免 `clippy::large_stack_frames`（`generate_context!` 生成闭包逐项装配各插件 JS API 与权限表，上游已单开 8MiB 栈线程，加插件即涨是预期行为）。
- 日志：关键分支打 `info`，异常打 `error`；崩溃走 `cores/system.rs` panic 钩子。禁止打印密钥 / Token / 完整 PII。
- 注释：`//!` 写模块职责、`///` 写导出项契约与"为什么"、`//` 只解释非直观取舍；文档注释标识符必须加反引号（Clippy `doc_markdown` 会拦截 `WebKitGTK`、`AppImage` 类驼峰词）。
- 示例：

```rust
// ✅ 推荐：commands 薄封装 + features 纯逻辑
#[tauri::command]
#[specta::specta]
pub fn my_command(input: String) -> CommandResult<String> {
    Ok(features::my_feature::run(&input)?)
}
// ❌ 禁止：commands 内写业务逻辑 / features 内依赖 Tauri 运行时
```

### 6.4 前后端交互约定

- 调用：一律 `libs/commands` 链式 API，禁止裸 `invoke`；`EnhancedCommand` 三种用法见 6.2 示例（取值 `.value()` / 分支 `.result()` / 事务 `.success()/.failed()`）。关键取值用 `.result()` 按 `status` 分支，`.value()` 的回落会把失败与空数据压成同一个值。回调内不得做关键状态变更（回调抛错只上报不扩散，`await` 仍得原结果）。
- 错误体：后端 `CommandError::Internal(...)`，前端 `.failed()` 收到 `{ kind, message }`；`anyhow::Error` 经 `From` 自动转换。
- 语言类型：前后端共用 specta 导出的 `"en" | "zh-CN"` 联合类型，禁止裸 `string` 或类型断言。
- 时间：后端 `chrono`；业务时间全链路 UTC，展示层转本地时区；崩溃日志时间戳使用本地时间字符串（`chrono::Local`，RFC3339 带时区偏移）；禁止无时区信息的本地时间与 UTC 解析混用。
- Prettier 硬性约束：`plugins` 数组中 `prettier-plugin-tailwindcss` **必须在最后**；`importOrderTypeScriptVersion` 必须与 `package.json` 的 `typescript` 版本同步。

---

## 7. 提交规范

- **格式：** Conventional Commits，由 commitlint（`commit-msg` 钩子）强制：类型小写、描述非空。
  - `type`：`feat | fix | refactor | doc | docs | test | chore | perf | build | ci | style | revert`，需与 `cliff.toml` 解析器对齐
  - 示例：`feat(i18n): persist locale in backend` / `fix(auth): avoid reload loop on startup`
- **语言：提交信息一律英文**：`type`、`scope`、描述与正文全部英文，即使本轮对话使用中文也不生成中文提交信息。
- **粒度：** 一次提交只做一件事；功能 + 修格式分开提交；禁止 `fix bug` / `update` 等无意义信息。
- **提交时机：** 由开发者手动提交，不允许自动提交，提交前务必运行 `pnpm format` 与 `pnpm validate` 调整与校验。
- **版本：** 三处版本号经 `pnpm release`（bumpp）联动升级：`package.json` + `src-tauri/tauri.conf.json` + `src-tauri/Cargo.toml`；

CI（`.github/workflows/ci.yml`）在 `main` 分支上按变更路径触发：

- **前端**：`i18n:compile` → `lint:frontend` → `check` → `test` → `build`。
- **后端**：`fmt --check` → `clippy -D warnings`（警告即错误）→ `check` → `test` → `git diff --exit-code src/libs/commands/bindings.ts`（绑定同步校验；`bindings.ts` 已纳入 backend 的 `changes` 过滤器，单改它也会触发后端校验）。
- 门禁作业（`ci-passed`）同时拦截 `changes`（变更检测）作业的失败——它失败时前后端都会被跳过，只查前后端会漏放。

发布流水线（`.github/workflows/release.yml`）会校验三处版本号（`package.json` + `tauri.conf.json` + `Cargo.toml`）与 `v*` 标签一致。

---

## 8. 安全红线

> 触碰以下任一条直接回滚，禁止讨论例外。

1. **禁止提交密钥：** `.env`、`*secret*`、`*key.pem`、Token、AK/SK 永不进仓库。 Updater 私钥等只走环境变量 / 密钥管理服务。
2. **禁止修改项（无书面批准不得触碰）：**
   - `capabilities/*.json` 权限配置与 `tauri.conf.json` 的 CSP / updater 配置（新增外部 URL 必须同步两处，见 5.5）
   - 已发布的 GitHub Release / `v*` 标签对应的版本号（三处联动，见第 7 节）
   - `.github/workflows/release.yml` 发布流水线
   - `bindings.ts` 生成物（只允许 `cargo test` 重生成，禁止手改）
3. **输入安全：** 所有外部输入必须校验 + 转义；Svelte 渲染默认转义，禁止 `{@html ...}` 直渲用户输入；Rust 侧字符串拼接 shell / SQL 时必须参数化。
4. **越权与能力最小化：** Tauri capability 按需最小授权，禁止全开 `*`；前端隐藏按钮不算权限控制，涉及本地文件 / 系统能力时后端必须二次校验。前端 `openExternal` 仅放行 `http(s)`（先剥 `git+` 前缀再校验），其它 scheme 直接拒绝并上报。`store` / `autostart` / `window-state` / `single-instance` 仅 Rust 侧注册使用（见 `lib.rs`），无前端 JS 直调，故不在 `plugins.json` 声明，属刻意最小授权；新增前端直调时再按需补声明。
5. **依赖安全：** 禁止引入未知来源依赖；新增依赖必须说明理由；前端经 `pnpm audit`、后端经 `cargo audit / cargo deny`（如已配置）检查无高危漏洞；依赖升级走手动 `chore(deps:update)` 提交，升级入口一律 `pnpm update:frontend | update:backend | update:all`（禁止裸 `pnpm update` / `cargo update`，前者是 pnpm 内建命令，会绕过脚本门禁）。

---

## 9. 测试验证

> 每次修改完成后必须执行，未通过禁止声称"完成"。
> 硬性规则：任何文件改动完成后，都必须先 `pnpm format`，再 `pnpm validate`；两者全绿前不得宣称完成、不得提交。钩子只覆盖暂存文件，是最后防线而非替代。

| 修改范围                                               | 最低验证要求                                                                                                   |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- |
| 仅前端 UI / 文案 / 配置                                | `pnpm format` + `pnpm validate`（含 `prettier --check`、`eslint`、`svelte-check`、`vitest`、`cargo test`）通过 |
| 前端逻辑 / commands 封装                               | 上述 + `pnpm test:vitest src/tests/<相关用例>` 通过                                                            |
| 后端 features / cores / commands                       | `pnpm test:cargo`（重写绑定 + 后端测试）+ `pnpm format` + `pnpm validate` 通过                                 |
| 命令契约变更（新增 / 改签名 / 改 `collect_commands!`） | 上述 + 确认 `git diff src/libs/commands/bindings.ts` 有同步更新 + 前后端联调                                   |
| 前端文案变更（`messages/`）                            | `pnpm i18n:compile` + `pnpm format` + `pnpm validate` 通过（跳过 compile 会导致 `build` / CI 失败）            |
| 发布相关（版本号 / CI / 打包）                         | `pnpm build` + `pnpm tauri:build:local --no-bundle`（按需）+ CI 全绿                                           |

**回归 checklist（提交前逐项确认）：**

- [ ] 修改功能以外的旧功能主流程不受影响
- [ ] 新增逻辑有单测覆盖核心分支（含异常分支；前端测试集中 `src/tests/` 按源路径镜像，后端 `cargo test`）
- [ ] 无 `console.log / print / debugger` 残留（失败上报走 plugin-log / 后端 log）
- [ ] 无硬编码 URL / 密钥 / 本地绝对路径（外部 URL 已同步 capabilities + CSP）
- [ ] 改文案已补跑 `i18n:compile`；`validate` 已含 `cargo test`（含绑定重生成），无需另跑

**提交前钩子**（`lint-staged`，最后防线）：`*.{js,ts,svelte}` → `prettier --write` + `eslint --fix`；`*.{json,md,html,css,yml,yaml}` → prettier（范围须与 CI 的 `prettier --check .` 一致）；`**/*.rs` → 暂存文件 `cargo fmt` + 工作区级 `clippy -D warnings`。

---

## 10. 特殊流程

> 正常 CRUD 之外的"坑"，做相关任务前必读。

1. **国际化流程：** 前端在 `src/libs/i18n/project.inlang/` 下改文案 → `pnpm i18n:compile` → 用 `m.<键>(...)` 按键取值、`setLocale(locale)` 切换（启动对齐传 `{ reload: false }`）；后端改 `src-tauri/locales/*.yml` → `rust_i18n::t!(...)` 取文案。`paraglide/` 生成物已忽略提交，禁止编辑。`project.inlang/paraglide.config.ts` 与 `settings.json` 已被该目录 `.gitignore` 放行，直接 `git add` 即可，无需 `git add -f`。
2. **命令契约流程：** `features/<name>.rs` 实现业务（`features/mod.rs` 声明 `pub mod <name>;`）→ `commands/<name>.rs` 薄封装（`CommandResult` + 双注解 + 进 `collect_commands!`）→ `cargo test` 重生成 `bindings.ts` → 前端链式 API 调用 → 联调。禁止跳过文档 / 生成步骤直接改代码。
3. **发版流程：** 必须由开发者手动发版，`main` 受保护 → 提 PR → CI 全绿 → Code Review → Squash 合并 → `pnpm release` 联动三处版本号 → 打 tag `vX.Y.Z`（须与 `tauri.conf.json` 一致）→ `release.yml` 自动打包 → `pnpm changelog` 生成日志。
4. **必须成对维护的配置：**
   - 两个 workflow 里 `dtolnay/rust-toolchain` 的 `toolchain: 1.99.0` ↔ `rust-toolchain.toml`
   - `.prettierrc` 的 `importOrderTypeScriptVersion` ↔ `package.json` 的 `typescript`
   - Tauri 插件前后端版本：`pnpm-lock.yaml` 的 npm 包 ↔ `Cargo.lock` 的 cargo 包（major.minor 一致，升级入口为 `pnpm update:all`，由 `scripts/check-version-sync.ts` 在 `pnpm check` 中校验）
   - `.prettierignore` ↔ `.gitignore` 中的构建产物（Prettier 不读 `.gitignore`）
   - CI backend 的 `changes` 路径过滤器 ↔ 新增的后端配置文件
5. **构建与忽略：** 构建产物（`target/`、`build/`、`.svelte-kit/`、`src-tauri/gen/`、`node_modules/`、`src/libs/i18n/paraglide/`）均已忽略；`bindings.ts` 虽是生成物但**需要提交**；`Cargo.lock` 需要提交；`static/icon.png` 为图标源文件（`pnpm tauri:icon` 生成各平台图标）。Vite 固定端口 `1420`，忽略监听 `src-tauri/**`，`clearScreen: false` 以保留 Rust 日志。主窗口初始 `visible: false`（防恢复闪烁），由 `cores/config.rs` 的 `setup` 按记住窗口配置恢复后统一 `show`，任何提前返回前必须显示，否则永久黑屏；仅恢复 `main` 窗口。
6. **工具新增流程：** `messages/` 加 `tool_<id>_name/description`（分类新增同步加 `tool_category_*`）→ `pnpm i18n:compile` → `libs/tools/registry.ts` 加条目（`id` / `path` / `category` / 文案函数 / 图标；新分类先扩 `TOOL_CATEGORIES` 元组）→ `routes/(tools)/<分类>/<工具名>/+page.svelte` 建路由（只做组装，内部组件放 `components/tools/<分类>/<工具名>/`）→ 标题栏经 `resolveTool` 自动取名字，无需改 `(tools)/+layout.svelte` → 深链详情分支按注册表校验放行（列表页 `/tools` 已有映射，新工具路径自动生效，仅需在前后端深链单测补用例）→ 前后端单测（注册表 + 网格 + 布局）→ 联调。禁止在首页 / 网格 / 标题栏 / 深链四处任一处硬编码工具信息。

---

## 11. 常见误区

1. **"裸 invoke 图快"：** 省掉 `libs/commands` 链式层会导致错误上报与类型契约丢失。—— 正确：先走链式 API（`.value() / .result() / .success()/.failed()`）。
2. **"手改 bindings.ts 省事"：** 生成物手改后下次 `cargo test` 即被覆盖，且 CI 会校验同步失败。—— 正确：改 Rust 侧后重跑 `cargo test`。
3. **"改完文案不用 compile"：** 跳过 `pnpm i18n:compile` 会导致 `build` / CI 失败。—— 正确：改 `messages/` 必跑 compile。
4. **"别名复制一份方便"：** 往 `vite.config.ts` 复制 `kit.alias`、往 `package.json` 复制 Paraglide 参数必然漂移。—— 正确：各认唯一来源（`svelte.config.ts` / `paraglide.config.ts`）。
5. **"TSDoc 里写示例"：** prettier-jsdoc 会把示例重排成散文，`jsdocCapitalizeDescription` 会大写英文小写开头。—— 正确：示例用 `//` 注释块，描述用中文开头。
6. **"提交前不跑 validate"：** CI 挂掉再修比本地修贵 5 倍，且钩子只看暂存文件。—— 正确：`pnpm format` + `pnpm validate`（+ 按需 `cargo test`）全绿再推。
7. **"CLI 加组件一路回车"：** `shadcn-svelte add` 遇到已安装组件会要求覆写，一路确认会污染 `button` / `input` 等无关文件。—— 正确：逐项拒绝覆写，事后用 `git diff` 回退无关文件。

---

## 12. 工作流程

> 智能体处理任务的标准链路：

```text
读 AGENTS.md → 定位目录/分层 → 小步修改 → 补生成步骤 → pnpm format → pnpm validate →
自查安全红线 → 汇报修改结果 → 给出下一步建议
```

**任务开始前：**

1. 通读本文件；
2. 用 `git status / git diff` 确认工作区干净，从最新 `main` 拉分支 `feat/* | fix/* | chore/*`。

**任务执行中：**

1. 严格按第 3、5 章的分层与边界写代码；
2. 按改动类型补齐生成步骤（改 `messages/` → `i18n:compile`；改后端命令 → `cargo test` 重生成绑定；改后端逻辑即使不涉及签名也跑 `cargo test`）；
3. 每完成一个小步即 `pnpm format` + `pnpm validate`，红灯立即停下修复。

**任务收尾：**

1. 跑第 9 章对应级别的完整验证；
2. 自查安全红线
3. 汇报修改结果并给出下一步建议

---

<!-- 维护说明：技术选型或流程变更时，优先更新本文档再改代码 -->
