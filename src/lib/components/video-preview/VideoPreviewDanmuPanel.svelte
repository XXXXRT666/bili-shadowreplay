<script lang="ts">
  import BuiProgressControl from "../BuiProgressControl.svelte";
  import {
    isDanmuFontValueSelected,
    resolveDanmuFontOptionLabel,
    resolveOptionLabel,
  } from "./playback/previewControlsState";
  import type { SeekbarPbpGenerationMethod } from "./timeline/timelinePresentation";

  const danmuEnabledToggleId = `danmu-enabled-${Math.random().toString(36).slice(2)}`;
  const danmuEmotesToggleId = `danmu-emotes-${Math.random().toString(36).slice(2)}`;
  const danmuOcclusionToggleId = `danmu-occlusion-${Math.random().toString(36).slice(2)}`;
  const danmuRateSyncToggleId = `danmu-rate-sync-${Math.random()
    .toString(36)
    .slice(2)}`;
  const danmuBoldToggleId = `danmu-bold-${Math.random().toString(36).slice(2)}`;

  export let danmuEnabled = true;
  export let renderDanmuEmotes = true;
  export let danmuPreventSubtitleOcclusionEnabled = true;
  export let danmuSyncWithPlaybackRateEnabled = true;
  export let danmakuBold = false;
  export let danmakuFontFamily = "";
  export let danmakuDisplayAreaIndex = 0;
  export let danmakuFontScale = 1;
  export let danmakuOpacity = 1;
  export let danmakuSpeedPresetIndex = 0;
  export let danmakuMaxOnScreenIndex = 0;
  export let isDanmuFontMenuVisible = false;
  export let isPbpMethodMenuVisible = false;
  export let seekbarPbpGenerationMethod: SeekbarPbpGenerationMethod = "conv_curve";
  export let danmakuDisplayAreaOptions: readonly number[] = [];
  export let danmakuSpeedPresetOptions: readonly string[] = [];
  export let danmakuMaxOnScreenOptions: readonly number[] = [];
  export let danmakuFontOptions: ReadonlyArray<{ label: string; value: string }> = [];
  export let seekbarPbpMethodOptions: ReadonlyArray<{
    label: string;
    value: SeekbarPbpGenerationMethod;
  }> = [];
  export let danmuFontMenuController: {
    open: () => void;
    scheduleClose: () => void;
  };
  export let pbpMethodMenuController: {
    open: () => void;
    scheduleClose: () => void;
  };
  export let onDanmuToggle: ((event: Event) => void) | undefined = undefined;
  export let onRenderDanmuEmotesToggle: ((event: Event) => void) | undefined = undefined;
  export let onDanmuPreventSubtitleOcclusionToggle:
    | ((event: Event) => void)
    | undefined = undefined;
  export let onDanmuSyncWithPlaybackRateToggle:
    | ((event: Event) => void)
    | undefined = undefined;
  export let onDanmuBoldToggle: ((event: Event) => void) | undefined = undefined;
  export let onToggleDanmuFontMenu: (() => void) | undefined = undefined;
  export let onHandleDanmuFontSelect: ((fontFamily: string) => void) | undefined =
    undefined;
  export let onTogglePbpMethodMenu: (() => void) | undefined = undefined;
  export let onHandleSeekbarPbpMethodSelect:
    | ((method: SeekbarPbpGenerationMethod) => void)
    | undefined = undefined;

  function getDanmuFontOptionLabel(fontFamily: string) {
    return resolveDanmuFontOptionLabel(fontFamily, danmakuFontOptions);
  }

  function isDanmuFontOptionSelected(optionValue: string) {
    return isDanmuFontValueSelected(danmakuFontFamily, optionValue);
  }

  function getSeekbarPbpMethodLabel(method: SeekbarPbpGenerationMethod) {
    return resolveOptionLabel(method, seekbarPbpMethodOptions, "高斯卷积");
  }

  function getDanmakuMaxOnScreenLabel(value: number) {
    return value < 0 ? "无限制" : `${value}`;
  }
</script>

<div class="p-4 space-y-4">
  <div class="p-4 rounded-xl border border-gray-800/50 bg-[#1c1c1e] space-y-4">
    <div class="space-y-1">
      <h3 class="text-sm font-medium text-white">弹幕设置</h3>
      <p class="text-xs text-gray-400 leading-5">
        控制预览里的弹幕显示。
      </p>
    </div>

    <div class="bui bui-switch danmu-switch">
      <div class="bui-area">
        <input
          id={danmuEnabledToggleId}
          class="bui-switch-input"
          type="checkbox"
          aria-label="显示弹幕"
          checked={danmuEnabled}
          on:change={onDanmuToggle}
        />
        <label class="bui-switch-label" for={danmuEnabledToggleId}>
          <span class="bui-switch-name">
            <span class="bui-switch-title">显示弹幕</span>
            <span class="bui-switch-desc">
              关闭后，预览区不再显示弹幕。
            </span>
          </span>
          <span class="bui-switch-body">
            <span class="bui-switch-dot">
              <span></span>
            </span>
          </span>
        </label>
      </div>
    </div>

    <div class="bui bui-switch danmu-switch">
      <div class="bui-area">
        <input
          id={danmuEmotesToggleId}
          class="bui-switch-input"
          type="checkbox"
          aria-label="渲染表情弹幕"
          checked={renderDanmuEmotes}
          disabled={!danmuEnabled}
          on:change={onRenderDanmuEmotesToggle}
        />
        <label class="bui-switch-label" for={danmuEmotesToggleId}>
          <span class="bui-switch-name">
            <span class="bui-switch-title">渲染表情弹幕</span>
            <span class="bui-switch-desc">
              开启后会把可识别的表情片段按图像渲染到预览和导出结果中。
            </span>
          </span>
          <span class="bui-switch-body">
            <span class="bui-switch-dot">
              <span></span>
            </span>
          </span>
        </label>
      </div>
    </div>

    <div class="bui bui-switch danmu-switch">
      <div class="bui-area">
        <input
          id={danmuOcclusionToggleId}
          class="bui-switch-input"
          type="checkbox"
          aria-label="防挡字幕"
          checked={danmuPreventSubtitleOcclusionEnabled}
          disabled={!danmuEnabled}
          on:change={onDanmuPreventSubtitleOcclusionToggle}
        />
        <label class="bui-switch-label" for={danmuOcclusionToggleId}>
          <span class="bui-switch-name">
            <span class="bui-switch-title">防挡字幕</span>
            <span class="bui-switch-desc">视频底部15%部分为空白保留区。</span>
          </span>
          <span class="bui-switch-body">
            <span class="bui-switch-dot">
              <span></span>
            </span>
          </span>
        </label>
      </div>
    </div>

    <div class="bui bui-switch danmu-switch">
      <div class="bui-area">
        <input
          id={danmuRateSyncToggleId}
          class="bui-switch-input"
          type="checkbox"
          aria-label="弹幕速度同步播放倍速"
          checked={danmuSyncWithPlaybackRateEnabled}
          disabled={!danmuEnabled}
          on:change={onDanmuSyncWithPlaybackRateToggle}
        />
        <label class="bui-switch-label" for={danmuRateSyncToggleId}>
          <span class="bui-switch-name">
            <span class="bui-switch-title">弹幕速度同步播放倍速</span>
          </span>
          <span class="bui-switch-body">
            <span class="bui-switch-dot">
              <span></span>
            </span>
          </span>
        </label>
      </div>
    </div>

    <div class="bui bui-switch danmu-switch">
      <div class="bui-area">
        <input
          id={danmuBoldToggleId}
          class="bui-switch-input"
          type="checkbox"
          aria-label="粗体弹幕"
          checked={danmakuBold}
          disabled={!danmuEnabled}
          on:change={onDanmuBoldToggle}
        />
        <label class="bui-switch-label" for={danmuBoldToggleId}>
          <span class="bui-switch-name">
            <span class="bui-switch-title">粗体弹幕</span>
            <span class="bui-switch-desc">开启后弹幕会使用粗体字体渲染。</span>
          </span>
          <span class="bui-switch-body">
            <span class="bui-switch-dot">
              <span></span>
            </span>
          </span>
        </label>
      </div>
    </div>

    <div class="danmu-font-select-row">
      <span class="danmu-font-select-title">弹幕字体</span>
      <div
        class="danmu-font-select relative overflow-visible"
        class:is-open={isDanmuFontMenuVisible}
        on:mouseenter={danmuFontMenuController.open}
        on:mouseleave={danmuFontMenuController.scheduleClose}
      >
        <button
          type="button"
          class="bpx-player-ctrl-btn bpx-player-ctrl-playbackrate danmu-font-select-trigger"
          aria-label="选择弹幕字体"
          aria-expanded={isDanmuFontMenuVisible}
          disabled={!danmuEnabled}
          on:click={onToggleDanmuFontMenu}
          on:blur={danmuFontMenuController.scheduleClose}
        >
          <span class="bpx-player-ctrl-playbackrate-result danmu-font-select-value">
            {getDanmuFontOptionLabel(danmakuFontFamily)}
          </span>
          <span
            class="danmu-font-select-chevron"
            class:is-open={isDanmuFontMenuVisible}
            aria-hidden="true"
          >
            ▼
          </span>
        </button>
        {#if isDanmuFontMenuVisible}
          <div class="bpx-player-ctrl-playbackrate-menu danmu-font-select-menu pointer-events-auto">
            {#each danmakuFontOptions as option}
              <button
                type="button"
                class="bpx-player-ctrl-playbackrate-menu-item danmu-font-select-item"
                class:bpx-state-active={isDanmuFontOptionSelected(option.value)}
                style={`font-family: ${option.value};`}
                on:click={() => onHandleDanmuFontSelect?.(option.value)}
              >
                {option.label}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="danmu-font-select-row">
      <span class="danmu-font-select-title">高能生成</span>
      <div
        class="danmu-font-select pbp-method-select relative overflow-visible"
        class:is-open={isPbpMethodMenuVisible}
        on:mouseenter={pbpMethodMenuController.open}
        on:mouseleave={pbpMethodMenuController.scheduleClose}
      >
        <button
          type="button"
          class="bpx-player-ctrl-btn bpx-player-ctrl-playbackrate danmu-font-select-trigger"
          aria-label="高能进度条生成方式"
          aria-expanded={isPbpMethodMenuVisible}
          on:click|stopPropagation={onTogglePbpMethodMenu}
          on:blur={pbpMethodMenuController.scheduleClose}
        >
          <span class="bpx-player-ctrl-playbackrate-result danmu-font-select-value">
            {getSeekbarPbpMethodLabel(seekbarPbpGenerationMethod)}
          </span>
          <span
            class="danmu-font-select-chevron"
            class:is-open={isPbpMethodMenuVisible}
            aria-hidden="true"
          >
            ▼
          </span>
        </button>
        {#if isPbpMethodMenuVisible}
          <div class="bpx-player-ctrl-playbackrate-menu danmu-font-select-menu pointer-events-auto">
            {#each seekbarPbpMethodOptions as option}
              <button
                type="button"
                class="bpx-player-ctrl-playbackrate-menu-item danmu-font-select-item"
                class:bpx-state-active={option.value === seekbarPbpGenerationMethod}
                on:click={() => onHandleSeekbarPbpMethodSelect?.(option.value)}
              >
                {option.label}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="pt-1 space-y-3">
      <BuiProgressControl
        title="显示区域"
        rootClass="bpx-player-dm-setting-left-area"
        contentClass="bpx-player-dm-setting-left-area-content"
        min={0}
        max={danmakuDisplayAreaOptions.length - 1}
        step={1}
        showSteps={true}
        stepsCount={danmakuDisplayAreaOptions.length}
        bind:value={danmakuDisplayAreaIndex}
        displayValue={`${danmakuDisplayAreaOptions[danmakuDisplayAreaIndex]}%`}
        disabled={!danmuEnabled}
      />
      <BuiProgressControl
        title="弹幕字号"
        rootClass="bpx-player-dm-setting-left-fontsize"
        contentClass="bpx-player-dm-setting-left-fontsize-content"
        min={0.4}
        max={1.6}
        step={0.01}
        bind:value={danmakuFontScale}
        displayValue={`${Math.round(danmakuFontScale * 100)}%`}
        disabled={!danmuEnabled}
      />
      <BuiProgressControl
        title="不透明度"
        rootClass="bpx-player-dm-setting-left-opacity"
        contentClass="bpx-player-dm-setting-left-opacity-content"
        min={0.1}
        max={1}
        step={0.01}
        bind:value={danmakuOpacity}
        displayValue={`${Math.round(danmakuOpacity * 100)}%`}
        disabled={!danmuEnabled}
      />
      <BuiProgressControl
        title="弹幕速度"
        rootClass="bpx-player-dm-setting-left-speedplus"
        contentClass="bpx-player-dm-setting-left-speedplus-content"
        min={0}
        max={danmakuSpeedPresetOptions.length - 1}
        step={1}
        showSteps={true}
        stepsCount={danmakuSpeedPresetOptions.length}
        bind:value={danmakuSpeedPresetIndex}
        displayValue={danmakuSpeedPresetOptions[danmakuSpeedPresetIndex]}
        disabled={!danmuEnabled}
      />
      <BuiProgressControl
        title="弹幕密度"
        rootClass="bpx-player-dm-setting-left-density"
        contentClass="bpx-player-dm-setting-left-density-content"
        min={0}
        max={danmakuMaxOnScreenOptions.length - 1}
        step={1}
        showSteps={true}
        stepsCount={danmakuMaxOnScreenOptions.length}
        bind:value={danmakuMaxOnScreenIndex}
        displayValue={getDanmakuMaxOnScreenLabel(
          danmakuMaxOnScreenOptions[danmakuMaxOnScreenIndex],
        )}
        disabled={!danmuEnabled}
      />
    </div>
  </div>
</div>

<style>
  .bpx-player-ctrl-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    fill: #fff;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0;
    height: 28px;
    line-height: 28px;
    outline: 0;
    position: relative;
    text-align: center;
    width: auto;
    z-index: 2;
    padding: 0 6px;
    border: 0;
    cursor: pointer;
    background: transparent;
    transition:
      color 0.18s ease,
      opacity 0.18s ease,
      transform 0.18s ease;
  }

  .bpx-player-ctrl-btn:hover,
  .bpx-player-ctrl-btn:focus-visible {
    color: #ffffff;
    outline: none;
  }

  .bpx-player-ctrl-playbackrate {
    font-size: 14px;
    width: 50px;
  }

  .bpx-player-ctrl-playbackrate-result {
    cursor: pointer;
    font-weight: 600;
    white-space: nowrap;
    width: 100%;
  }

  .bpx-player-ctrl-playbackrate-menu {
    background-color: rgba(20, 20, 20, 0.9);
    border-radius: 2px;
    bottom: calc(100% + 14px);
    box-sizing: border-box;
    left: 50%;
    margin: 0;
    padding: 0;
    position: absolute;
    text-align: center;
    transform: translateX(-50%);
    width: 70px;
  }

  .bpx-player-ctrl-playbackrate-menu-item {
    cursor: pointer;
    height: 36px;
    line-height: 36px;
    position: relative;
    display: block;
    width: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text1, rgba(255, 255, 255, 0.68));
    font-size: 12px;
    text-align: center;
  }

  .bpx-player-ctrl-playbackrate-menu-item:hover,
  .bpx-player-ctrl-playbackrate-menu-item:focus-visible {
    background: rgba(255, 255, 255, 0.1);
    color: #ffffff;
    outline: none;
  }

  .bpx-player-ctrl-playbackrate-menu-item.bpx-state-active {
    color: var(--bpx-primary-color, #00a1d6);
  }

  .bui-switch .bui-switch-input {
    cursor: pointer;
    height: 100%;
    left: 0;
    margin: 0;
    opacity: 0;
    position: absolute;
    top: 0;
    width: 100%;
    z-index: 1;
  }

  .bui-switch {
    position: relative;
    display: block;
  }

  .bui-switch .bui-area {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .bui-switch-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    cursor: pointer;
    color: #ffffff;
    user-select: none;
  }

  .bui-switch-name {
    font-size: 12px;
    color: #ffffff;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bui-switch-body {
    position: relative;
    width: 28px;
    height: 16px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.18);
    flex: 0 0 auto;
    transition: background-color 0.15s ease-in-out;
  }

  .bui-switch-dot {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #ffffff;
    transition: transform 0.15s ease-in-out;
  }

  .bui-switch-dot > span {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: 50%;
  }

  .bui-switch-input:checked + .bui-switch-label .bui-switch-body {
    background: #00a1d6;
  }

  .bui-switch-input:checked + .bui-switch-label .bui-switch-dot {
    transform: translateX(12px);
  }

  .bui-switch-input:disabled + .bui-switch-label {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .bui-switch-input:focus-visible + .bui-switch-label {
    outline: 1px solid rgba(255, 255, 255, 0.6);
    outline-offset: 2px;
    border-radius: 6px;
  }

  .danmu-switch {
    padding: 6px 0;
  }

  .danmu-switch .bui-switch-label {
    align-items: flex-start;
  }

  .danmu-switch .bui-switch-name {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 14px;
    white-space: normal;
  }

  .danmu-switch .bui-switch-title {
    font-size: 14px;
    color: #ffffff;
  }

  .danmu-switch .bui-switch-desc {
    font-size: 12px;
    color: #9ca3af;
    line-height: 1.4;
  }

  .danmu-switch .bui-switch-body {
    margin-top: 2px;
  }

  .danmu-font-select-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 4px 0 6px;
  }

  .danmu-font-select {
    position: relative;
    flex: 0 0 auto;
    z-index: 60;
  }

  .danmu-font-select.is-open {
    z-index: 80;
  }

  .danmu-font-select-title {
    font-size: 14px;
    color: #ffffff;
    line-height: 1.2;
  }

  .danmu-font-select-trigger {
    width: 82px;
    height: 28px;
    padding: 0 6px 0 8px;
    border: 1px solid rgba(255, 255, 255, 0.26);
    border-radius: 4px;
    background: rgba(20, 20, 20, 0.9);
    transition:
      border-color 0.16s ease,
      background-color 0.16s ease;
  }

  .danmu-font-select-trigger:hover,
  .danmu-font-select-trigger:focus-visible,
  .danmu-font-select-trigger[aria-expanded="true"] {
    border-color: rgba(0, 161, 214, 0.85);
    background: rgba(28, 28, 28, 0.95);
  }

  .danmu-font-select-trigger:focus-visible {
    outline: none;
  }

  .danmu-font-select-trigger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .danmu-font-select-value {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    padding-right: 4px;
  }

  .danmu-font-select-chevron {
    flex: 0 0 auto;
    font-size: 9px;
    line-height: 1;
    color: rgba(255, 255, 255, 0.72);
    transform-origin: center;
    transition: transform 0.15s ease;
  }

  .danmu-font-select-chevron.is-open {
    transform: rotate(180deg);
  }

  .danmu-font-select-menu {
    top: calc(100% + 10px);
    bottom: auto;
    width: 92px;
    max-height: 190px;
    overflow-y: auto;
  }

  .danmu-font-select-item {
    height: 32px;
    line-height: 32px;
    font-size: 12px;
  }

  .danmu-font-select-menu::-webkit-scrollbar {
    width: 6px;
  }

  .danmu-font-select-menu::-webkit-scrollbar-thumb {
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.24);
  }

  .danmu-font-select-menu::-webkit-scrollbar-track {
    background: transparent;
  }
</style>
