<script lang="ts">
  import { CheckSquare, Square } from "lucide-svelte";
  import type { ClipSelection } from "./clip/clipSelectionRuntime";

  const mergeClipSelectionsToggleId = `clip-merge-${Math.random()
    .toString(36)
    .slice(2)}`;

  export let clipSelections: ClipSelection[] = [];
  export let activeClipSelectionId: string | null = null;
  export let clipExportSelectionIds: string[] = [];
  export let mergeClipSelectionsOnExport = true;
  export let formatTime: (time: number) => string;
  export let onSetActiveClipSelection:
    | ((id: string | null) => void)
    | undefined = undefined;
  export let onSeekToTime: ((time: number) => void) | undefined = undefined;

  $: selectedIdSet = new Set(clipExportSelectionIds);
  $: selectedCount = clipSelections.filter((selection) =>
    selectedIdSet.has(selection.id)
  ).length;
  $: areAllSelectionsSelected =
    clipSelections.length > 0 && selectedCount === clipSelections.length;
  $: totalSelectedDuration = clipSelections.reduce((total, selection) => {
    if (!selectedIdSet.has(selection.id)) {
      return total;
    }

    return total + getSelectionDuration(selection);
  }, 0);

  function toggleSelection(id: string) {
    if (selectedIdSet.has(id)) {
      clipExportSelectionIds = clipExportSelectionIds.filter(
        (selectionId) => selectionId !== id
      );
      return;
    }

    clipExportSelectionIds = [
      ...clipExportSelectionIds,
      id,
    ].filter((selectionId, index, ids) => ids.indexOf(selectionId) === index);
  }

  function toggleAllSelections() {
    clipExportSelectionIds = areAllSelectionsSelected
      ? []
      : clipSelections.map((selection) => selection.id);
  }

  function getSelectionDuration(selection: ClipSelection) {
    return Math.max(0, selection.endTime - selection.startTime);
  }

</script>

<div class="p-4 space-y-4">
  <div class="p-4 rounded-xl border border-gray-800/50 bg-[#1c1c1e] space-y-4">
    <div class="flex items-start justify-between gap-3">
      <div class="space-y-1">
        <h3 class="text-sm font-medium text-white">选区导出</h3>
        <p class="text-xs text-gray-400 leading-5">
          已选 {selectedCount}/{clipSelections.length}，总长 {formatTime(totalSelectedDuration)}
        </p>
      </div>
      <div class="flex shrink-0 overflow-hidden rounded-lg border border-gray-700">
        <button
          class="px-2.5 py-1 text-xs text-gray-300 hover:text-white hover:bg-white/5 disabled:opacity-40"
          on:click={toggleAllSelections}
          disabled={clipSelections.length === 0}
        >
          {areAllSelectionsSelected ? "全不选" : "全选"}
        </button>
      </div>
    </div>

    <div class="bui bui-switch clip-merge-switch">
      <div class="bui-area">
        <input
          id={mergeClipSelectionsToggleId}
          class="bui-switch-input"
          type="checkbox"
          aria-label="合并导出"
          bind:checked={mergeClipSelectionsOnExport}
        />
        <label class="bui-switch-label" for={mergeClipSelectionsToggleId}>
          <span class="bui-switch-name">
            <span class="bui-switch-title">合并导出</span>
            <span class="bui-switch-desc">勾选选区拼成一个切片。</span>
          </span>
          <span class="bui-switch-body">
            <span class="bui-switch-dot">
              <span></span>
            </span>
          </span>
        </label>
      </div>
    </div>
  </div>

  <div class="space-y-2">
    {#if clipSelections.length === 0}
      <div
        class="rounded-xl border border-dashed border-gray-700 bg-[#1c1c1e] px-4 py-8 text-center text-sm text-gray-400"
      >
        暂无选区
      </div>
    {:else}
      {#each clipSelections as selection, index (selection.id)}
        <div
          class="rounded-xl border border-gray-800/50 bg-[#1c1c1e] p-3 transition-colors duration-200"
          class:border-[#0A84FF]={selection.id === activeClipSelectionId}
          class:bg-[#242426]={selection.id === activeClipSelectionId}
        >
          <div class="flex items-start gap-3">
            <button
              class="mt-0.5 text-[#0A84FF] hover:text-[#0A84FF]/80"
              aria-label={selectedIdSet.has(selection.id)
                ? "取消导出选区"
                : "导出选区"}
              on:click={() => toggleSelection(selection.id)}
            >
              {#if selectedIdSet.has(selection.id)}
                <CheckSquare class="w-5 h-5" />
              {:else}
                <Square class="w-5 h-5" />
              {/if}
            </button>

            <div class="min-w-0 flex-1">
              <div class="flex items-center justify-between gap-2">
                <button
                  class="min-w-0 truncate text-left text-sm font-medium text-white hover:text-[#0A84FF]"
                  on:click={() => onSetActiveClipSelection?.(selection.id)}
                >
                  选区 {index + 1}
                </button>
                <span class="text-xs text-gray-400">
                  {formatTime(getSelectionDuration(selection))}
                </span>
              </div>
              <div class="mt-2 flex items-center gap-1 text-xs text-gray-400">
                <button
                  class="text-[#0A84FF] hover:text-[#0A84FF]/80"
                  on:click|stopPropagation={() => onSeekToTime?.(selection.startTime)}
                >
                  {formatTime(selection.startTime)}
                </button>
                <span>→</span>
                <button
                  class="text-[#0A84FF] hover:text-[#0A84FF]/80"
                  on:click|stopPropagation={() => onSeekToTime?.(selection.endTime)}
                >
                  {formatTime(selection.endTime)}
                </button>
              </div>
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>
