// Tauri 没有 Node.js 服务器来做 SSR，使用静态适配器并回退到 `index.html`，将网站置于 SPA 模式
export const ssr = false;

// 启动时的配置水合与语言对齐不在 `load()` 里做：Tauri 的 `invoke` 底层走 `window.fetch`
// 调 `ipc://` 端点，放在 `load` 内会触发 SvelteKit 的 fetch 追踪警告；且水合本就是后台
// fire-and-forget（首帧不等水合），改由根布局 `onMount` 经 `hydrateAndAlignLocale` 完成。
