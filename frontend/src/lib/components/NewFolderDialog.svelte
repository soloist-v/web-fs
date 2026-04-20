<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { FolderPlus, X, Loader2 } from 'lucide-svelte';
  import { createDir } from '$lib/api';
  import { cn } from '$lib/utils';

  interface Props {
    open: boolean;
    currentPath: string;
    onOpenChange: (open: boolean) => void;
    onCreated: () => void;
  }

  let { open, currentPath, onOpenChange, onCreated }: Props = $props();

  let folderName = $state('');
  let creating = $state(false);
  let error = $state<string | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (open) {
      folderName = '';
      error = null;
      creating = false;
      // Focus input after dialog opens
      setTimeout(() => inputEl?.focus(), 50);
    }
  });

  function buildPath(): string {
    const base = currentPath === '/' ? '' : currentPath;
    return `${base}/${folderName.trim()}`;
  }

  const isValid = $derived(folderName.trim().length > 0 && !folderName.includes('/'));

  async function handleCreate() {
    if (!isValid || creating) return;

    const name = folderName.trim();
    if (!name) return;

    // Validate folder name
    if (name.includes('/')) {
      error = '文件夹名称不能包含 "/"';
      return;
    }
    if (name === '.' || name === '..') {
      error = '无效的文件夹名称';
      return;
    }

    creating = true;
    error = null;

    try {
      await createDir(buildPath());
      onCreated();
      onOpenChange(false);
    } catch (err) {
      error = err instanceof Error ? err.message : '创建失败，请重试';
    } finally {
      creating = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleCreate();
    }
  }

  function handleInput(e: Event) {
    const val = (e.currentTarget as HTMLInputElement).value;
    folderName = val;
    if (error) error = null;
  }
</script>

<Dialog.Root
  open={open}
  onOpenChange={onOpenChange}
>
  <Dialog.Portal>
    <Dialog.Overlay
      class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
    />
    <Dialog.Content
      class={cn(
        'fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2',
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
          <div class="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <FolderPlus size={18} />
          </div>
          <div>
            <Dialog.Title class="text-base font-semibold text-foreground">
              新建文件夹
            </Dialog.Title>
            <Dialog.Description class="text-xs text-muted-foreground mt-0.5">
              在
              <span class="font-mono text-xs bg-muted rounded px-1 py-0.5">
                {currentPath}
              </span>
              中创建
            </Dialog.Description>
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
          <label for="folder-name" class="text-sm font-medium text-foreground">
            文件夹名称
          </label>
          <input
            id="folder-name"
            bind:this={inputEl}
            type="text"
            value={folderName}
            oninput={handleInput}
            onkeydown={handleKeydown}
            placeholder="例如：新建文件夹"
            autocomplete="off"
            spellcheck={false}
            disabled={creating}
            class={cn(
              'h-10 w-full rounded-md border px-3 text-sm outline-none transition-colors',
              'bg-background placeholder:text-muted-foreground',
              'focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background',
              error
                ? 'border-destructive focus-visible:ring-destructive'
                : 'border-border hover:border-ring/50',
              creating && 'opacity-60 cursor-not-allowed',
            )}
          />

          {#if error}
            <p class="flex items-center gap-1.5 text-xs text-destructive">
              <span class="inline-block h-1.5 w-1.5 rounded-full bg-destructive"></span>
              {error}
            </p>
          {:else}
            <p class="text-xs text-muted-foreground">
              不能包含 "/" 字符
            </p>
          {/if}
        </div>
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
          onclick={handleCreate}
          disabled={!isValid || creating}
          class={cn(
            'flex h-9 items-center gap-2 rounded-md px-4 text-sm font-medium transition-colors',
            'bg-primary text-primary-foreground hover:bg-primary/90',
            'disabled:pointer-events-none disabled:opacity-50',
          )}
        >
          {#if creating}
            <Loader2 size={14} class="animate-spin" />
            创建中…
          {:else}
            <FolderPlus size={14} />
            创建
          {/if}
        </button>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
