<script lang="ts">
  // CodeMirror 6 封装：onMount 建 EditorView、$effect 同步内容/语言/只读、onDestroy 销毁。
  // 语言经 Compartment 重配（切换格式不重建编辑器）；主题走 CSS 变量，随 mode-watcher 的 .dark 自动适配。
  import { basicSetup } from "codemirror";
  import { Compartment, EditorState } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { onMount } from "svelte";
  import { extensionForFormat } from "./languages";
  import type { ConvertFormat } from "./formats";
  import { cn } from "$libs/utils/shadcn-svelte";

  interface Props {
    /** 编辑器内容（外部真值，内部输入经 onInput 回写） */
    value: string;
    /** 当前高亮格式：null 即纯文本（输入端自动识别未命中时） */
    format: ConvertFormat | null;
    /** 是否只读（输出端） */
    readonly?: boolean;
    /** 无障碍标签（输入端/输出端文案） */
    label: string;
    /** 内容变化回调（仅用户输入触发，外部同步不回环） */
    onInput?: (value: string) => void;
    /** 行跳转请求（消费语义）：`seq` 变化即跳到该行行首并滚动可见，越界钳制到文档末行 */
    scrollTick?: { line: number; seq: number } | null;
  }

  let { value, format, readonly = false, label, onInput, scrollTick = null }: Props = $props();

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | undefined = undefined;

  const languageConf = new Compartment();
  const readonlyConf = new Compartment();

  // 编辑器面板透明走页面背景，字体继承外观设置的界面字体
  const appTheme = EditorView.theme({
    "&": { height: "100%", backgroundColor: "transparent" },
    ".cm-content": { fontFamily: "var(--app-font-family)" },
    ".cm-scroller": { overflow: "auto" },
  });

  onMount(() => {
    const startState = EditorState.create({
      doc: value,
      extensions: [
        basicSetup,
        appTheme,
        languageConf.of(format ? extensionForFormat(format) : []),
        readonlyConf.of(EditorState.readOnly.of(readonly)),
        // 更新监听只注册一次；onInput 是 props 解构的响应式引用，闭包内读取恒为最新值
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onInput?.(update.state.doc.toString());
          }
        }),
      ],
    });
    view = new EditorView({ parent: host, state: startState });
    return () => {
      view?.destroy();
      view = undefined;
    };
  });

  // 外部内容变化时同步进编辑器：先比对才 dispatch，用户正在输入时不回环覆盖
  $effect(() => {
    const next = value;
    if (view && next !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next } });
    }
  });

  // 格式变化时重配高亮扩展
  $effect(() => {
    view?.dispatch({ effects: languageConf.reconfigure(format ? extensionForFormat(format) : []) });
  });

  // 只读变化时重配（输出端恒只读，此分支主要兜未来复用）
  $effect(() => {
    view?.dispatch({ effects: readonlyConf.reconfigure(EditorState.readOnly.of(readonly)) });
  });

  // 行跳转请求：落到该行行首并滚动可见；jsdom 下 CM 测量不可用，此分支仅真实环境触发
  $effect(() => {
    const tick = scrollTick;
    if (!view || !tick) return;
    const target = Math.min(Math.max(tick.line, 1), view.state.doc.lines);
    const pos = view.state.doc.line(target).from;
    view.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
  });
</script>

<div
  bind:this={host}
  class={cn("h-full min-h-0 w-full overflow-hidden text-sm")}
  aria-label={label}
></div>
