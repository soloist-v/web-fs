<script lang="ts">
    import {
        listDir,
        searchFiles,
        downloadFile,
        copyEntry,
        renameEntry,
    } from "$lib/api";
    import type {
        FileEntry,
        DirListing,
        SortField,
        SortOrder,
        ViewMode,
        SearchResponse,
        UserSettings,
    } from "$lib/types";
    import { DEFAULT_SETTINGS } from "$lib/types";
    import { toast } from "svelte-sonner";

    import Toolbar from "./Toolbar.svelte";
    import FileBrowser from "./FileBrowser.svelte";
    import FilePreview from "./FilePreview.svelte";
    import UploadZone from "./UploadZone.svelte";
    import NewFolderDialog from "./NewFolderDialog.svelte";
    import RenameDialog from "./RenameDialog.svelte";
    import DeleteDialog from "./DeleteDialog.svelte";

    // ── Navigation state ─────────────────────────────────────────────────────
    const CURRENT_PATH_KEY = "web_fs:currentPath";

    function loadCurrentPath(): string {
        try {
            return localStorage.getItem(CURRENT_PATH_KEY) ?? "/";
        } catch {
            return "/";
        }
    }

    function saveCurrentPath(p: string) {
        try {
            localStorage.setItem(CURRENT_PATH_KEY, p);
        } catch {
            // ignore
        }
    }

    const _initPath = loadCurrentPath();
    let currentPath = $state(_initPath);
    let navHistory = $state<string[]>([_initPath]);
    let historyIndex = $state(0);

    // ── Directory listing state ───────────────────────────────────────────────
    let entries = $state<FileEntry[]>([]);
    let loading = $state(false);
    let loadError = $state<string | null>(null);

    // ── Persist settings to / from localStorage ───────────────────────────────
    const SETTINGS_KEY = "web_fs:settings";

    function loadSettings(): UserSettings {
        try {
            const raw = localStorage.getItem(SETTINGS_KEY);
            if (raw) {
                const parsed = JSON.parse(raw) as Partial<UserSettings>;
                return { ...DEFAULT_SETTINGS, ...parsed };
            }
        } catch {
            // ignore parse errors
        }
        return { ...DEFAULT_SETTINGS };
    }

    function saveSettings() {
        try {
            const s: UserSettings = {
                viewMode,
                sortField,
                sortOrder,
                showHidden,
                pageSize: DEFAULT_SETTINGS.pageSize,
                confirmDelete: DEFAULT_SETTINGS.confirmDelete,
            };
            localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
        } catch {
            // ignore write errors (e.g. private browsing quota)
        }
    }

    // ── View/sort settings (initialised from localStorage) ────────────────────
    const _init = loadSettings();
    let viewMode = $state<ViewMode>(_init.viewMode);
    let sortField = $state<SortField>(_init.sortField);
    let sortOrder = $state<SortOrder>(_init.sortOrder);
    let showHidden = $state(_init.showHidden);

    // Persist whenever any setting changes
    $effect(() => {
        // Read each reactive value so Svelte tracks them all
        const _v = viewMode;
        const _sf = sortField;
        const _so = sortOrder;
        const _sh = showHidden;
        saveSettings();
    });

    // ── Selection ─────────────────────────────────────────────────────────────
    let selectedPaths = $state<Set<string>>(new Set());

    // ── Search ────────────────────────────────────────────────────────────────
    let searchQuery = $state("");
    let searchResults = $state<FileEntry[] | null>(null);
    let searchTimer: ReturnType<typeof setTimeout> | null = null;

    // ── Preview panel ─────────────────────────────────────────────────────────
    let previewEntry = $state<FileEntry | null>(null);

    // ── Upload zone ───────────────────────────────────────────────────────────
    let showUploadZone = $state(false);
    // Track whether a file drag is over the window (not just the upload panel)
    let windowDragCounter = $state(0);

    // ── Clipboard (copy / cut) ────────────────────────────────────────────────
    let clipboard = $state<{
        paths: string[];
        operation: "copy" | "cut";
    } | null>(null);

    // ── Dialogs ───────────────────────────────────────────────────────────────
    let showNewFolderDialog = $state(false);
    let renameTarget = $state<FileEntry | null>(null);
    let deleteTargets = $state<FileEntry[]>([]);

    // ── Derived ──────────────────────────────────────────────────────────────
    const canGoBack = $derived(historyIndex > 0);
    const canGoForward = $derived(historyIndex < navHistory.length - 1);
    const breadcrumbs = $derived(buildBreadcrumbs(currentPath));

    function buildBreadcrumbs(path: string) {
        const crumbs = [{ label: "根目录", path: "/" }];
        if (path === "/") return crumbs;
        const parts = path.split("/").filter(Boolean);
        let cum = "";
        for (const p of parts) {
            cum += "/" + p;
            crumbs.push({ label: p, path: cum });
        }
        return crumbs;
    }

    // ── Reactive loader ───────────────────────────────────────────────────────
    // Re-runs whenever the path, sort, or hidden-file toggle changes.
    $effect(() => {
        const path = currentPath;
        const sort = sortField;
        const order = sortOrder;
        const hidden = showHidden;

        loading = true;
        loadError = null;

        listDir(path, sort, order, hidden)
            .then((result: DirListing) => {
                entries = result.entries;
                loading = false;
                // Persist the successfully loaded path
                saveCurrentPath(path);
            })
            .catch((err: unknown) => {
                // If the persisted path no longer exists, fall back to "/"
                if (path !== "/") {
                    saveCurrentPath("/");
                    currentPath = "/";
                    navHistory = ["/"];
                    historyIndex = 0;
                } else {
                    loadError =
                        err instanceof Error ? err.message : "加载目录失败";
                    loading = false;
                }
            });
    });

    // ── Navigation ────────────────────────────────────────────────────────────
    function navigate(path: string) {
        if (path === currentPath) {
            refresh();
            return;
        }
        // Truncate forward history then append
        navHistory = [...navHistory.slice(0, historyIndex + 1), path];
        historyIndex = navHistory.length - 1;
        currentPath = path;
        // Reset UI state for the new path
        selectedPaths = new Set();
        clearSearch();
        previewEntry = null;
    }

    function goBack() {
        if (!canGoBack) return;
        historyIndex -= 1;
        currentPath = navHistory[historyIndex];
        selectedPaths = new Set();
        clearSearch();
    }

    function goForward() {
        if (!canGoForward) return;
        historyIndex += 1;
        currentPath = navHistory[historyIndex];
        selectedPaths = new Set();
        clearSearch();
    }

    function goUp() {
        if (currentPath === "/") return;
        const parent =
            currentPath.substring(0, currentPath.lastIndexOf("/")) || "/";
        navigate(parent);
    }

    function refresh() {
        // Force the $effect to re-run by creating a dummy assignment that Svelte
        // will notice doesn't change the value, but calling listDir explicitly.
        loading = true;
        loadError = null;
        listDir(currentPath, sortField, sortOrder, showHidden)
            .then((result: DirListing) => {
                entries = result.entries;
                loading = false;
            })
            .catch((err: unknown) => {
                loadError = err instanceof Error ? err.message : "加载目录失败";
                loading = false;
            });
    }

    // ── Sort ──────────────────────────────────────────────────────────────────
    function handleSort(field: SortField) {
        if (sortField === field) {
            sortOrder = sortOrder === "asc" ? "desc" : "asc";
        } else {
            sortField = field;
            sortOrder = "asc";
        }
        // $effect above will fire automatically because sortField / sortOrder changed
    }

    // ── Search ────────────────────────────────────────────────────────────────
    function handleSearchChange(q: string) {
        searchQuery = q;
        if (searchTimer) {
            clearTimeout(searchTimer);
            searchTimer = null;
        }
        if (!q.trim()) {
            searchResults = null;
            return;
        }
        searchTimer = setTimeout(async () => {
            try {
                const res: SearchResponse = await searchFiles(currentPath, q);
                searchResults = res.results.map((r) => r.entry);
            } catch {
                searchResults = [];
            }
        }, 300);
    }

    function clearSearch() {
        if (searchTimer) {
            clearTimeout(searchTimer);
            searchTimer = null;
        }
        searchQuery = "";
        searchResults = null;
    }

    // ── Clipboard ─────────────────────────────────────────────────────────────
    function handleCopy(paths: string[]) {
        clipboard = { paths, operation: "copy" };
        toast.success(`已复制 ${paths.length} 个项目`, {
            description: "可在目标目录粘贴",
        });
    }

    function handleCut(paths: string[]) {
        clipboard = { paths, operation: "cut" };
        toast.success(`已剪切 ${paths.length} 个项目`, {
            description: "可在目标目录粘贴",
        });
    }

    async function handlePaste() {
        if (!clipboard) return;
        const { paths, operation } = clipboard;

        try {
            for (const srcPath of paths) {
                const name = srcPath.split("/").pop()!;
                const dst =
                    (currentPath === "/" ? "" : currentPath) + "/" + name;
                if (operation === "copy") {
                    await copyEntry(srcPath, dst);
                } else {
                    await renameEntry(srcPath, dst);
                }
            }
            if (operation === "cut") clipboard = null;
            refresh();
            toast.success("粘贴成功");
        } catch (err) {
            toast.error("粘贴失败", {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    }

    function handleCopySelected() {
        const paths = [...selectedPaths];
        if (paths.length === 0) return;
        handleCopy(paths);
    }

    function handleCutSelected() {
        const paths = [...selectedPaths];
        if (paths.length === 0) return;
        handleCut(paths);
    }

    // ── Delete ────────────────────────────────────────────────────────────────
    function handleDelete(targets: FileEntry[]) {
        if (targets.length === 0) return;
        deleteTargets = targets;
    }

    function handleDeleteSelected() {
        const targets = [...selectedPaths]
            .map((p) => entries.find((e) => e.path === p))
            .filter((e): e is FileEntry => e != null);
        handleDelete(targets);
    }

    // ── Rename ────────────────────────────────────────────────────────────────
    function handleRename(entry: FileEntry) {
        renameTarget = entry;
    }

    // ── Download ──────────────────────────────────────────────────────────────
    function handleDownload(entry: FileEntry) {
        downloadFile(entry.path);
    }

    // ── Window drag-and-drop → open upload zone ───────────────────────────────
    function onWindowDragEnter(e: DragEvent) {
        if (!e.dataTransfer?.types.includes("Files")) return;
        e.preventDefault();
        windowDragCounter += 1;
        if (windowDragCounter === 1) showUploadZone = true;
    }

    function onWindowDragLeave(e: DragEvent) {
        if (!e.dataTransfer?.types.includes("Files")) return;
        windowDragCounter -= 1;
        if (windowDragCounter <= 0) {
            windowDragCounter = 0;
            // Don't close here — let UploadZone's close button do it,
            // so the user has a chance to drop the file.
        }
    }

    function onWindowDragOver(e: DragEvent) {
        // Must prevent default to allow dropping
        if (e.dataTransfer?.types.includes("Files")) e.preventDefault();
    }

    // Close upload zone and reset drag state
    function handleUploadClose() {
        showUploadZone = false;
        windowDragCounter = 0;
    }

    function handleUploadComplete() {
        refresh();
        // Don't auto-close; UploadZone has its own "完成" button
    }
</script>

<!-- Global drag-over listeners to detect file-drops anywhere on the page -->
<svelte:window
    ondragenter={onWindowDragEnter}
    ondragleave={onWindowDragLeave}
    ondragover={onWindowDragOver}
/>

<div class="flex h-dvh flex-col overflow-hidden bg-background">
    <!-- ── Toolbar ── -->
    <Toolbar
        {currentPath}
        {breadcrumbs}
        {viewMode}
        {sortField}
        {sortOrder}
        {showHidden}
        {searchQuery}
        {canGoBack}
        {canGoForward}
        onNavigate={navigate}
        onBack={goBack}
        onForward={goForward}
        onUp={goUp}
        onRefresh={refresh}
        onViewModeChange={(mode: ViewMode) => (viewMode = mode)}
        onToggleHidden={() => {
            showHidden = !showHidden;
        }}
        onSearchChange={handleSearchChange}
        onNewFolder={() => (showNewFolderDialog = true)}
        onUpload={() => (showUploadZone = true)}
        onSort={handleSort}
    />

    <!-- ── Main content area ── -->
    <main class="flex min-h-0 flex-1 overflow-hidden">
        <FileBrowser
            {entries}
            {loading}
            error={loadError}
            {viewMode}
            {sortField}
            {sortOrder}
            {selectedPaths}
            {searchResults}
            onNavigate={navigate}
            onSort={handleSort}
            onSelect={(paths: Set<string>) => (selectedPaths = paths)}
            onPreview={(entry: FileEntry) => (previewEntry = entry)}
            onRename={handleRename}
            onDelete={handleDelete}
            onDownload={handleDownload}
            onCopy={handleCopy}
            onCut={handleCut}
            onPaste={handlePaste}
            {clipboard}
        />
    </main>

    <!-- ── Preview dialog (portal, zero layout impact) ── -->
    <FilePreview entry={previewEntry} onclose={() => (previewEntry = null)} />

    <!-- ── Upload zone (modal overlay) ── -->
    <UploadZone
        path={currentPath}
        show={showUploadZone}
        onupload={handleUploadComplete}
        onclose={handleUploadClose}
    />

    <!-- ── Dialogs ── -->
    <NewFolderDialog
        open={showNewFolderDialog}
        {currentPath}
        onOpenChange={(v: boolean) => (showNewFolderDialog = v)}
        onCreated={() => {
            showNewFolderDialog = false;
            refresh();
        }}
    />

    <RenameDialog
        open={renameTarget !== null}
        entry={renameTarget}
        onOpenChange={(v: boolean) => {
            if (!v) renameTarget = null;
        }}
        onRenamed={() => {
            renameTarget = null;
            refresh();
        }}
    />

    <DeleteDialog
        open={deleteTargets.length > 0}
        entries={deleteTargets}
        onOpenChange={(v: boolean) => {
            if (!v) deleteTargets = [];
        }}
        onDeleted={() => {
            deleteTargets = [];
            selectedPaths = new Set();
            refresh();
        }}
    />
</div>
