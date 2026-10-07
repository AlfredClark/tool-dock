<script lang="ts">
  // 数据互转编辑器测试替身：CodeMirror 在 jsdom 下无 Range 测量能力，键盘输入不可靠，
  // 此处用原生 textarea 承接 value/onInput 接线，仅验证 workspace 的调用与分支逻辑。
  // scrollTick 经 data 属性透出，供跳转用例断言。
  interface Props {
    value: string;
    label: string;
    onInput?: (value: string) => void;
    scrollTick?: { line: number; seq: number } | null;
  }

  let { value, label, onInput, scrollTick = null }: Props = $props();
</script>

<textarea
  aria-label={label}
  {value}
  data-scroll-line={scrollTick?.line ?? ""}
  data-scroll-seq={scrollTick?.seq ?? ""}
  oninput={(event) => onInput?.(event.currentTarget.value)}></textarea>
