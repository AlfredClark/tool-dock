<script lang="ts">
  // 首页英雄区：技术栈图标 + 应用名 + 项目简介，在内容区内水平垂直居中，无业务逻辑。
  import { m } from "$libs/i18n/paraglide/messages";

  const appName = __APP_TAURI_CONF__.productName;

  /** 技术栈图标：static 目录原样拷贝到产物根目录，经根路径引用（CSP 仅放行 self 源）。 */
  const stackIcons: { src: string; alt: string }[] = [
    { src: "/tauri.svg", alt: "Tauri" },
    { src: "/svelte.svg", alt: "Svelte" },
    { src: "/vite.svg", alt: "Vite" },
  ];
</script>

<div class="flex h-full w-full flex-col items-center justify-center gap-6 p-6 text-center">
  <div class="flex items-center justify-center gap-6">
    {#each stackIcons as icon (icon.src)}
      <img src={icon.src} alt={icon.alt} class="h-16 w-16 object-contain" />
    {/each}
  </div>
  <h1 class="text-2xl font-medium">{appName}</h1>
  <p class="max-w-md text-sm text-muted-foreground">{m.page_home_intro()}</p>
</div>
