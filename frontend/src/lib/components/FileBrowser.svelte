<script lang="ts">
    import { ContextMenu } from "bits-ui";
    import {
        Folder,
        FolderOpen,
        File,
        FileText,
        FileCode,
        FileArchive,
        Image,
        Film,
        Music,
        Terminal,
        ArrowUp,
        ArrowDown,
        ChevronsUpDown,
        Download,
        Pencil,
        Scissors,
        Copy,
        Clipboard,
        Trash2,
        AlertCircle,
        Search,
    } from "lucide-svelte";
    import { cn } from "$lib/utils";
    import { downloadFile } from "$lib/api";
    import type { FileEntry, ViewMode, SortField, SortOrder } from "$lib/types";

    // ── Props ──────────────────────────────────────────────────────────────────

    interface Props {
        entries: FileEntry[];
        loading: boolean;
        error: string | null;
        viewMode: ViewMode;
        sortField: SortField;
        sortOrder: SortOrder;
        selectedPaths: Set<string>;
        searchResults: FileEntry[] | null;
        onNavigate: (path: string) => void;
        onSort: (field: SortField) => void;
        onSelect: (paths: Set<string>) => void;
        onPreview: (entry: FileEntry) => void;
        onRename: (entry: FileEntry) => void;
        onDelete: (entries: FileEntry[]) => void;
        onDownload: (entry: FileEntry) => void;
        onCopy: (paths: string[]) => void;
        onCut: (paths: string[]) => void;
        onPaste: () => void;
        clipboard: { paths: string[]; operation: "copy" | "cut" } | null;
    }

    let {
        entries,
        loading,
        error,
        viewMode,
        sortField,
        sortOrder,
        selectedPaths,
        searchResults,
        onNavigate,
        onSort,
        onSelect,
        onPreview,
        onRename,
        onDelete,
        onDownload,
        onCopy,
        onCut,
        onPaste,
        clipboard,
    }: Props = $props();

    // ── Derived state ──────────────────────────────────────────────────────────

    let displayEntries = $derived(searchResults ?? entries);
    let isSearchMode = $derived(searchResults !== null);

    let allSelected = $derived(
        displayEntries.length > 0 &&
            displayEntries.every((e) => selectedPaths.has(e.path)),
    );
    let someSelected = $derived(
        displayEntries.some((e) => selectedPaths.has(e.path)) && !allSelected,
    );
    let hasPermissions = $derived(
        displayEntries.some((e) => e.permissions != null),
    );

    // ── Local state ────────────────────────────────────────────────────────────

    let lastClickedIndex = $state(-1);
    let headerCbEl = $state<HTMLInputElement | null>(null);

    $effect(() => {
        if (headerCbEl) {
            headerCbEl.indeterminate = someSelected;
        }
    });

    // ── Selection helpers ──────────────────────────────────────────────────────

    function handleSelectAll(checked: boolean) {
        if (checked) {
            onSelect(new Set(displayEntries.map((e) => e.path)));
        } else {
            onSelect(new Set());
        }
    }

    function handleRowClick(e: MouseEvent, entry: FileEntry, index: number) {
        if (e.shiftKey && lastClickedIndex >= 0) {
            const start = Math.min(lastClickedIndex, index);
            const end = Math.max(lastClickedIndex, index);
            const next = new Set(selectedPaths);
            for (let i = start; i <= end; i++) {
                next.add(displayEntries[i].path);
            }
            onSelect(next);
        } else if (e.ctrlKey || e.metaKey) {
            const next = new Set(selectedPaths);
            if (next.has(entry.path)) {
                next.delete(entry.path);
            } else {
                next.add(entry.path);
            }
            onSelect(next);
            lastClickedIndex = index;
        } else {
            onSelect(new Set([entry.path]));
            lastClickedIndex = index;
        }
    }

    function handleRowDoubleClick(entry: FileEntry) {
        if (entry.is_dir) {
            onNavigate(entry.path);
        } else {
            onPreview(entry);
        }
    }

    /** Ensure the right-clicked entry is selected before showing the context menu. */
    function handleContextMenuOpen(entry: FileEntry) {
        if (!selectedPaths.has(entry.path)) {
            onSelect(new Set([entry.path]));
        }
    }

    /** Returns all selected entries if the target is part of the selection, otherwise just the target. */
    function getContextTargets(entry: FileEntry): FileEntry[] {
        if (selectedPaths.has(entry.path) && selectedPaths.size > 1) {
            return displayEntries.filter((e) => selectedPaths.has(e.path));
        }
        return [entry];
    }

    // ── Keyboard shortcuts ─────────────────────────────────────────────────────

    function handleKeydown(e: KeyboardEvent) {
        const target = e.target as HTMLElement;
        if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;

        if (e.key === "Delete" || (e.key === "Backspace" && e.metaKey)) {
            if (selectedPaths.size > 0) {
                const targets = displayEntries.filter((en) =>
                    selectedPaths.has(en.path),
                );
                if (targets.length > 0) onDelete(targets);
            }
        } else if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            handleSelectAll(true);
        } else if (e.key === "Escape") {
            if (selectedPaths.size > 0) onSelect(new Set());
        } else if (e.key === "F2") {
            if (selectedPaths.size === 1) {
                const found = displayEntries.find((en) =>
                    selectedPaths.has(en.path),
                );
                if (found) onRename(found);
            }
        } else if (e.key === "c" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            if (selectedPaths.size > 0) onCopy(Array.from(selectedPaths));
        } else if (e.key === "x" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            if (selectedPaths.size > 0) onCut(Array.from(selectedPaths));
        } else if (e.key === "v" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            if (clipboard) onPaste();
        }
    }

    // ── Formatting helpers ─────────────────────────────────────────────────────

    function formatDate(iso: string | null): string {
        if (!iso) return "—";
        try {
            const d = new Date(iso);
            const y = d.getFullYear();
            const mo = String(d.getMonth() + 1).padStart(2, "0");
            const day = String(d.getDate()).padStart(2, "0");
            const h = String(d.getHours()).padStart(2, "0");
            const m = String(d.getMinutes()).padStart(2, "0");
            return `${y}-${mo}-${day} ${h}:${m}`;
        } catch {
            return iso;
        }
    }

    // ── Icon helper ────────────────────────────────────────────────────────────

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    type AnyIcon = any;

    interface IconInfo {
        icon: AnyIcon;
        colorClass: string;
    }

    function getIconInfo(entry: FileEntry): IconInfo {
        if (entry.is_dir)
            return { icon: Folder, colorClass: "text-yellow-400" };

        const ext = (entry.extension ?? "").toLowerCase();

        if (
            [
                "jpg",
                "jpeg",
                "png",
                "gif",
                "webp",
                "svg",
                "bmp",
                "ico",
                "avif",
            ].includes(ext)
        )
            return { icon: Image, colorClass: "text-purple-400" };

        if (
            ["mp4", "mkv", "avi", "mov", "webm", "ogv", "flv", "m4v"].includes(
                ext,
            )
        )
            return { icon: Film, colorClass: "text-blue-400" };

        if (
            ["mp3", "flac", "wav", "ogg", "aac", "m4a", "opus", "wma"].includes(
                ext,
            )
        )
            return { icon: Music, colorClass: "text-green-400" };

        if (ext === "pdf")
            return { icon: FileText, colorClass: "text-red-400" };

        if (
            [
                "zip",
                "tar",
                "gz",
                "bz2",
                "7z",
                "xz",
                "rar",
                "zst",
                "tgz",
            ].includes(ext)
        )
            return { icon: FileArchive, colorClass: "text-orange-400" };

        if (
            [
                "rs",
                "ts",
                "tsx",
                "js",
                "jsx",
                "py",
                "go",
                "java",
                "c",
                "cpp",
                "h",
                "hpp",
                "cs",
                "rb",
                "php",
                "swift",
                "kt",
                "dart",
                "scala",
                "lua",
                "sh",
                "bash",
                "zsh",
                "fish",
                "ps1",
                "sql",
                "html",
                "htm",
                "css",
                "scss",
                "less",
                "sass",
                "svelte",
                "vue",
                "graphql",
                "proto",
            ].includes(ext)
        )
            return { icon: FileCode, colorClass: "text-cyan-400" };

        if (
            [
                "txt",
                "md",
                "markdown",
                "rst",
                "csv",
                "log",
                "json",
                "jsonc",
                "xml",
                "yaml",
                "yml",
                "toml",
                "ini",
                "env",
                "conf",
                "cfg",
                "lock",
                "gitignore",
                "dockerignore",
                "editorconfig",
            ].includes(ext)
        )
            return { icon: FileText, colorClass: "text-muted-foreground" };

        if (
            !ext ||
            ["exe", "bin", "out", "run", "appimage", "msi", "dmg"].includes(ext)
        )
            return { icon: Terminal, colorClass: "text-green-400" };

        return { icon: File, colorClass: "text-muted-foreground" };
    }

    // ── Shared class strings ───────────────────────────────────────────────────

    const ctxItemCls = cn(
        "flex cursor-default select-none items-center gap-2 rounded-md px-2.5 py-1.5",
        "text-sm outline-none transition-colors",
        "data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground",
        "data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
    );

    const ctxItemDestructiveCls = cn(
        "flex cursor-default select-none items-center gap-2 rounded-md px-2.5 py-1.5",
        "text-sm text-destructive outline-none transition-colors",
        "data-[highlighted]:bg-destructive data-[highlighted]:text-destructive-foreground",
    );

    const ctxContentCls = cn(
        "z-50 min-w-44 overflow-hidden rounded-lg border border-border",
        "bg-popover text-popover-foreground p-1 shadow-xl",
        "data-[state=open]:animate-in data-[state=closed]:animate-out",
        "data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
        "data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
    );

    const SKELETON_COUNT = 5;
</script>

<!-- ── Outer container ─────────────────────────────────────────────────────── -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
    class="flex h-full flex-col overflow-hidden bg-background outline-none"
    tabindex={0}
    onkeydown={handleKeydown}
    role="region"
    aria-label="文件列表"
>
    <!-- ══════════════════════════════════════════════════════════════════════ -->
    <!-- LOADING SKELETON                                                       -->
    <!-- ══════════════════════════════════════════════════════════════════════ -->
    {#if loading}
        <div class="flex-1 overflow-auto">
            <table class="w-full table-fixed border-separate border-spacing-0">
                <thead class="sticky top-0 z-10">
                    <tr class="border-b border-border bg-muted/50">
                        <th class="w-10 p-3"></th>
                        <th
                            class="p-3 text-left text-xs font-medium text-muted-foreground"
                            >名称</th
                        >
                        <th
                            class="w-28 p-3 text-right text-xs font-medium text-muted-foreground"
                            >大小</th
                        >
                        <th
                            class="w-44 p-3 text-left text-xs font-medium text-muted-foreground"
                            >修改时间</th
                        >
                        <th class="w-24 p-3"></th>
                    </tr>
                </thead>
                <tbody>
                    {#each Array.from({ length: SKELETON_COUNT }) as _, i (i)}
                        <tr class="animate-pulse border-b border-border/40">
                            <td class="p-3">
                                <div
                                    class="mx-auto h-4 w-4 rounded bg-muted"
                                ></div>
                            </td>
                            <td class="p-3">
                                <div class="flex items-center gap-2.5">
                                    <div
                                        class="h-5 w-5 shrink-0 rounded bg-muted"
                                    ></div>
                                    <div
                                        class="h-4 rounded bg-muted"
                                        style="width: {38 + ((i * 19) % 38)}%"
                                    ></div>
                                </div>
                            </td>
                            <td class="p-3">
                                <div
                                    class="ml-auto h-4 w-16 rounded bg-muted"
                                ></div>
                            </td>
                            <td class="p-3">
                                <div class="h-4 w-32 rounded bg-muted"></div>
                            </td>
                            <td class="p-3">
                                <div
                                    class="ml-auto h-4 w-16 rounded bg-muted"
                                ></div>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>

        <!-- ══════════════════════════════════════════════════════════════════════ -->
        <!-- ERROR STATE                                                            -->
        <!-- ══════════════════════════════════════════════════════════════════════ -->
    {:else if error}
        <div class="flex flex-1 items-center justify-center p-8">
            <div
                class={cn(
                    "flex w-full max-w-sm flex-col items-center gap-4 rounded-xl p-8 text-center",
                    "border border-destructive/30 bg-destructive/5",
                )}
            >
                <div
                    class="flex h-14 w-14 items-center justify-center rounded-full bg-destructive/10 text-destructive"
                >
                    <AlertCircle size={28} />
                </div>
                <div>
                    <p class="text-sm font-semibold text-foreground">
                        加载目录失败
                    </p>
                    <p
                        class="mt-1.5 text-xs leading-relaxed text-muted-foreground"
                    >
                        {error}
                    </p>
                </div>
                <p class="text-xs text-muted-foreground">
                    请使用工具栏的刷新按钮重试，或导航到其他目录。
                </p>
            </div>
        </div>

        <!-- ══════════════════════════════════════════════════════════════════════ -->
        <!-- EMPTY STATE                                                            -->
        <!-- ══════════════════════════════════════════════════════════════════════ -->
    {:else if displayEntries.length === 0}
        <div class="flex flex-1 items-center justify-center p-8">
            <div class="flex flex-col items-center gap-3 text-center">
                {#if isSearchMode}
                    <div
                        class="flex h-16 w-16 items-center justify-center rounded-full bg-muted text-muted-foreground"
                    >
                        <Search size={30} />
                    </div>
                    <p class="text-sm font-semibold text-foreground">
                        未找到匹配的文件
                    </p>
                    <p class="text-xs text-muted-foreground">
                        请尝试其他关键词或检查拼写
                    </p>
                {:else}
                    <div
                        class="flex h-16 w-16 items-center justify-center rounded-full bg-muted text-muted-foreground"
                    >
                        <FolderOpen size={30} />
                    </div>
                    <p class="text-sm font-semibold text-foreground">空目录</p>
                    <p class="text-xs text-muted-foreground">
                        此目录中没有任何文件或文件夹
                    </p>
                {/if}
            </div>
        </div>

        <!-- ══════════════════════════════════════════════════════════════════════ -->
        <!-- LIST VIEW                                                              -->
        <!-- ══════════════════════════════════════════════════════════════════════ -->
    {:else if viewMode === "list"}
        <div class="relative flex-1 overflow-auto">
            <table class="w-full table-fixed border-separate border-spacing-0">
                <!-- Sticky header -->
                <thead class="sticky top-0 z-10">
                    <tr
                        class="border-b border-border bg-muted/60 backdrop-blur-sm"
                    >
                        <!-- Select-all checkbox -->
                        <th class="w-10 p-3 text-center">
                            <input
                                bind:this={headerCbEl}
                                type="checkbox"
                                checked={allSelected}
                                onchange={(e) =>
                                    handleSelectAll(
                                        (e.currentTarget as HTMLInputElement)
                                            .checked,
                                    )}
                                class="h-4 w-4 cursor-pointer rounded border-border accent-primary"
                                aria-label="全选"
                            />
                        </th>

                        <!-- Name (sortable) -->
                        <th class="p-3 text-left">
                            <button
                                onclick={() => onSort("name")}
                                class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground"
                            >
                                名称
                                {#if sortField === "name"}
                                    {#if sortOrder === "asc"}
                                        <ArrowUp
                                            size={12}
                                            class="text-primary"
                                        />
                                    {:else}
                                        <ArrowDown
                                            size={12}
                                            class="text-primary"
                                        />
                                    {/if}
                                {:else}
                                    <ChevronsUpDown
                                        size={12}
                                        class="opacity-40"
                                    />
                                {/if}
                            </button>
                        </th>

                        <!-- Size (sortable) -->
                        <th class="w-28 p-3 text-right">
                            <button
                                onclick={() => onSort("size")}
                                class="ml-auto flex items-center gap-1.5 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground"
                            >
                                {#if sortField === "size"}
                                    {#if sortOrder === "asc"}
                                        <ArrowUp
                                            size={12}
                                            class="text-primary"
                                        />
                                    {:else}
                                        <ArrowDown
                                            size={12}
                                            class="text-primary"
                                        />
                                    {/if}
                                {:else}
                                    <ChevronsUpDown
                                        size={12}
                                        class="opacity-40"
                                    />
                                {/if}
                                大小
                            </button>
                        </th>

                        <!-- Modified (sortable) -->
                        <th class="w-44 p-3 text-left">
                            <button
                                onclick={() => onSort("modified")}
                                class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground"
                            >
                                修改时间
                                {#if sortField === "modified"}
                                    {#if sortOrder === "asc"}
                                        <ArrowUp
                                            size={12}
                                            class="text-primary"
                                        />
                                    {:else}
                                        <ArrowDown
                                            size={12}
                                            class="text-primary"
                                        />
                                    {/if}
                                {:else}
                                    <ChevronsUpDown
                                        size={12}
                                        class="opacity-40"
                                    />
                                {/if}
                            </button>
                        </th>

                        <!-- Permissions (conditional) -->
                        {#if hasPermissions}
                            <th
                                class="w-28 p-3 text-left text-xs font-medium text-muted-foreground"
                            >
                                权限
                            </th>
                        {/if}

                        <!-- Actions -->
                        <th class="w-24 p-3"></th>
                    </tr>
                </thead>

                <tbody>
                    {#each displayEntries as entry, index (entry.path)}
                        {@const iconInfo = getIconInfo(entry)}
                        {@const IconComponent = iconInfo.icon}
                        {@const isSelected = selectedPaths.has(entry.path)}
                        {@const isCut =
                            clipboard?.operation === "cut" &&
                            clipboard.paths.includes(entry.path)}

                        <ContextMenu.Root>
                            <ContextMenu.Trigger asChild>
                                {#snippet child({ props: ctxProps })}
                                    <tr
                                        {...ctxProps}
                                        aria-selected={isSelected}
                                        class={cn(
                                            "group border-b border-border/40 cursor-default select-none transition-colors",
                                            isSelected
                                                ? "bg-accent/70"
                                                : "hover:bg-accent/30",
                                            isCut && "opacity-50",
                                        )}
                                        oncontextmenu={(e) => {
                                            handleContextMenuOpen(entry);
                                            if (
                                                typeof ctxProps.oncontextmenu ===
                                                "function"
                                            ) {
                                                (
                                                    ctxProps as Record<
                                                        string,
                                                        unknown
                                                    >
                                                )["oncontextmenu"] &&
                                                    (
                                                        ctxProps as {
                                                            oncontextmenu: (
                                                                e: MouseEvent,
                                                            ) => void;
                                                        }
                                                    ).oncontextmenu(e);
                                            }
                                        }}
                                        onclick={(e) =>
                                            handleRowClick(e, entry, index)}
                                        ondblclick={() =>
                                            handleRowDoubleClick(entry)}
                                    >
                                        <!-- Checkbox cell -->
                                        <td
                                            class="p-3 text-center"
                                            onclick={(e) => e.stopPropagation()}
                                            ondblclick={(e) =>
                                                e.stopPropagation()}
                                        >
                                            <input
                                                type="checkbox"
                                                checked={isSelected}
                                                onchange={(e) => {
                                                    const next = new Set(
                                                        selectedPaths,
                                                    );
                                                    if (
                                                        (
                                                            e.currentTarget as HTMLInputElement
                                                        ).checked
                                                    ) {
                                                        next.add(entry.path);
                                                    } else {
                                                        next.delete(entry.path);
                                                    }
                                                    onSelect(next);
                                                }}
                                                class="h-4 w-4 cursor-pointer rounded border-border accent-primary"
                                                aria-label={`选择 ${entry.name}`}
                                            />
                                        </td>

                                        <!-- Name + Icon cell -->
                                        <td class="p-3">
                                            <div
                                                class="flex min-w-0 items-center gap-2.5"
                                            >
                                                <IconComponent
                                                    size={17}
                                                    class={cn(
                                                        "shrink-0",
                                                        iconInfo.colorClass,
                                                    )}
                                                />
                                                <span
                                                    class={cn(
                                                        "min-w-0 flex-1 truncate text-sm text-foreground",
                                                        entry.name.startsWith(
                                                            ".",
                                                        ) &&
                                                            "text-muted-foreground",
                                                        entry.is_dir &&
                                                            "font-medium",
                                                    )}
                                                    title={entry.name}
                                                >
                                                    {entry.name}
                                                </span>
                                                {#if entry.is_symlink}
                                                    <span
                                                        class="shrink-0 rounded bg-muted px-1 py-0.5 text-[10px] text-muted-foreground"
                                                    >
                                                        链接
                                                    </span>
                                                {/if}
                                                {#if entry.readonly}
                                                    <span
                                                        class="shrink-0 rounded bg-muted px-1 py-0.5 text-[10px] text-muted-foreground"
                                                    >
                                                        只读
                                                    </span>
                                                {/if}
                                            </div>
                                        </td>

                                        <!-- Size cell -->
                                        <td
                                            class="p-3 text-right font-mono text-xs text-muted-foreground"
                                        >
                                            {entry.is_dir
                                                ? "—"
                                                : entry.size_human}
                                        </td>

                                        <!-- Modified cell -->
                                        <td
                                            class="p-3 text-xs text-muted-foreground"
                                        >
                                            {formatDate(entry.modified)}
                                        </td>

                                        <!-- Permissions cell (conditional) -->
                                        {#if hasPermissions}
                                            <td
                                                class="p-3 font-mono text-xs text-muted-foreground"
                                            >
                                                {entry.permissions ?? ""}
                                            </td>
                                        {/if}

                                        <!-- Actions cell -->
                                        <td
                                            class="p-3"
                                            onclick={(e) => e.stopPropagation()}
                                            ondblclick={(e) =>
                                                e.stopPropagation()}
                                        >
                                            <div
                                                class="flex items-center justify-end gap-0.5 opacity-0 transition-opacity group-hover:opacity-100"
                                            >
                                                {#if !entry.is_dir}
                                                    <button
                                                        onclick={() => {
                                                            onDownload(entry);
                                                            downloadFile(
                                                                entry.path,
                                                            );
                                                        }}
                                                        title="下载"
                                                        class={cn(
                                                            "flex h-6 w-6 items-center justify-center rounded transition-colors",
                                                            "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                                                        )}
                                                    >
                                                        <Download size={13} />
                                                    </button>
                                                {/if}
                                                <button
                                                    onclick={() =>
                                                        onRename(entry)}
                                                    title="重命名"
                                                    class={cn(
                                                        "flex h-6 w-6 items-center justify-center rounded transition-colors",
                                                        "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                                                    )}
                                                >
                                                    <Pencil size={13} />
                                                </button>
                                                <button
                                                    onclick={() =>
                                                        onDelete(
                                                            getContextTargets(
                                                                entry,
                                                            ),
                                                        )}
                                                    title="删除"
                                                    class={cn(
                                                        "flex h-6 w-6 items-center justify-center rounded transition-colors",
                                                        "text-muted-foreground hover:bg-destructive/80 hover:text-destructive-foreground",
                                                    )}
                                                >
                                                    <Trash2 size={13} />
                                                </button>
                                            </div>
                                        </td>
                                    </tr>
                                {/snippet}
                            </ContextMenu.Trigger>

                            <!-- ── List row context menu ── -->
                            <ContextMenu.Portal>
                                <ContextMenu.Content class={ctxContentCls}>
                                    {#if !entry.is_dir}
                                        <ContextMenu.Item
                                            onclick={() => {
                                                onDownload(entry);
                                                downloadFile(entry.path);
                                            }}
                                            class={ctxItemCls}
                                        >
                                            <Download size={14} />
                                            下载
                                        </ContextMenu.Item>
                                    {/if}

                                    <ContextMenu.Item
                                        onclick={() => onRename(entry)}
                                        class={ctxItemCls}
                                    >
                                        <Pencil size={14} />
                                        重命名
                                    </ContextMenu.Item>

                                    <ContextMenu.Separator
                                        class="my-1 h-px bg-border"
                                    />

                                    <ContextMenu.Item
                                        onclick={() =>
                                            onCopy(
                                                getContextTargets(entry).map(
                                                    (e) => e.path,
                                                ),
                                            )}
                                        class={ctxItemCls}
                                    >
                                        <Copy size={14} />
                                        复制
                                        {#if selectedPaths.size > 1 && selectedPaths.has(entry.path)}
                                            <span
                                                class="ml-auto text-xs text-muted-foreground"
                                            >
                                                ×{selectedPaths.size}
                                            </span>
                                        {/if}
                                    </ContextMenu.Item>

                                    <ContextMenu.Item
                                        onclick={() =>
                                            onCut(
                                                getContextTargets(entry).map(
                                                    (e) => e.path,
                                                ),
                                            )}
                                        class={ctxItemCls}
                                    >
                                        <Scissors size={14} />
                                        剪切
                                        {#if selectedPaths.size > 1 && selectedPaths.has(entry.path)}
                                            <span
                                                class="ml-auto text-xs text-muted-foreground"
                                            >
                                                ×{selectedPaths.size}
                                            </span>
                                        {/if}
                                    </ContextMenu.Item>

                                    {#if clipboard}
                                        <ContextMenu.Item
                                            onclick={onPaste}
                                            class={ctxItemCls}
                                        >
                                            <Clipboard size={14} />
                                            粘贴
                                            <span
                                                class="ml-auto text-xs text-muted-foreground"
                                            >
                                                {clipboard.operation === "cut"
                                                    ? "移动"
                                                    : "复制"} ×{clipboard.paths
                                                    .length}
                                            </span>
                                        </ContextMenu.Item>
                                    {/if}

                                    <ContextMenu.Separator
                                        class="my-1 h-px bg-border"
                                    />

                                    <ContextMenu.Item
                                        onclick={() =>
                                            onDelete(getContextTargets(entry))}
                                        class={ctxItemDestructiveCls}
                                    >
                                        <Trash2 size={14} />
                                        删除
                                        {#if selectedPaths.size > 1 && selectedPaths.has(entry.path)}
                                            <span
                                                class="ml-auto text-xs opacity-70"
                                                >×{selectedPaths.size}</span
                                            >
                                        {/if}
                                    </ContextMenu.Item>
                                </ContextMenu.Content>
                            </ContextMenu.Portal>
                        </ContextMenu.Root>
                    {/each}
                </tbody>
            </table>
        </div>

        <!-- ══════════════════════════════════════════════════════════════════════ -->
        <!-- GRID VIEW                                                              -->
        <!-- ══════════════════════════════════════════════════════════════════════ -->
    {:else}
        <div class="flex-1 overflow-auto p-4">
            <div
                class="grid grid-cols-2 gap-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8"
            >
                {#each displayEntries as entry, index (entry.path)}
                    {@const iconInfo = getIconInfo(entry)}
                    {@const IconComponent = iconInfo.icon}
                    {@const isSelected = selectedPaths.has(entry.path)}
                    {@const isCut =
                        clipboard?.operation === "cut" &&
                        clipboard.paths.includes(entry.path)}

                    <ContextMenu.Root>
                        <ContextMenu.Trigger asChild>
                            {#snippet child({ props: ctxProps })}
                                <div
                                    {...ctxProps}
                                    aria-selected={isSelected}
                                    class={cn(
                                        "group flex cursor-default select-none flex-col items-center",
                                        "gap-2 rounded-xl p-3 transition-all",
                                        isSelected
                                            ? "bg-accent/80 ring-1 ring-primary/40"
                                            : "hover:bg-accent/40",
                                        isCut && "opacity-50",
                                    )}
                                    oncontextmenu={(e) => {
                                        handleContextMenuOpen(entry);
                                        if (
                                            typeof ctxProps.oncontextmenu ===
                                            "function"
                                        ) {
                                            (
                                                ctxProps as {
                                                    oncontextmenu: (
                                                        e: MouseEvent,
                                                    ) => void;
                                                }
                                            ).oncontextmenu(e);
                                        }
                                    }}
                                    onclick={(e) =>
                                        handleRowClick(e, entry, index)}
                                    ondblclick={() =>
                                        handleRowDoubleClick(entry)}
                                >
                                    <!-- Large icon -->
                                    <div class="relative">
                                        <IconComponent
                                            size={48}
                                            class={cn(
                                                iconInfo.colorClass,
                                                isCut && "opacity-70",
                                            )}
                                        />
                                        {#if entry.is_symlink}
                                            <div
                                                class="absolute -bottom-0.5 -right-0.5 flex h-4 w-4 items-center justify-center rounded-full border border-border bg-muted"
                                            >
                                                <span
                                                    class="text-[8px] font-bold leading-none text-muted-foreground"
                                                >
                                                    L
                                                </span>
                                            </div>
                                        {/if}
                                        {#if entry.readonly}
                                            <div
                                                class="absolute -top-0.5 -right-0.5 flex h-4 w-4 items-center justify-center rounded-full border border-border bg-muted"
                                                title="只读"
                                            >
                                                <span
                                                    class="text-[8px] font-bold leading-none text-muted-foreground"
                                                >
                                                    R
                                                </span>
                                            </div>
                                        {/if}
                                    </div>

                                    <!-- Name (2-line clamp) -->
                                    <span
                                        class={cn(
                                            "line-clamp-2 w-full break-all text-center text-xs leading-tight",
                                            isSelected
                                                ? "font-medium text-foreground"
                                                : "text-foreground",
                                            entry.name.startsWith(".") &&
                                                "text-muted-foreground",
                                        )}
                                        title={entry.name}
                                    >
                                        {entry.name}
                                    </span>

                                    <!-- Size (files only) -->
                                    {#if !entry.is_dir}
                                        <span
                                            class="text-[10px] text-muted-foreground"
                                        >
                                            {entry.size_human}
                                        </span>
                                    {/if}
                                </div>
                            {/snippet}
                        </ContextMenu.Trigger>

                        <!-- ── Grid item context menu ── -->
                        <ContextMenu.Portal>
                            <ContextMenu.Content class={ctxContentCls}>
                                {#if !entry.is_dir}
                                    <ContextMenu.Item
                                        onclick={() => {
                                            onDownload(entry);
                                            downloadFile(entry.path);
                                        }}
                                        class={ctxItemCls}
                                    >
                                        <Download size={14} />
                                        下载
                                    </ContextMenu.Item>
                                {/if}

                                <ContextMenu.Item
                                    onclick={() => onRename(entry)}
                                    class={ctxItemCls}
                                >
                                    <Pencil size={14} />
                                    重命名
                                </ContextMenu.Item>

                                <ContextMenu.Separator
                                    class="my-1 h-px bg-border"
                                />

                                <ContextMenu.Item
                                    onclick={() =>
                                        onCopy(
                                            getContextTargets(entry).map(
                                                (e) => e.path,
                                            ),
                                        )}
                                    class={ctxItemCls}
                                >
                                    <Copy size={14} />
                                    复制
                                    {#if selectedPaths.size > 1 && selectedPaths.has(entry.path)}
                                        <span
                                            class="ml-auto text-xs text-muted-foreground"
                                        >
                                            ×{selectedPaths.size}
                                        </span>
                                    {/if}
                                </ContextMenu.Item>

                                <ContextMenu.Item
                                    onclick={() =>
                                        onCut(
                                            getContextTargets(entry).map(
                                                (e) => e.path,
                                            ),
                                        )}
                                    class={ctxItemCls}
                                >
                                    <Scissors size={14} />
                                    剪切
                                    {#if selectedPaths.size > 1 && selectedPaths.has(entry.path)}
                                        <span
                                            class="ml-auto text-xs text-muted-foreground"
                                        >
                                            ×{selectedPaths.size}
                                        </span>
                                    {/if}
                                </ContextMenu.Item>

                                {#if clipboard}
                                    <ContextMenu.Item
                                        onclick={onPaste}
                                        class={ctxItemCls}
                                    >
                                        <Clipboard size={14} />
                                        粘贴
                                        <span
                                            class="ml-auto text-xs text-muted-foreground"
                                        >
                                            {clipboard.operation === "cut"
                                                ? "移动"
                                                : "复制"} ×{clipboard.paths
                                                .length}
                                        </span>
                                    </ContextMenu.Item>
                                {/if}

                                <ContextMenu.Separator
                                    class="my-1 h-px bg-border"
                                />

                                <ContextMenu.Item
                                    onclick={() =>
                                        onDelete(getContextTargets(entry))}
                                    class={ctxItemDestructiveCls}
                                >
                                    <Trash2 size={14} />
                                    删除
                                    {#if selectedPaths.size > 1 && selectedPaths.has(entry.path)}
                                        <span class="ml-auto text-xs opacity-70"
                                            >×{selectedPaths.size}</span
                                        >
                                    {/if}
                                </ContextMenu.Item>
                            </ContextMenu.Content>
                        </ContextMenu.Portal>
                    </ContextMenu.Root>
                {/each}
            </div>
        </div>
    {/if}
</div>
