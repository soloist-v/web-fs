<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { Pencil, X, Loader2 } from 'lucide-svelte';
  import { renameEntry } from '$lib/api';
  import { cn } from '$lib/utils';
  import type { FileEntry } from '$lib/types';

  interface Props {
    open: boolean;
    entry: FileEntry | null;
    onOpenChange: (open: boolean) => void;
    onRenamed: () => void;
  }

  let { open, entry, onOpenChange, onRenamed }: Props = $props();

  let newName = $state('');
  let renaming = $state(false);
  let error = $state<string | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);

  // When the dialog opens (or entry changes), populate the name field and
  // select only the base name (without extension) for quick editing.
  $effect(() => {
    if (open && entry) {
      newName = entry.name;
      error = null;
      renaming = false;

      setTimeout(() => {
        if (!inputEl) return;
        inputEl.focus();

        if (!entry.is_dir && entry.extension && entry.name.endsWith(`.${entry.extension}`)) {
          // Select everything except the trailing ".ext"
          const selEnd = entry.name.length - entry.extension.length - 1;
          inputEl.setSelectionRange(0, selEnd);
        } else {
          inputEl.select();
        }
      }, 50);
    }
  });

  const isValid = $derived(
    newName.trim().length > 0 &&
      !newName.includes('/') &&
      newName.trim() !== '.' &&
      newName.trim() !== '..' &&
      newName.trim() !== entry?.name,
  );

  function buildDestPath(): string {
    if (!entry) return '';
    const parentPath = entry.path.includes('/')
      ? entry.path.substring(0, entry.path.lastIndexOf('/')) || '/'
      : '/';
    const base = parentPath === '/' ? '' : parentPath;
    return `${base}/${newName.trim()}`;
  }

  async function handleRename() {
    if (!isValid || renaming || !entry) return;

    const trimmed = newName.trim();

    if (trimmed.includes('/')) {
      error = '名称不能包含 "/"';
      return;
    }
    if (trimmed === '.' || trimmed === '..') {
      error = '无效的名称';
      return;
    }
    if (trimmed === entry.name) {
      onOpenChange(false);
      return;
    }

    renaming = true;
    error = null;

    try {
      await renameEntry(entry.path, buildDestPath());
      onRenamed();
      onOpenChange(false);
    } catch (err) {
      error = err instanceof Error ? err.message : '重命名失败，请重试';
    } finally {
      renaming = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleRename();
    }
  }

  function handleInput(e: Event) {
    newName = (e.currentTarget as HTMLInputElement).value;
    if (error) error = null;
  }

  // Derive a human-readable entry type label
  const entryTypeLabel = $derived(entry?.is_dir ? '文件夹' : '文件');
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
            class="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 text-primary"
          >
            <Pencil size={18} />
          </div>
          <div>
            <Dialog.Title class="text-base font-semibold text-foreground">
              重命名{entryTypeLabel}
            </Dialog.Title>
            {#if entry}
              <Dialog.Description class="mt-0.5 text-xs text-muted-foreground">
                当前名称：
                <span class="font-mono rounded bg-muted px-1 py-0.5 text-xs">
                  {entry.name}
                </span>
              </Dialog.Description>
            {/if}
          </div>
        </div>

        <Dialog.Close
          class={cn(
            'flex h-7 w-7 items-center justify-center rounded-md transition-colors',
            'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
          )}
        >
          <X size={15} />
        </Dialog.Close>
      </div>

      <!-- Body -->
      <div class="px-6 py-5">
        <div class="flex flex-col gap-1.5">
          <label for="rename-input" class="text-sm font-medium text-foreground">
            新名称
          </label>

          <input
            id="rename-input"
            bind:this={inputEl}
            type="text"
            value={newName}
            oninput={handleInput}
            onkeydown={handleKeydown}
            placeholder="输入新名称…"
            autocomplete="off"
            spellcheck={false}
            disabled={renaming}
            class={cn(
              'h-10 w-full rounded-md border px-3 text-sm outline-none transition-colors',
              'bg-background placeholder:text-muted-foreground',
              'focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1',
              'focus-visible:ring-offset-background',
              error
                ? 'border-destructive focus-visible:ring-destructive'
                : 'border-border hover:border-ring/50',
              renaming && 'cursor-not-allowed opacity-60',
            )}
          />

          {#if error}
            <p class="flex items-center gap-1.5 text-xs text-destructive">
              <span class="inline-block h-1.5 w-1.5 shrink-0 rounded-full bg-destructive"></span>
              {error}
            </p>
          {:else if entry && !entry.is_dir && entry.extension}
            <p class="text-xs text-muted-foreground">
              扩展名 <span class="font-mono">.{entry.extension}</span> 已预先保留，可直接修改主文件名
            </p>
          {:else}
            <p class="text-xs text-muted-foreground">不能包含 "/" 字符</p>
          {/if}
        </div>

        <!-- Preview of the resulting path -->
        {#if entry && newName.trim() && newName.trim() !== entry.name}
          {@const parent =
            entry.path.includes('/')
              ? entry.path.substring(0, entry.path.lastIndexOf('/')) || '/'
              : '/'}
          <div class="mt-4 rounded-md border border-border bg-muted/40 px-3 py-2.5">
            <p class="mb-1 text-xs font-medium text-muted-foreground">重命名后的路径</p>
            <p class="truncate font-mono text-xs text-foreground">
              {parent === '/' ? '' : parent}/{newName.trim()}
            </p>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-2 border-t border-border px-6 py-4">
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
          onclick={handleRename}
          disabled={!isValid || renaming}
          class={cn(
            'flex h-9 items-center gap-2 rounded-md px-4 text-sm font-medium transition-colors',
            'bg-primary text-primary-foreground hover:bg-primary/90',
            'disabled:pointer-events-none disabled:opacity-50',
          )}
        >
          {#if renaming}
            <Loader2 size={14} class="animate-spin" />
            重命名中…
          {:else}
            <Pencil size={14} />
            重命名
          {/if}
        </button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
