<script lang="ts">
  import { BrainCircuit, Eraser, Minus, Plus, Settings } from "lucide-svelte";

  type SubtitlePanelItem = {
    startTime: number;
    endTime: number;
    text: string;
  };

  export let subtitles: SubtitlePanelItem[] = [];
  export let currentSubtitleIndex = -1;
  export let currentGenerateEventId: string | null = null;
  export let formatTime: (time: number) => string;
  export let onOpenStyleEditor: (() => void) | undefined = undefined;
  export let onClearSubtitles: (() => void | Promise<void>) | undefined = undefined;
  export let onGenerateSubtitles: (() => void | Promise<void>) | undefined = undefined;
  export let onAddSubtitle: (() => void | Promise<void>) | undefined = undefined;
  export let onSeekToTime: ((time: number) => void) | undefined = undefined;
  export let onAdjustTime:
    | ((index: number, isStart: boolean, delta: number) => void)
    | undefined = undefined;
  export let onRemoveSubtitle:
    | ((index: number) => void | Promise<void>)
    | undefined = undefined;

  let subtitleElements: Array<HTMLElement | null> = [];

  $: if (currentSubtitleIndex >= 0 && subtitleElements[currentSubtitleIndex]) {
    subtitleElements[currentSubtitleIndex]?.scrollIntoView({
      behavior: "smooth",
      block: "nearest",
    });
  }
</script>

<div class="p-4 space-y-4">
  <div class="w-full sticky top-12 bg-[#2c2c2e] z-10 pb-4">
    <div class="flex flex-col space-y-2">
      <div class="flex space-x-2">
        <button
          class="flex-1 px-3 py-1.5 text-sm bg-[#1c1c1e] text-gray-300 rounded-lg hover:bg-[#2c2c2e] transition-colors duration-200 flex items-center justify-center space-x-1 border border-gray-700"
          on:click={onOpenStyleEditor}
        >
          <Settings class="w-4 h-4" />
          <span>字幕样式</span>
        </button>
        <button
          class="flex-1 px-3 py-1.5 text-sm bg-[#1c1c1e] text-gray-300 rounded-lg hover:bg-[#2c2c2e] transition-colors duration-200 flex items-center justify-center space-x-1 border border-gray-700"
          on:click={onClearSubtitles}
        >
          <Eraser class="w-4 h-4" />
          <span>清空列表</span>
        </button>
      </div>
      <div class="flex space-x-2">
        <button
          class="flex-1 px-3 py-1.5 text-sm bg-[#1c1c1e] text-gray-300 rounded-lg hover:bg-[#2c2c2e] transition-colors duration-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center space-x-1 border border-gray-700"
          on:click={onGenerateSubtitles}
          disabled={currentGenerateEventId !== null}
        >
          {#if currentGenerateEventId !== null}
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
            <BrainCircuit class="w-4 h-4" />
          {/if}
          <span id="generate-prompt">AI 生成字幕</span>
        </button>
        <button
          class="flex-1 px-3 py-1.5 text-sm bg-[#1c1c1e] text-gray-300 rounded-lg hover:bg-[#2c2c2e] transition-colors duration-200 flex items-center justify-center space-x-1 border border-gray-700"
          on:click={onAddSubtitle}
        >
          <Plus class="w-4 h-4" />
          <span>手动添加</span>
        </button>
      </div>
    </div>
  </div>

  <div class="space-y-2">
    {#each subtitles as subtitle, index}
      <div
        bind:this={subtitleElements[index]}
        class="p-3 bg-[#1c1c1e] rounded-lg space-y-2 transition-colors duration-200"
        class:bg-[#2c2c2e]={currentSubtitleIndex === index}
        class:border={currentSubtitleIndex === index}
        class:border-[#0A84FF]={currentSubtitleIndex === index}
      >
        <div class="flex justify-between items-center">
          <div class="flex items-center space-x-4">
            <div class="flex items-center space-x-1">
              <button
                class="text-sm text-[#0A84FF] hover:text-[#0A84FF]/80"
                on:click={() => onSeekToTime?.(subtitle.startTime)}
              >
                {formatTime(subtitle.startTime)}
              </button>
              <button
                class="p-0.5 text-gray-400 hover:text-white"
                on:click={() => onAdjustTime?.(index, true, -0.1)}
              >
                <Minus class="w-3 h-3" />
              </button>
              <button
                class="p-0.5 text-gray-400 hover:text-white"
                on:click={() => onAdjustTime?.(index, true, 0.1)}
              >
                <Plus class="w-3 h-3" />
              </button>
            </div>
            <span class="text-gray-400">→</span>
            <div class="flex items-center space-x-1">
              <button
                class="text-sm text-[#0A84FF] hover:text-[#0A84FF]/80"
                on:click={() => onSeekToTime?.(subtitle.endTime)}
              >
                {formatTime(subtitle.endTime)}
              </button>
              <button
                class="p-0.5 text-gray-400 hover:text-white"
                on:click={() => onAdjustTime?.(index, false, -0.1)}
              >
                <Minus class="w-3 h-3" />
              </button>
              <button
                class="p-0.5 text-gray-400 hover:text-white"
                on:click={() => onAdjustTime?.(index, false, 0.1)}
              >
                <Plus class="w-3 h-3" />
              </button>
            </div>
          </div>
          <button
            class="text-sm text-red-500 hover:text-red-400"
            on:click={() => onRemoveSubtitle?.(index)}
          >
            删除
          </button>
        </div>
        <input
          type="text"
          bind:value={subtitle.text}
          class="w-full px-3 py-2 bg-[#2c2c2e] text-white rounded-lg border border-gray-800/50 focus:border-[#0A84FF] transition duration-200 outline-none hover:border-gray-700/50"
          placeholder="输入字幕文本"
        />
      </div>
    {/each}
  </div>
</div>
