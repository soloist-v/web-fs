<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { Trash2, X, Loader2, AlertTriangle, Folder, File } from 'lucide-svelte';
  import { deleteEntry } from '$lib/api';
  import { cn } from '$lib/utils';
  import type { FileEntry } from '$lib/types';

  interface Props {
    open: boolean;
    entries: FileEntry[];
    onOpenChange: (open: boolean) => void;
    onDeleted: () => void;
  }

  let { open, entries, onOpenChange, onDeleted }: Props = $props();

  let deleting = $state(false);
  let progress = $state(0);
  let currentFile = $state('');
  let error = $state<string | null>(null);
  let failedEntries = $state<{ entry: FileEntry; error: string }[]>([]);

  $effect(() => {
    if (open) {
      deleting = false;
      progress = 0;
      currentFile = '';
      error = null;
      failedEntries = [];
    }
  });

  const isSingle = $derived(entries.length === 1);
  const hasDirectories = $derived(entries.some((e) => e.is_dir));

  const title = $derived(() => {
    if (isSingle) {
      return entries[0].is_dir ? '删除文件夹' : '删除文件';
    }
    return `删除 ${entries.length} 个项目`;
  });

  const warningText = $derived(() => {
    if (isSingle) {
      const e = entries[0];
      if (e.is_dir) {
        return `将永久删除文件夹 "${e.name}" 及其所有内容，此操作无法撤销。`;
      }
      return `将永久删除文件 "${e.name}"，此操作无法撤销。`;
    }
    if (hasDirectories) {
      return `将永久删除选中的 ${entries.length} 个项目（包含文件夹及其所有内容），此操作无法撤销。`;
    }
    return `将永久删除选中的 ${entries.length} 个文件，此操作无法撤销。`;
  });

  async function handleDelete() {
    if (deleting || entries.length === 0) return;

    deleting = true;
    error = null;
    failedEntries = [];
    progress = 0;

    const failed: { entry: FileEntry; error: string }[] = [];

    for (let i = 0; i < entries.length; i++) {
      const entry = entries[i];
      currentFile = entry.name;
      progress = Math.round(((i) / entries.length) * 100);

      try {
        await deleteEntry(entry.path);
      } catch (err) {
        failed.push({
          entry,
          error: err instanceof Error ? err.message : '删除失败',
        });
      }
    }

    progress = 100;
    currentFile = '';

    if (failed.length > 0) {
      failedEntries = failed;
      deleting = false;
      // If some succeeded, still notify parent to refresh
      if (failed.length < entries.length) {
        onDeleted();
      }
    } else {
      deleting = false;
      onDeleted();
      onOpenChange(false);
    }
  }
</script>

<Dialog.Root {open} onOpenChange={onOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay
      class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm
             data-[state=open]:animate-in data-[state=closed]:animate-out
             data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
    />
    <Dialog.Content
      class={cn(
        'fixed left-1/2 top-1/2 z-50 w-full max-w-md',
        '-translate-x-1/2 -translate-y-1/2',
        'rounded-xl border border-border bg-card shadow-2xl',
        'data-[state=open]:animate-in data-[state=closed]:animate-out',
        'data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0',
        'data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95',
        'data-[state=closed]:slide-out-to-left-1/2 data-[state=open]:slide-in-from-left-1/2',
        'data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-top-[48%]',
      )}
    >
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-border px-6 py-4">
        <div class="flex items-center gap-3">
          <div
            class="flex h-9 w-9 items-center justify-center rounded-lg bg-destructive/10 text-destructive"
          >
            <Trash2 size={18} />
          </div>
          <div>
            <Dialog.Title class="text-base font-semibold text-foreground">
              {title()}
            </Dialog.Title>
            <Dialog.Description class="mt-0.5 text-xs text-muted-foreground">
              该操作不可撤销，请谨慎确认
            </Dialog.Description>
          </div>
        </div>

        {#if !deleting}
          <Dialog.Close
            class={cn(
              'flex h-7 w-7 items-center justify-center rounded-md transition-colors',
              'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
            )}
          >
            <X size={15} />
          </Dialog.Close>
        {/if}
      </div>

      <!-- Body -->
      <div class="px-6 py-5">
        {#if failedEntries.length > 0}
          <!-- Error state after partial deletion -->
          <div class="flex flex-col gap-3">
            <div class="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-3">
              <AlertTriangle size={16} class="mt-0.5 shrink-0 text-destructive" />
              <div class="text-sm">
                <p class="font-medium text-destructive">部分删除失败</p>
                <p class="mt-0.5 text-muted-foreground">
                  {failedEntries.length} 个项目删除失败，其余已成功删除。
                </p>
              </div>
            </div>

            <div class="flex max-h-48 flex-col gap-1.5 overflow-y-auto">
              {#each failedEntries as { entry: failedEntry, error: entryError }}
                <div class="flex items-start gap-2 rounded-md border border-border bg-muted/30 px-3 py-2">
                  <div class="mt-0.5 shrink-0 text-muted-foreground">
                    {#if failedEntry.is_dir}
                      <Folder size={13} />
                    {:else}
                      <File size={13} />
                    {/if}
                  </div>
                  <div class="min-w-0 flex-1">
                    <p class="truncate text-xs font-medium text-foreground">{failedEntry.name}</p>
                    <p class="text-xs text-destructive">{entryError}</p>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {:else if deleting}
          <!-- Deleting progress -->
          <div class="flex flex-col gap-3">
            <div class="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 size={14} class="animate-spin shrink-0" />
              <span class="truncate">
                {currentFile ? `正在删除：${currentFile}` : '正在删除…'}
              </span>
            </div>

            {#if entries.length > 1}
              <div class="flex flex-col gap-1">
                <div class="flex justify-between text-xs text-muted-foreground">
                  <span>进度</span>
                  <span>{progress}%</span>
                </div>
                <div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
                  <div
                    class="h-full rounded-full bg-destructive transition-all duration-300"
                    style="width: {progress}%"
                  ></div>
                </div>
              </div>
            {/if}
          </div>
        {:else}
          <!-- Confirmation state -->
          <div class="flex flex-col gap-4">
            <!-- Warning banner -->
            <div class="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-3">
              <AlertTriangle size={16} class="mt-0.5 shrink-0 text-destructive" />
              <p class="text-sm text-foreground">
                {warningText()}
              </p>
            </div>

            <!-- Entry list (for multiple) -->
            {#if !isSingle && entries.length > 0}
              <div class="flex max-h-48 flex-col gap-1 overflow-y-auto rounded-md border border-border bg-muted/30 p-2">
                {#each entries as entry}
                  <div class="flex items-center gap-2 rounded px-2 py-1 hover:bg-accent/50 transition-colors">
                    <div class="shrink-0 text-muted-foreground">
                      {#if entry.is_dir}
                        <Folder size={13} class="text-yellow-400" />
                      {:else}
                        <File size={13} />
                      {/if}
                    </div>
                    <span class="flex-1 truncate text-xs text-foreground">{entry.name}</span>
                    {#if !entry.is_dir}
                      <span class="shrink-0 text-xs text-muted-foreground">{entry.size_human}</span>
                    {/if}
                  </div>
                {/each}
              </div>
            {:else if isSingle && entries[0]?.is_dir}
              <div class="rounded-md border border-border bg-muted/30 px-3 py-2.5">
                <p class="text-xs text-muted-foreground">
                  目录 <span class="font-mono font-medium text-foreground">{entries[0].path}</span>
                  及其所有内容将被永久删除。
                </p>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-2 border-t border-border px-6 py-4">
        {#if failedEntries.length > 0}
          <!-- After failure: just close -->
          <button
            onclick={() => onOpenChange(false)}
            class={cn(
              'h-9 rounded-md px-4 text-sm font-medium transition-colors',
              'bg-primary text-primary-foreground hover:bg-primary/90',
            )}
          >
            关闭
          </button>
        {:else if deleting}
          <!-- While deleting: disabled state -->
          <button
            disabled
            class={cn(
              'flex h-9 items-center gap-2 rounded-md px-4 text-sm font-medium',
              'bg-destructive/50 text-destructive-foreground cursor-not-allowed opacity-70',
            )}
          >
            <Loader2 size={14} class="animate-spin" />
            删除中…
          </button>
        {:else}
          <!-- Confirmation buttons -->
          <Dialog.Close
            class={cn(
              'h-9 rounded-md px-4 text-sm font-medium transition-colors',
              'border border-border bg-background text-foreground',
              'hover:bg-accent hover:text-accent-foreground',
            )}
          >
            取消
          </Dialog.Close>

          <button
            onclick={handleDelete}
            disabled={entries.length === 0}
            class={cn(
              'flex h-9 items-center gap-2 rounded-md px-4 text-sm font-medium transition-colors',
              'bg-destructive text-destructive-foreground hover:bg-destructive/90',
              'disabled:pointer-events-none disabled:opacity-50',
            )}
          >
            <Trash2 size={14} />
            {isSingle ? '确认删除' : `删除 ${entries.length} 个项目`}
          </button>
        {/if}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
