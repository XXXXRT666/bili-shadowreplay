<script lang="ts">
  import { X } from "lucide-svelte";
  import BuiProgressControl from "./BuiProgressControl.svelte";
  import type { DanmakuStyle } from "../interface";
  import {
    DANMAKU_FONT_FAMILY,
    DANMAKU_FONT_SIZE_PX,
    DANMAKU_FONT_WEIGHT,
    DANMAKU_LINE_HEIGHT,
    DANMAKU_TEXT_SHADOW,
    defaultDanmakuStyle,
    saveDanmakuStyle,
  } from "../danmaku-emotes";

  export let show = false;
  export let onClose: () => void;
  export let roomId: string;
  export let style: DanmakuStyle = defaultDanmakuStyle();

  const displayAreaOptions = [10, 25, 50, 75, 100] as const;
  const speedOptions = ["极慢", "较慢", "适中", "较快", "极快"] as const;
  const speedRateHints = [0.6, 0.8, 1, 1.2, 1.4] as const;

  let displayAreaIndex = 4;
  let speedPresetIndex = 2;

  $: {
    const nextIndex = displayAreaOptions.indexOf(style.displayArea);
    if (nextIndex >= 0) {
      displayAreaIndex = nextIndex;
    }
  }

  $: {
    const clampedIndex = Math.max(
      0,
      Math.min(displayAreaOptions.length - 1, Math.round(displayAreaIndex))
    );
    const mappedValue = displayAreaOptions[clampedIndex];
    if (style.displayArea !== mappedValue) {
      style.displayArea = mappedValue;
    }
    if (displayAreaIndex !== clampedIndex) {
      displayAreaIndex = clampedIndex;
    }
  }

  $: {
    const nextIndex = Math.max(
      0,
      Math.min(speedOptions.length - 1, Math.round(style.speedPreset))
    );
    if (speedPresetIndex !== nextIndex) {
      speedPresetIndex = nextIndex;
    }
  }

  $: {
    const clampedIndex = Math.max(
      0,
      Math.min(speedOptions.length - 1, Math.round(speedPresetIndex))
    );
    if (style.speedPreset !== clampedIndex) {
      style.speedPreset = clampedIndex as DanmakuStyle["speedPreset"];
    }
    if (speedPresetIndex !== clampedIndex) {
      speedPresetIndex = clampedIndex;
    }
  }

  $: previewFontSize = `${DANMAKU_FONT_SIZE_PX * style.fontScale}px`;

  function handleClose() {
    saveDanmakuStyle(roomId, style);
    onClose();
  }
</script>

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div
    class="fixed inset-0 bg-black/50 z-[1100] flex items-center justify-center"
    on:click|self={handleClose}
  >
    <div
      class="bg-[#1c1c1e] rounded-lg w-[600px] max-h-[80vh] overflow-y-auto sidebar-scrollbar"
    >
      <div
        class="flex items-center justify-between p-4 border-b border-gray-800/50"
      >
        <h2 class="text-lg font-medium text-white">弹幕样式设置</h2>
        <button
          class="text-gray-400 hover:text-white transition-colors duration-200"
          on:click={handleClose}
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <div class="p-4 space-y-6">
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300">显示设置</h3>
          <div
            class="p-3 rounded-lg border border-gray-800/70 bg-[#232326] space-y-3"
          >
            <BuiProgressControl
              title="显示区域"
              rootClass="bpx-player-dm-setting-left-area"
              contentClass="bpx-player-dm-setting-left-area-content"
              min={0}
              max={displayAreaOptions.length - 1}
              step={1}
              showSteps={true}
              stepsCount={displayAreaOptions.length}
              bind:value={displayAreaIndex}
              displayValue={`${displayAreaOptions[displayAreaIndex]}%`}
            />
            <BuiProgressControl
              title="弹幕字号"
              rootClass="bpx-player-dm-setting-left-fontsize"
              contentClass="bpx-player-dm-setting-left-fontsize-content"
              min={0.4}
              max={1.6}
              step={0.01}
              bind:value={style.fontScale}
              displayValue={`${Math.round(style.fontScale * 100)}%`}
            />
            <BuiProgressControl
              title="不透明度"
              rootClass="bpx-player-dm-setting-left-opacity"
              contentClass="bpx-player-dm-setting-left-opacity-content"
              min={0.1}
              max={1}
              step={0.01}
              bind:value={style.opacity}
              displayValue={`${Math.round(style.opacity * 100)}%`}
            />
            <BuiProgressControl
              title="弹幕速度"
              rootClass="bpx-player-dm-setting-left-speedplus"
              contentClass="bpx-player-dm-setting-left-speedplus-content"
              min={0}
              max={speedOptions.length - 1}
              step={1}
              showSteps={true}
              stepsCount={speedOptions.length}
              bind:value={speedPresetIndex}
              displayValue={speedOptions[speedPresetIndex]}
            />
          </div>
        </div>

        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300">预览</h3>
          <div class="bg-black p-4 rounded-lg flex items-center justify-center">
            <div
              style={`
                display: flex;
                align-items: center;
                opacity: ${style.opacity};
                color: #ffffff;
                font-family: ${DANMAKU_FONT_FAMILY};
                font-size: ${previewFontSize};
                font-weight: ${DANMAKU_FONT_WEIGHT};
                line-height: ${DANMAKU_LINE_HEIGHT};
                white-space: pre;
                text-shadow: ${DANMAKU_TEXT_SHADOW};
              `}
            >
              这是一条弹幕预览
            </div>
          </div>
          <p class="text-xs text-gray-400">
            当前实际字号: <span class="font-mono">{previewFontSize}</span>
            <span class="mx-2 text-gray-600">|</span>
            当前速度: <span class="font-mono">{speedOptions[speedPresetIndex]}</span>
            <span class="text-gray-500">({speedRateHints[speedPresetIndex]}x)</span>
          </p>
        </div>
      </div>
    </div>
  </div>
{/if}
