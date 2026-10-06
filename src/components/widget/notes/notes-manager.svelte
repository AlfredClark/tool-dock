<script lang="ts">
  // 笔记管理：顶部新增表单 + 列表行内编辑 + 删除二次确认，数据经 libs/notes，零 props。
  // 校验失败 toast 提示（键与 validate 同源）；删除经通用确认弹窗；时间展示转本地时区。
  import { onMount } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import ConfirmDialog from "$components/common/confirm-dialog.svelte";
  import { Button } from "$components/shadcn-svelte/button";
  import { Input } from "$components/shadcn-svelte/input";
  import { Textarea } from "$components/shadcn-svelte/textarea";
  import CardSection from "$components/common/card-section.svelte";
  import { createNote, deleteNote, listNotes, NOTES_PAGE_SIZE, updateNote } from "$libs/notes/db";
  import type { Note, NoteValidationError } from "$libs/notes/types";
  import { validateNote } from "$libs/notes/validate";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 列表 + 新增草稿 + 行内编辑态 + 删除确认，全页私有（切页不保留） */
  let notes = $state<Note[]>([]);
  let draftTitle = $state("");
  let draftBody = $state("");
  let busy = $state(false);
  let loadingMore = $state(false);
  let hasMore = $state(false);
  let editingId = $state<number | null>(null);
  let editTitle = $state("");
  let editBody = $state("");
  let deletingId = $state<number | null>(null);
  let confirmOpen = $state(false);

  /** 首帧加载；失败 toast，列表保持空并显示空态 */
  onMount(() => {
    void refresh();
  });

  /** 重拉列表；各写操作成功后复用，保持展示与库一致 */
  async function refresh(): Promise<void> {
    try {
      notes = await listNotes();
      hasMore = notes.length === NOTES_PAGE_SIZE;
    } catch {
      toast.error(m.notes_load_failed());
    }
  }

  /** 追加下一页；写操作仍走 refresh 重拉，此处只追加展示 */
  async function loadMore(): Promise<void> {
    if (busy || loadingMore || !hasMore) return;
    loadingMore = true;
    try {
      const more = await listNotes(NOTES_PAGE_SIZE, notes.length);
      notes = [...notes, ...more];
      hasMore = more.length === NOTES_PAGE_SIZE;
    } catch {
      toast.error(m.notes_load_failed());
    } finally {
      loadingMore = false;
    }
  }

  /** 校验键转文案：键集合与 validate 返回自治，不做类型断言 */
  function validationMessage(key: NoteValidationError): string {
    return m[key]();
  }

  /** 新增：先校验再写库，成功清空草稿并重拉 */
  async function handleAdd(): Promise<void> {
    if (busy) return;
    const invalid = validateNote(draftTitle, draftBody);
    if (invalid) {
      toast.error(validationMessage(invalid));
      return;
    }
    busy = true;
    try {
      await createNote(draftTitle.trim(), draftBody);
      draftTitle = "";
      draftBody = "";
      await refresh();
    } catch {
      toast.error(m.notes_save_failed());
    } finally {
      busy = false;
    }
  }

  /** 进入行内编辑：草稿取当前行快照，取消即丢弃 */
  function startEdit(note: Note): void {
    editingId = note.id;
    editTitle = note.title;
    editBody = note.body;
  }

  /** 保存编辑：校验通过才写库，成功退出编辑态并重拉 */
  async function handleSaveEdit(id: number): Promise<void> {
    if (busy) return;
    const invalid = validateNote(editTitle, editBody);
    if (invalid) {
      toast.error(validationMessage(invalid));
      return;
    }
    busy = true;
    try {
      await updateNote(id, editTitle.trim(), editBody);
      editingId = null;
      await refresh();
    } catch {
      toast.error(m.notes_save_failed());
    } finally {
      busy = false;
    }
  }

  /** 删除二次确认：先开弹窗，确认回调里真删 */
  function askDelete(id: number): void {
    deletingId = id;
    confirmOpen = true;
  }

  /** 确认删除：目标已消失按幂等成功处理，重拉即可 */
  async function handleConfirmDelete(): Promise<void> {
    if (deletingId === null || busy) return;
    busy = true;
    try {
      await deleteNote(deletingId);
      deletingId = null;
      await refresh();
    } catch {
      toast.error(m.notes_delete_failed());
    } finally {
      busy = false;
    }
  }

  /** UTC ISO 转本地展示；脏数据回落原文，不抛错 */
  function formatLocal(iso: string): string {
    const time = new Date(iso).getTime();
    if (Number.isNaN(time)) return iso;
    return new Date(time).toLocaleString();
  }

  /** 行时间展示缓存：草稿键入等重渲染不重复做 toLocaleString，随列表整体更新 */
  const formattedTimes = $derived.by(() => {
    const cache = new SvelteMap<number, string>();
    for (const note of notes) {
      cache.set(note.id, formatLocal(note.updated_at));
    }
    return cache;
  });
</script>

<CardSection title={m.notes_page_title()} description={m.notes_page_description()}>
  <div class="flex flex-col gap-2">
    <Input bind:value={draftTitle} maxlength={100} placeholder={m.notes_add_title_placeholder()} />
    <Textarea bind:value={draftBody} placeholder={m.notes_add_body_placeholder()} />
    <div>
      <Button size="sm" disabled={busy} onclick={() => void handleAdd()}>
        {m.notes_add_button()}
      </Button>
    </div>
  </div>

  {#if notes.length === 0}
    <p class="text-sm text-muted-foreground">{m.notes_empty_hint()}</p>
  {:else}
    <div class="flex flex-col gap-3">
      {#each notes as note (note.id)}
        {#if editingId === note.id}
          <div class="flex flex-col gap-2 rounded-lg border p-3">
            <Input bind:value={editTitle} maxlength={100} />
            <Textarea bind:value={editBody} />
            <div class="flex gap-2">
              <Button size="sm" disabled={busy} onclick={() => void handleSaveEdit(note.id)}>
                {m.notes_save_button()}
              </Button>
              <Button size="sm" variant="outline" onclick={() => (editingId = null)}>
                {m.notes_cancel_button()}
              </Button>
            </div>
          </div>
        {:else}
          <div class="flex flex-col gap-1 rounded-lg border p-3">
            <p class="text-sm font-medium">{note.title}</p>
            {#if note.body}
              <p class="text-sm whitespace-pre-wrap text-muted-foreground">{note.body}</p>
            {/if}
            <p class="text-xs text-muted-foreground">
              {formattedTimes.get(note.id) ?? note.updated_at}
            </p>
            <div class="flex gap-2 pt-1">
              <Button size="sm" variant="outline" onclick={() => startEdit(note)}>
                {m.notes_edit_button()}
              </Button>
              <Button size="sm" variant="outline" onclick={() => askDelete(note.id)}>
                {m.notes_delete_button()}
              </Button>
            </div>
          </div>
        {/if}
      {/each}
    </div>
    {#if hasMore}
      <div>
        <Button size="sm" variant="outline" disabled={loadingMore} onclick={() => void loadMore()}>
          {m.notes_load_more_button()}
        </Button>
      </div>
    {/if}
  {/if}
</CardSection>

<ConfirmDialog
  bind:open={confirmOpen}
  title={m.notes_delete_title()}
  description={m.notes_delete_description()}
  cancelLabel={m.notes_cancel_button()}
  confirmLabel={m.notes_delete_ok()}
  onConfirm={() => void handleConfirmDelete()}
/>
