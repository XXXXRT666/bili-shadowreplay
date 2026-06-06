<script lang="ts">
  import { Download, Film, Scissors, Trash2 } from "lucide-svelte";
  import type { VideoItem } from "../../interface";

  export let video: VideoItem;
  export let videos: Array<VideoItem & { name?: string }> = [];
  export let videoSelectLabel = "";
  export let isVideoSelectMenuVisible = false;
  export let videoSelectMenuController: {
    close: () => void;
    open: () => void;
  };
  export let onToggleVideoMenu: (() => void) | undefined = undefined;
  export let onSelectVideo: ((video: VideoItem) => void) | undefined = undefined;
  export let onDeleteVideo: (() => void | Promise<void>) | undefined = undefined;
  export let showDownloadButton = false;
  export let onDownloadVideo: (() => void | Promise<void>) | undefined = undefined;
  export let canClip = false;
  export let clipping = false;
  export let currentClipEventId: string | null = null;
  export let onGenerateClip: (() => void | Promise<void>) | undefined = undefined;
  export let currentEncodeEventId: string | null = null;
  export let onOpenEncodeModal: (() => void) | undefined = undefined;
</script>

<div class="clip-top-bar relative z-10 flex h-14 shrink-0 border-b border-gray-800/50 bg-[#2c2c2e]">
  <div class="flex min-w-0 flex-1 items-center px-4">
    <div class="clip-video-select relative flex min-w-0 flex-1 items-center">
      <button
        type="button"
        class="clip-video-select-trigger"
        aria-label="选择视频"
        aria-expanded={isVideoSelectMenuVisible}
        on:click={onToggleVideoMenu}
      >
        <span class="clip-video-select-value">{videoSelectLabel}</span>
        <span
          class="clip-video-select-chevron"
          class:is-open={isVideoSelectMenuVisible}
          aria-hidden="true"
        >
          ▼
        </span>
      </button>
      {#if isVideoSelectMenuVisible}
        <div class="clip-video-select-menu pointer-events-auto">
          {#each videos as item}
            <button
              type="button"
              class="clip-video-select-item"
              class:is-active={item.id === video.id}
              on:click={() => {
                videoSelectMenuController.close();
                onSelectVideo?.(item);
              }}
            >
              {item.name ?? ""}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <div
    class="clip-top-bar-side-spacer w-80 shrink-0 border-l border-gray-800/50 px-4 flex items-center justify-between gap-3"
  >
    <div class="flex shrink-0 items-center gap-2">
      <button
        class="text-red-500 hover:text-red-400 transition-colors duration-200 px-2 py-1.5 rounded-md hover:bg-red-500/10"
        on:click={onDeleteVideo}
      >
        <Trash2 class="w-4 h-4" />
      </button>
      {#if showDownloadButton}
        <button
          class="text-blue-500 hover:text-blue-400 transition-colors duration-200 px-2 py-1.5 rounded-md hover:bg-blue-500/10"
          on:click={onDownloadVideo}
        >
          <Download class="w-4 h-4" />
        </button>
      {/if}
    </div>

    <div class="flex shrink-0 items-center space-x-2">
      {#if canClip}
        <button
          class="px-4 py-1.5 text-sm bg-green-600 text-white rounded-md hover:bg-green-600/90 transition-colors duration-200 border border-gray-600/50 flex items-center space-x-2 disabled:opacity-50 disabled:cursor-not-allowed"
          on:click={onGenerateClip}
          disabled={clipping || currentClipEventId !== null}
        >
          {#if clipping || currentClipEventId !== null}
            <svg
              class="animate-spin h-4 w-4"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                class="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                stroke-width="4"
              ></circle>
              <path
                class="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              ></path>
            </svg>
          {:else}
            <Scissors class="w-4 h-4" />
          {/if}
          <span id="generate-clip-prompt">{clipping ? "生成中..." : "生成切片"}</span>
        </button>
      {/if}

      <button
        class="px-4 py-1.5 text-sm bg-[#0A84FF] text-white rounded-md hover:bg-[#0A84FF]/90 transition-colors duration-200 border border-gray-600/50 flex items-center space-x-2 disabled:opacity-50 disabled:cursor-not-allowed"
        on:click={onOpenEncodeModal}
        disabled={currentEncodeEventId !== null}
      >
        {#if currentEncodeEventId !== null}
          <svg
            class="animate-spin h-4 w-4"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              class="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              stroke-width="4"
            ></circle>
            <path
              class="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 714 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            ></path>
          </svg>
        {:else}
          <Film class="w-4 h-4" />
        {/if}
        <span id="encode-prompt">压制</span>
      </button>
    </div>
  </div>
</div>

<style>
  .clip-video-select {
    z-index: 40;
  }

  .clip-video-select-trigger {
    display: inline-flex;
    width: 100%;
    min-width: 0;
    height: 34px;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 10px 0 12px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 8px;
    background: #1c1c1e;
    color: rgba(255, 255, 255, 0.86);
    font-size: 13px;
    line-height: 34px;
    transition:
      border-color 0.16s ease,
      background-color 0.16s ease,
      color 0.16s ease;
  }

  .clip-video-select-trigger:hover,
  .clip-video-select-trigger:focus-visible,
  .clip-video-select-trigger[aria-expanded="true"] {
    border-color: rgba(10, 132, 255, 0.78);
    background: #222225;
    color: #ffffff;
    outline: none;
  }

  .clip-video-select-value {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .clip-video-select-chevron {
    flex: 0 0 auto;
    font-size: 9px;
    line-height: 1;
    color: rgba(255, 255, 255, 0.68);
    transition: transform 0.15s ease;
  }

  .clip-video-select-chevron.is-open {
    transform: rotate(180deg);
  }

  .clip-video-select-menu {
    position: absolute;
    top: calc(100% + 10px);
    left: 0;
    right: 0;
    max-height: 280px;
    overflow-y: auto;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    background: rgba(28, 28, 30, 0.98);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.28);
    backdrop-filter: blur(8px);
  }

  .clip-video-select-item {
    display: block;
    width: 100%;
    min-width: 0;
    padding: 9px 12px;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.78);
    font-size: 12px;
    line-height: 1.4;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .clip-video-select-item:hover,
  .clip-video-select-item:focus-visible {
    background: rgba(255, 255, 255, 0.08);
    color: #ffffff;
    outline: none;
  }

  .clip-video-select-item.is-active {
    color: #00a1d6;
    background: rgba(0, 161, 214, 0.08);
  }

  :global(.clip-web-fullscreen) .clip-top-bar {
    display: none;
  }
</style>
