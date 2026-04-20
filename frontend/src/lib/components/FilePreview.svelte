<script lang="ts">
    import { Dialog } from "bits-ui";
    import {
        X,
        Download,
        Folder,
        FileText,
        Image as ImageIcon,
        Video,
        Music,
        File,
        Archive,
        Loader2,
        AlertCircle,
        ExternalLink,
    } from "lucide-svelte";
    import { cn } from "$lib/utils";
    import type { FileEntry, PreviewType } from "$lib/types";
    import { fetchFileBlob, getFileContent, downloadFile } from "$lib/api";

    interface Props {
        entry: FileEntry | null;
        onclose: () => void;
    }

    let { entry, onclose }: Props = $props();

    const open = $derived(entry !== null);

    // ── Preview state ────────────────────────────────────────────────
    let textContent = $state<string | null>(null);
    let objectUrl = $state<string | null>(null);
    let loading = $state(false);
    let error = $state<string | null>(null);

    // ── Preview type ─────────────────────────────────────────────────
    const previewType = $derived(
        entry ? detectPreviewType(entry) : "unsupported",
    );

    function detectPreviewType(e: FileEntry): PreviewType {
        if (e.is_dir) return "directory";

        const mime = e.mime_type ?? "";
        const ext = (e.extension ?? "").toLowerCase();

        if (mime.startsWith("image/")) return "image";
        if (mime.startsWith("video/")) return "video";
        if (mime.startsWith("audio/")) return "audio";
        if (mime === "application/pdf") return "pdf";

        const textExts = new Set([
            "txt",
            "md",
            "markdown",
            "rst",
            "rs",
            "toml",
            "lock",
            "ts",
            "tsx",
            "js",
            "jsx",
            "mjs",
            "cjs",
            "py",
            "pyi",
            "rb",
            "go",
            "c",
            "h",
            "cpp",
            "cc",
            "cxx",
            "hpp",
            "java",
            "kt",
            "kts",
            "swift",
            "yaml",
            "yml",
            "json",
            "jsonc",
            "json5",
            "xml",
            "svg",
            "html",
            "htm",
            "css",
            "scss",
            "sass",
            "less",
            "sh",
            "bash",
            "zsh",
            "fish",
            "ps1",
            "vue",
            "svelte",
            "sql",
            "graphql",
            "gql",
            "php",
            "lua",
            "r",
            "jl",
            "env",
            "gitignore",
            "dockerignore",
            "editorconfig",
            "ini",
            "cfg",
            "conf",
            "log",
            "diff",
            "patch",
            "csv",
            "tsv",
            "makefile",
            "dockerfile",
        ]);

        if (mime.startsWith("text/") || textExts.has(ext)) return "text";

        const archiveExts = new Set([
            "zip",
            "tar",
            "gz",
            "bz2",
            "xz",
            "7z",
            "rar",
            "zst",
            "tgz",
        ]);
        if (archiveExts.has(ext)) return "archive";

        return "unsupported";
    }

    // ── Load content when entry changes ─────────────────────────────
    $effect(() => {
        if (!entry) return;

        const path = entry.path;
        const type = previewType;

        textContent = null;
        error = null;

        if (
            type === "directory" ||
            type === "archive" ||
            type === "unsupported" ||
            type === "pdf"
        ) {
            loading = false;
            return;
        }

        let cancelled = false;
        let blobUrl: string | null = null;
        loading = true;

        (async () => {
            try {
                if (type === "image" || type === "video" || type === "audio") {
                    const blob = await fetchFileBlob(path);
                    if (cancelled) return;
                    blobUrl = URL.createObjectURL(blob);
                    objectUrl = blobUrl;
                } else if (type === "text") {
                    const content = await getFileContent(path);
                    if (cancelled) return;
                    textContent = content;
                }
            } catch (e) {
                if (!cancelled)
                    error = e instanceof Error ? e.message : "加载预览失败";
            } finally {
                if (!cancelled) loading = false;
            }
        })();

        return () => {
            cancelled = true;
            if (blobUrl) URL.revokeObjectURL(blobUrl);
            objectUrl = null;
        };
    });

    function handleDownload() {
        if (entry) downloadFile(entry.path);
    }

    const downloadUrl = $derived(
        entry ? `/api/download?path=${encodeURIComponent(entry.path)}` : "#",
    );

    function formatDate(iso: string | null): string {
        if (!iso) return "—";
        return new Date(iso).toLocaleString("zh-CN", {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
        });
    }

    function formatSize(bytes: number): string {
        if (bytes === 0) return "0 B";
        const units = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.min(
            Math.floor(Math.log(bytes) / Math.log(1024)),
            units.length - 1,
        );
        return `${(bytes / 1024 ** i).toFixed(i === 0 ? 0 : 2)} ${units[i]}`;
    }

    function handleOpenChange(v: boolean) {
        if (!v) onclose();
    }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
    <Dialog.Portal>
        <!-- Backdrop -->
        <Dialog.Overlay
            class="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm
                   data-[state=open]:animate-in data-[state=closed]:animate-out
                   data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
        />

        <!-- Dialog panel -->
        <Dialog.Content
            class={cn(
                "fixed left-1/2 top-1/2 z-50 -translate-x-1/2 -translate-y-1/2",
                "flex w-[92vw] max-w-3xl flex-col overflow-hidden",
                "rounded-xl border border-border bg-card shadow-2xl",
                "max-h-[88vh]",
                "data-[state=open]:animate-in data-[state=closed]:animate-out",
                "data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
                "data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
                "data-[state=closed]:slide-out-to-left-1/2 data-[state=open]:slide-in-from-left-1/2",
                "data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-top-[48%]",
            )}
        >
            {#if entry}
                <!-- ── Header ── -->
                <div
                    class="flex shrink-0 items-start gap-3 border-b border-border px-5 py-4"
                >
                    <!-- Type icon -->
                    <div class="mt-0.5 shrink-0">
                        {#if entry.is_dir}
                            <Folder class="h-8 w-8 text-yellow-400" />
                        {:else if previewType === "image"}
                            <ImageIcon class="h-8 w-8 text-purple-400" />
                        {:else if previewType === "video"}
                            <Video class="h-8 w-8 text-blue-400" />
                        {:else if previewType === "audio"}
                            <Music class="h-8 w-8 text-green-400" />
                        {:else if previewType === "pdf"}
                            <FileText class="h-8 w-8 text-red-400" />
                        {:else if previewType === "text"}
                            <FileText class="h-8 w-8 text-sky-400" />
                        {:else if previewType === "archive"}
                            <Archive class="h-8 w-8 text-orange-400" />
                        {:else}
                            <File class="h-8 w-8 text-muted-foreground" />
                        {/if}
                    </div>

                    <!-- Name + mime -->
                    <div class="min-w-0 flex-1">
                        <Dialog.Title
                            class="break-all text-sm font-semibold leading-snug text-foreground"
                            title={entry.name}
                        >
                            {entry.name}
                        </Dialog.Title>
                        <Dialog.Description
                            class="mt-0.5 text-xs text-muted-foreground"
                        >
                            {entry.is_dir
                                ? "文件夹"
                                : (entry.mime_type ??
                                  entry.extension?.toUpperCase() ??
                                  "未知类型")}
                            {#if !entry.is_dir}
                                · {entry.size_human}
                            {/if}
                        </Dialog.Description>
                    </div>

                    <!-- Close -->
                    <Dialog.Close
                        class="ml-1 shrink-0 rounded-md p-1.5 text-muted-foreground transition-colors
                               hover:bg-accent hover:text-accent-foreground
                               focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                        aria-label="关闭预览"
                    >
                        <X class="h-4 w-4" />
                    </Dialog.Close>
                </div>

                <!-- ── Preview body ── -->
                <div class="relative min-h-0 flex-1 overflow-hidden">
                    {#if loading}
                        <div
                            class="flex h-full min-h-48 items-center justify-center"
                        >
                            <Loader2
                                class="h-8 w-8 animate-spin text-muted-foreground"
                            />
                        </div>
                    {:else if error}
                        <div
                            class="flex h-full min-h-48 flex-col items-center justify-center gap-3 px-6 text-center"
                        >
                            <AlertCircle
                                class="h-10 w-10 text-destructive/70"
                            />
                            <p class="text-sm text-muted-foreground">{error}</p>
                            <button
                                class="inline-flex h-8 items-center gap-2 rounded-md border border-border px-3 text-xs font-medium transition-colors hover:bg-accent"
                                onclick={handleDownload}
                            >
                                <Download class="h-3.5 w-3.5" />
                                下载文件
                            </button>
                        </div>
                    {:else if previewType === "image" && objectUrl}
                        <div
                            class="flex h-full min-h-48 items-center justify-center overflow-auto bg-muted/20 p-4"
                        >
                            <img
                                src={objectUrl}
                                alt={entry.name}
                                class="max-h-full max-w-full rounded object-contain shadow"
                            />
                        </div>
                    {:else if previewType === "text" && textContent !== null}
                        <div class="h-full overflow-auto">
                            <pre
                                class="min-h-16 whitespace-pre-wrap break-words p-5 font-mono text-xs leading-relaxed text-foreground/90">{textContent}</pre>
                        </div>
                    {:else if previewType === "video" && objectUrl}
                        <div
                            class="flex h-full min-h-48 items-center justify-center bg-black/90 p-4"
                        >
                            <!-- svelte-ignore a11y_media_has_caption -->
                            <video
                                src={objectUrl}
                                controls
                                class="max-h-full max-w-full rounded shadow"
                            ></video>
                        </div>
                    {:else if previewType === "audio" && objectUrl}
                        <div
                            class="flex h-full min-h-36 flex-col items-center justify-center gap-5 p-6"
                        >
                            <Music class="h-16 w-16 text-muted-foreground/20" />
                            <!-- svelte-ignore a11y_media_has_caption -->
                            <audio
                                src={objectUrl}
                                controls
                                class="w-full max-w-sm"
                            ></audio>
                        </div>
                    {:else if previewType === "pdf"}
                        <div
                            class="flex h-full min-h-48 flex-col items-center justify-center gap-4 p-6 text-center"
                        >
                            <FileText class="h-14 w-14 text-red-400/40" />
                            <p class="text-sm text-muted-foreground">
                                PDF 文档无法内嵌预览
                            </p>
                            <a
                                href={downloadUrl}
                                target="_blank"
                                rel="noreferrer noopener"
                                class="inline-flex h-8 items-center gap-2 rounded-md border border-border px-3 text-xs font-medium transition-colors hover:bg-accent"
                            >
                                <ExternalLink class="h-3.5 w-3.5" />
                                在新标签页打开
                            </a>
                        </div>
                    {:else if previewType === "directory"}
                        <div
                            class="flex h-full min-h-36 flex-col items-center justify-center gap-3 p-6"
                        >
                            <Folder class="h-14 w-14 text-yellow-400/40" />
                            <p class="text-sm text-muted-foreground">文件夹</p>
                        </div>
                    {:else}
                        <div
                            class="flex h-full min-h-48 flex-col items-center justify-center gap-3 p-6 text-center"
                        >
                            {#if previewType === "archive"}
                                <Archive class="h-14 w-14 text-orange-400/40" />
                                <p class="text-sm text-muted-foreground">
                                    压缩文件 · 暂不支持预览
                                </p>
                            {:else}
                                <File
                                    class="h-14 w-14 text-muted-foreground/20"
                                />
                                <p class="text-sm text-muted-foreground">
                                    暂不支持预览此文件类型
                                </p>
                            {/if}
                            <button
                                class="inline-flex h-8 items-center gap-2 rounded-md border border-border px-3 text-xs font-medium transition-colors hover:bg-accent"
                                onclick={handleDownload}
                            >
                                <Download class="h-3.5 w-3.5" />
                                下载文件
                            </button>
                        </div>
                    {/if}
                </div>

                <!-- ── Footer: metadata + download ── -->
                {#if !entry.is_dir}
                    <div
                        class="shrink-0 border-t border-border bg-muted/20 px-5 py-3"
                    >
                        <dl class="flex flex-wrap gap-x-6 gap-y-1 text-xs">
                            <div class="flex items-center gap-1.5">
                                <dt class="text-muted-foreground">大小</dt>
                                <dd class="font-medium text-foreground">
                                    {entry.size_human}
                                </dd>
                            </div>

                            {#if entry.modified}
                                <div class="flex items-center gap-1.5">
                                    <dt class="text-muted-foreground">
                                        修改时间
                                    </dt>
                                    <dd class="font-medium text-foreground">
                                        {formatDate(entry.modified)}
                                    </dd>
                                </div>
                            {/if}

                            {#if entry.permissions}
                                <div class="flex items-center gap-1.5">
                                    <dt class="text-muted-foreground">权限</dt>
                                    <dd
                                        class="font-mono font-medium text-foreground"
                                    >
                                        {entry.permissions}
                                    </dd>
                                </div>
                            {/if}

                            {#if entry.mime_type}
                                <div class="flex items-center gap-1.5">
                                    <dt class="text-muted-foreground">MIME</dt>
                                    <dd
                                        class="max-w-[200px] truncate font-medium text-foreground"
                                        title={entry.mime_type}
                                    >
                                        {entry.mime_type}
                                    </dd>
                                </div>
                            {/if}
                        </dl>

                        <button
                            class="mt-2.5 inline-flex w-full items-center justify-center gap-2 rounded-md
                                   bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground
                                   transition-colors hover:bg-primary/90
                                   focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                            onclick={handleDownload}
                        >
                            <Download class="h-3.5 w-3.5" />
                            下载
                        </button>
                    </div>
                {/if}
            {/if}
        </Dialog.Content>
    </Dialog.Portal>
</Dialog.Root>
