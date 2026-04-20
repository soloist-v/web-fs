<script lang="ts">
    import { DropdownMenu } from "bits-ui";
    import {
        ChevronLeft,
        ChevronRight,
        ChevronUp,
        RefreshCw,
        Search,
        X,
        FolderPlus,
        Upload,
        List,
        LayoutGrid,
        MoreVertical,
        Eye,
        EyeOff,
        Check,
    } from "lucide-svelte";
    import { cn } from "$lib/utils";
    import type { ViewMode, SortField, SortOrder } from "$lib/types";

    interface Props {
        currentPath: string;
        breadcrumbs: Array<{ label: string; path: string }>;
        viewMode: ViewMode;
        sortField: SortField;
        sortOrder: SortOrder;
        showHidden: boolean;
        searchQuery: string;
        canGoBack: boolean;
        canGoForward: boolean;
        onNavigate: (path: string) => void;
        onBack: () => void;
        onForward: () => void;
        onUp: () => void;
        onRefresh: () => void;
        onViewModeChange: (mode: ViewMode) => void;
        onToggleHidden: () => void;
        onSearchChange: (q: string) => void;
        onNewFolder: () => void;
        onUpload: () => void;
        onSort: (field: SortField) => void;
    }

    let {
        currentPath,
        breadcrumbs,
        viewMode,
        sortField,
        sortOrder,
        showHidden,
        searchQuery,
        canGoBack,
        canGoForward,
        onNavigate,
        onBack,
        onForward,
        onUp,
        onRefresh,
        onViewModeChange,
        onToggleHidden,
        onSearchChange,
        onNewFolder,
        onUpload,
        onSort,
    }: Props = $props();

    // Local search state — synced from parent for external clears
    let localSearch = $state(searchQuery);

    $effect(() => {
        localSearch = searchQuery;
    });

    function handleSearchInput(e: Event) {
        const value = (e.target as HTMLInputElement).value;
        localSearch = value;
        onSearchChange(value);
    }

    function handleSearchKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            localSearch = "";
            onSearchChange("");
        }
    }

    function clearSearch() {
        localSearch = "";
        onSearchChange("");
    }

    const sortLabels: Record<SortField, string> = {
        name: "名称",
        size: "大小",
        modified: "修改时间",
        type: "类型",
    };

    const sortFieldList: SortField[] = ["name", "size", "modified", "type"];
</script>

<div
    class="flex flex-col border-b border-border bg-background/80 backdrop-blur-sm"
>
    <!-- ── Row 1: Main toolbar (48 px) ── -->
    <div class="flex h-12 items-center gap-1 px-2">
        <!-- Navigation buttons -->
        <div class="flex items-center gap-0.5">
            <button
                class={cn(
                    "inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition-colors",
                    "hover:bg-accent hover:text-accent-foreground",
                    "disabled:pointer-events-none disabled:opacity-40",
                )}
                onclick={onBack}
                disabled={!canGoBack}
                title="后退 (Alt+Left)"
                aria-label="后退"
            >
                <ChevronLeft class="h-4 w-4" />
            </button>

            <button
                class={cn(
                    "inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition-colors",
                    "hover:bg-accent hover:text-accent-foreground",
                    "disabled:pointer-events-none disabled:opacity-40",
                )}
                onclick={onForward}
                disabled={!canGoForward}
                title="前进 (Alt+Right)"
                aria-label="前进"
            >
                <ChevronRight class="h-4 w-4" />
            </button>

            <button
                class="inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                onclick={onUp}
                title="上级目录 (Alt+Up)"
                aria-label="上级目录"
            >
                <ChevronUp class="h-4 w-4" />
            </button>

            <button
                class="inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                onclick={onRefresh}
                title="刷新 (F5)"
                aria-label="刷新"
            >
                <RefreshCw class="h-4 w-4" />
            </button>
        </div>

        <span class="mx-1 h-5 w-px shrink-0 bg-border"></span>

        <!-- Breadcrumbs (flex-1, scrollable) -->
        <nav
            class="flex min-w-0 flex-1 items-center overflow-x-auto"
            aria-label="导航路径"
        >
            <ol class="flex items-center gap-0.5">
                {#each breadcrumbs as crumb, i (crumb.path)}
                    {#if i > 0}
                        <li class="shrink-0 text-muted-foreground/40">
                            <ChevronRight class="h-3 w-3" />
                        </li>
                    {/if}
                    <li>
                        {#if i === breadcrumbs.length - 1}
                            <span
                                class="block max-w-[9rem] truncate rounded px-1 py-0.5 text-sm font-medium text-foreground"
                                title={crumb.label}
                            >
                                {crumb.label}
                            </span>
                        {:else}
                            <button
                                class="block max-w-[8rem] truncate rounded px-1 py-0.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                                onclick={() => onNavigate(crumb.path)}
                                title={crumb.path}
                            >
                                {crumb.label}
                            </button>
                        {/if}
                    </li>
                {/each}
            </ol>
        </nav>

        <span class="mx-1 h-5 w-px shrink-0 bg-border"></span>

        <!-- Search box -->
        <div class="relative flex w-52 shrink-0 items-center">
            <Search
                class="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-muted-foreground"
            />
            <input
                type="search"
                value={localSearch}
                oninput={handleSearchInput}
                onkeydown={handleSearchKeydown}
                placeholder="搜索文件..."
                aria-label="搜索文件"
                class="h-8 w-full rounded-md border border-border bg-muted/50 pl-8 pr-7 text-sm text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none focus:ring-1 focus:ring-ring"
            />
            {#if localSearch}
                <button
                    type="button"
                    class="absolute right-2 flex items-center text-muted-foreground transition-colors hover:text-foreground"
                    onclick={clearSearch}
                    title="清空搜索"
                    aria-label="清空搜索"
                >
                    <X class="h-3.5 w-3.5" />
                </button>
            {/if}
        </div>

        <span class="mx-1 h-5 w-px shrink-0 bg-border"></span>

        <!-- Action buttons + view toggle + more menu -->
        <div class="flex items-center gap-0.5">
            <!-- New Folder -->
            <button
                class="inline-flex h-8 items-center gap-1.5 rounded-md px-2.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                onclick={onNewFolder}
                title="新建文件夹"
            >
                <FolderPlus class="h-4 w-4 shrink-0" />
                <span class="hidden lg:inline">新建文件夹</span>
            </button>

            <!-- Upload -->
            <button
                class="inline-flex h-8 items-center gap-1.5 rounded-md px-2.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                onclick={onUpload}
                title="上传文件"
            >
                <Upload class="h-4 w-4 shrink-0" />
                <span class="hidden lg:inline">上传</span>
            </button>

            <span class="mx-0.5 h-5 w-px shrink-0 bg-border"></span>

            <!-- View mode toggle -->
            <div class="flex overflow-hidden rounded-md border border-border">
                <button
                    class={cn(
                        "inline-flex h-8 w-8 items-center justify-center transition-colors",
                        viewMode === "list"
                            ? "bg-primary text-primary-foreground"
                            : "bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                    )}
                    onclick={() => onViewModeChange("list")}
                    title="列表视图"
                    aria-label="列表视图"
                    aria-pressed={viewMode === "list"}
                >
                    <List class="h-4 w-4" />
                </button>
                <button
                    class={cn(
                        "inline-flex h-8 w-8 items-center justify-center transition-colors",
                        viewMode === "grid"
                            ? "bg-primary text-primary-foreground"
                            : "bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                    )}
                    onclick={() => onViewModeChange("grid")}
                    title="网格视图"
                    aria-label="网格视图"
                    aria-pressed={viewMode === "grid"}
                >
                    <LayoutGrid class="h-4 w-4" />
                </button>
            </div>

            <!-- More options dropdown -->
            <DropdownMenu.Root>
                <DropdownMenu.Trigger
                    class="ml-0.5 inline-flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                    aria-label="更多选项"
                    title="更多选项"
                >
                    <MoreVertical class="h-4 w-4" />
                </DropdownMenu.Trigger>

                <DropdownMenu.Portal>
                    <DropdownMenu.Content
                        class="z-50 min-w-52 rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-lg"
                        align="end"
                        sideOffset={5}
                    >
                        <!-- Toggle hidden files -->
                        <DropdownMenu.Item
                            class="flex cursor-pointer items-center gap-2.5 rounded-md px-3 py-2 text-sm outline-none hover:bg-accent hover:text-accent-foreground"
                            onSelect={onToggleHidden}
                        >
                            {#if showHidden}
                                <EyeOff
                                    class="h-4 w-4 shrink-0 text-muted-foreground"
                                />
                                <span class="flex-1">隐藏隐藏文件</span>
                                <Check
                                    class="h-3.5 w-3.5 shrink-0 text-primary"
                                />
                            {:else}
                                <Eye
                                    class="h-4 w-4 shrink-0 text-muted-foreground"
                                />
                                <span class="flex-1">显示隐藏文件</span>
                            {/if}
                        </DropdownMenu.Item>

                        <DropdownMenu.Separator class="my-1 h-px bg-border" />

                        <div
                            class="px-3 pb-1 pt-1.5 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
                        >
                            排序方式
                        </div>

                        {#each sortFieldList as field (field)}
                            <DropdownMenu.Item
                                class="flex cursor-pointer items-center gap-2.5 rounded-md px-3 py-2 text-sm outline-none hover:bg-accent hover:text-accent-foreground"
                                onSelect={() => onSort(field)}
                            >
                                <span class="flex-1">{sortLabels[field]}</span>
                                {#if sortField === field}
                                    <Check
                                        class="h-3.5 w-3.5 shrink-0 text-primary"
                                    />
                                {/if}
                            </DropdownMenu.Item>
                        {/each}
                    </DropdownMenu.Content>
                </DropdownMenu.Portal>
            </DropdownMenu.Root>
        </div>
    </div>
</div>
