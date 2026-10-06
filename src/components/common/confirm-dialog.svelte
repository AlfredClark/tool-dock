<script lang="ts">
  // 通用确认弹窗：标题 + 描述 + 取消/确认双按钮；点确认先关弹窗再调回调。
  import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
  } from "$components/shadcn-svelte/alert-dialog";

  let {
    open = $bindable(false),
    title,
    description,
    cancelLabel,
    confirmLabel,
    onConfirm,
  }: {
    open: boolean;
    title: string;
    description: string;
    cancelLabel: string;
    confirmLabel: string;
    onConfirm: () => unknown;
  } = $props();

  /** 确认先关弹窗再执行业务；回调可为 async，抛错不影响关闭 */
  function handleConfirm(): void {
    open = false;
    void onConfirm();
  }
</script>

<AlertDialog bind:open>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>{title}</AlertDialogTitle>
      <AlertDialogDescription>{description}</AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>{cancelLabel}</AlertDialogCancel>
      <AlertDialogAction onclick={handleConfirm}>{confirmLabel}</AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
