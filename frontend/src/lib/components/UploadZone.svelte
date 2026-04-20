<script lang="ts">
  import { Upload, X, FileUp, CheckCircle, AlertCircle, Loader2 } from 'lucide-svelte';
  import { uploadFiles } from '$lib/api';
  import { cn } from '$lib/utils';
  import type { UploadProgress, UploadResult } from '$lib/types';

  interface Props {
    path: string;
    show: boolean;
    onupload: () => void;
    onclose?: () => void;
  }

  let { path, show, onupload, onclose }: Props = $props();

  type UploadState = 'idle' | 'uploading' | 'done' | 'error';

  interface FileStatus {
    file: File;
    state: UploadState;
    progress: number;
    error?: string;
    resultPath?: string;
  }

  let dragOver = $state(false);
  let uploading = $state(false);
  let fileStatuses = $state<FileStatus[]>([]);
  let dragCounter = $state(0);

  const allDone = $derived(
    fileStatuses.length > 0 && fileStatuses.every((f) => f.state === 'done' || f.state === 'error'),
  );

  const hasErrors = $derived(fileStatuses.some((f) => f.state === 'error'));

  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    dragCounter++;
    dragOver = true;
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    dragCounter--;
    if (dragCounter <= 0) {
      dragCounter = 0;
      dragOver = false;
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy';
    }
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    dragCounter = 0;

    const files = e.dataTransfer?.files;
    if (!files || files.length === 0) return;

    await startUpload(Array.from(files));
  }

  function handleFileInput(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = input.files;
    if (!files || files.length === 0) return;
    startUpload(Array.from(files));
    input.value = '';
  }

  async function startUpload(files: File[]) {
    if (uploading) return;

    uploading = true;
    fileStatuses = files.map((file) => ({
      file,
      state: 'idle',
      progress: 0,
    }));

    // Mark all as uploading
    fileStatuses = fileStatuses.map((f) => ({ ...f, state: 'uploading' as UploadState }));

    try {
      await uploadFiles(path, files, (progress: UploadProgress) => {
        fileStatuses = fileStatuses.map((f, i) => {
          if (i === progress.fileIndex) {
            return { ...f, progress: progress.percent, state: 'uploading' as UploadState };
          }
          return f;
        });
      });

      // Mark all as done (uploadFiles resolves after all complete)
      fileStatuses = fileStatuses.map((f) => ({ ...f, state: 'done' as UploadState, progress: 100 }));
      onupload();
    } catch (err) {
      fileStatuses = fileStatuses.map((f) => {
        if (f.state === 'uploading') {
          return {
            ...f,
            state: 'error' as UploadState,
            error: err instanceof Error ? err.message : '上传失败',
          };
        }
        return f;
      });
    } finally {
      uploading = false;
    }
  }

  function handleClose() {
    if (uploading) return;
    fileStatuses = [];
    dragCounter = 0;
    dragOver = false;
    onclose?.();
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
</script>

{#if show}
  <!-- Backdrop overlay -->
  <div
    class={cn(
      'fixed inset-0 z-50 flex items-center justify-center transition-colors duration-200',
      dragOver ? 'bg-primary/10' : 'bg-black/60',
    )}
    ondragenter={handleDragEnter}
    ondragleave={handleDragLeave}
    ondragover={handleDragOver}
    ondrop={handleDrop}
    role="presentation"
  >
    <!-- Drop zone panel -->
    <div
      class={cn(
        'relative mx-4 w-full max-w-lg rounded-xl border-2 transition-all duration-200',
        'bg-card shadow-2xl',
        dragOver
          ? 'border-primary scale-[1.02] bg-primary/5'
          : 'border-dashed border-border',
        fileStatuses.length > 0 ? 'border-solid' : '',
      )}
    >
      <!-- Close button -->
      {#if !uploading}
        <button
          onclick={handleClose}
          class="absolute right-3 top-3 flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
          title="关闭"
        >
          <X size={15} />
        </button>
      {/if}

      <div class="p-8">
        {#if fileStatuses.length === 0}
          <!-- Idle state -->
          <div class="flex flex-col items-center gap-4 text-center">
            <div
              class={cn(
                'flex h-20 w-20 items-center justify-center rounded-full transition-colors',
                dragOver ? 'bg-primary/20 text-primary' : 'bg-muted text-muted-foreground',
              )}
            >
              <Upload size={36} class={cn(dragOver && 'animate-bounce')} />
            </div>

            <div>
              <p class="text-lg font-semibold text-foreground">
                {dragOver ? '松开以上传' : '拖拽文件到这里'}
              </p>
              <p class="mt-1 text-sm text-muted-foreground">
                上传到：<span class="font-mono text-xs bg-muted rounded px-1.5 py-0.5">{path}</span>
              </p>
            </div>

            <div class="flex items-center gap-3 text-sm text-muted-foreground">
              <div class="h-px flex-1 bg-border"></div>
              <span>或</span>
              <div class="h-px flex-1 bg-border"></div>
            </div>

            <!-- File picker button -->
            <label
              class={cn(
                'flex cursor-pointer items-center gap-2 rounded-lg px-5 py-2.5 text-sm font-medium',
                'bg-primary text-primary-foreground hover:bg-primary/90 transition-colors',
              )}
            >
              <FileUp size={15} />
              选择文件
              <input
                type="file"
                multiple
                class="sr-only"
                oninput={handleFileInput}
              />
            </label>

            <p class="text-xs text-muted-foreground">支持多文件同时上传</p>
          </div>
        {:else}
          <!-- Upload progress list -->
          <div class="flex flex-col gap-3">
            <div class="flex items-center justify-between">
              <h3 class="font-semibold text-foreground">
                {#if uploading}
                  上传中…
                {:else if hasErrors}
                  上传完成（有错误）
                {:else}
                  上传完成
                {/if}
              </h3>
              <span class="text-xs text-muted-foreground">
                {fileStatuses.filter((f) => f.state === 'done').length} / {fileStatuses.length} 个文件
              </span>
            </div>

            <!-- File list -->
            <div class="flex max-h-60 flex-col gap-2 overflow-y-auto pr-1">
              {#each fileStatuses as status}
                <div
                  class={cn(
                    'flex flex-col gap-1.5 rounded-lg border p-3 transition-colors',
                    status.state === 'done' && 'border-green-500/30 bg-green-500/5',
                    status.state === 'error' && 'border-destructive/30 bg-destructive/5',
                    status.state === 'uploading' && 'border-border bg-muted/30',
                    status.state === 'idle' && 'border-border',
                  )}
                >
                  <div class="flex items-center gap-2">
                    <!-- Status icon -->
                    <div class="shrink-0">
                      {#if status.state === 'uploading'}
                        <Loader2 size={15} class="animate-spin text-primary" />
                      {:else if status.state === 'done'}
                        <CheckCircle size={15} class="text-green-500" />
                      {:else if status.state === 'error'}
                        <AlertCircle size={15} class="text-destructive" />
                      {:else}
                        <FileUp size={15} class="text-muted-foreground" />
                      {/if}
                    </div>

                    <!-- File name + size -->
                    <div class="flex flex-1 items-center justify-between gap-2 min-w-0">
                      <span class="truncate text-sm font-medium text-foreground">
                        {status.file.name}
                      </span>
                      <span class="shrink-0 text-xs text-muted-foreground">
                        {formatSize(status.file.size)}
                      </span>
                    </div>
                  </div>

                  <!-- Progress bar -->
                  {#if status.state === 'uploading'}
                    <div class="h-1 w-full overflow-hidden rounded-full bg-muted">
                      <div
                        class="h-full rounded-full bg-primary transition-all duration-300"
                        style="width: {status.progress}%"
                      ></div>
                    </div>
                    <span class="text-xs text-muted-foreground">{status.progress}%</span>
                  {/if}

                  <!-- Error message -->
                  {#if status.state === 'error' && status.error}
                    <p class="text-xs text-destructive">{status.error}</p>
                  {/if}
                </div>
              {/each}
            </div>

            <!-- Done actions -->
            {#if allDone}
              <div class="flex justify-end gap-2 pt-1">
                <button
                  onclick={() => {
                    fileStatuses = [];
                  }}
                  class="rounded-md px-3 py-1.5 text-sm text-muted-foreground hover:bg-accent transition-colors"
                >
                  继续上传
                </button>
                <button
                  onclick={handleClose}
                  class="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
                >
                  完成
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
