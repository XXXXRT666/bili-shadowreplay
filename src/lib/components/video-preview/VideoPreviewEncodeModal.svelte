<script lang="ts">
  import { CheckSquare, Square } from "lucide-svelte";

  export let show = false;
  export let title = "确认压制";
  export let description = "选择需要烧录进视频画面的内容。";
  export let confirmLabel = "确认";
  export let requireSelection = true;
  export let includeSubtitle = true;
  export let includeDanmu = false;
  export let onCancel: (() => void) | undefined = undefined;
  export let onConfirm: (() => void) | undefined = undefined;

  $: canConfirm = !requireSelection || includeSubtitle || includeDanmu;

  function toggleSubtitle() {
    includeSubtitle = !includeSubtitle;
  }

  function toggleDanmu() {
    includeDanmu = !includeDanmu;
  }
</script>

{#if show}
  <div class="fixed inset-0 bg-black/50 z-[1100] flex items-center justify-center">
    <div class="bg-[#2c2c2e] rounded-lg shadow-xl w-[360px] max-w-[90vw]">
      <div
        class="px-4 py-3 border-b border-gray-800/50 flex items-center justify-between"
      >
        <h3 class="text-sm font-medium text-gray-200">{title}</h3>
        <button
          class="text-gray-400 hover:text-white transition-colors duration-200"
          on:click={onCancel}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-4 w-4"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <div class="p-4 text-gray-300 space-y-4">
        <p class="text-xs leading-5 text-gray-400">{description}</p>

        <div class="space-y-2">
          <div
            class="rounded-xl border border-gray-800/50 bg-[#1c1c1e] p-3 transition-colors duration-200"
            class:border-[#0A84FF]={includeSubtitle}
            class:bg-[#242426]={includeSubtitle}
          >
            <div class="flex items-center gap-3">
              <button
                class="text-[#0A84FF] hover:text-[#0A84FF]/80"
                aria-label={includeSubtitle ? "取消字幕" : "选择字幕"}
                on:click={toggleSubtitle}
              >
                {#if includeSubtitle}
                  <CheckSquare class="w-5 h-5" />
                {:else}
                  <Square class="w-5 h-5" />
                {/if}
              </button>
              <button
                class="min-w-0 flex-1 text-left text-sm font-medium text-white hover:text-[#0A84FF]"
                on:click={toggleSubtitle}
              >
                字幕
              </button>
            </div>
          </div>

          <div
            class="rounded-xl border border-gray-800/50 bg-[#1c1c1e] p-3 transition-colors duration-200"
            class:border-[#0A84FF]={includeDanmu}
            class:bg-[#242426]={includeDanmu}
          >
            <div class="flex items-center gap-3">
              <button
                class="text-[#0A84FF] hover:text-[#0A84FF]/80"
                aria-label={includeDanmu ? "取消弹幕" : "选择弹幕"}
                on:click={toggleDanmu}
              >
                {#if includeDanmu}
                  <CheckSquare class="w-5 h-5" />
                {:else}
                  <Square class="w-5 h-5" />
                {/if}
              </button>
              <button
                class="min-w-0 flex-1 text-left text-sm font-medium text-white hover:text-[#0A84FF]"
                on:click={toggleDanmu}
              >
                弹幕
              </button>
            </div>
          </div>
        </div>

        {#if requireSelection && !canConfirm}
          <p class="text-xs text-red-400">请至少选择字幕或弹幕。</p>
        {/if}
      </div>

      <div
        class="px-4 py-3 border-t border-gray-800/50 flex justify-end space-x-2"
      >
        <button
          class="px-4 py-1.5 text-sm bg-gray-700/50 text-gray-200 rounded-md hover:bg-gray-700/70 transition-colors duration-200 border border-gray-600/50"
          on:click={onCancel}
        >
          取消
        </button>
        <button
          class="px-4 py-1.5 text-sm bg-[#0A84FF] text-white rounded-md hover:bg-[#0A84FF]/90 transition-colors duration-200 disabled:opacity-50 disabled:cursor-not-allowed flex items-center space-x-1"
          on:click={onConfirm}
          disabled={!canConfirm}
        >
          <span>{confirmLabel}</span>
        </button>
      </div>
    </div>
  </div>
{/if}
