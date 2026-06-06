<script lang="ts">
  import { X } from "lucide-svelte";
  import BuiProgressControl from "./BuiProgressControl.svelte";
  import {
    buildSubtitlePreviewTextShadow,
    parseSubtitleStyle,
    type SubtitleStyle,
  } from "../interface";

  export let show = false;
  export let onClose: () => void;
  export let roomId: string;

  // 默认样式
  const defaultStyle: SubtitleStyle = {
    fontName: "Arial",
    fontSize: 40,
    fontWeight: 700,
    fontColor: "#FFFFFF",
    outlineColor: "#000000",
    outlineWidth: 2,
    alignment: 2,
    marginV: -20,
    marginL: 20,
    marginR: 20,
  };

  // 从 localStorage 加载样式，如果没有则使用默认值
  let style: SubtitleStyle = (() => {
    const savedStyle = localStorage.getItem(`subtitle_style_${roomId}`);
    return savedStyle ? { ...defaultStyle, ...JSON.parse(savedStyle) } : defaultStyle;
  })();

  // 保存样式到 localStorage
  function saveStyle() {
    localStorage.setItem(`subtitle_style_${roomId}`, JSON.stringify(style));
  }

  // 生成 ffmpeg 样式参数
  $: styleString = parseSubtitleStyle(style);

  // 预览样式
  $: previewStyle = (() => {
    // 将颜色值转换为 rgba 格式
    const fontColor = style.fontColor.startsWith("#")
      ? `rgba(${parseInt(style.fontColor.slice(1, 3), 16)}, ${parseInt(style.fontColor.slice(3, 5), 16)}, ${parseInt(style.fontColor.slice(5, 7), 16)}, 1)`
      : style.fontColor;

    const outlineColor = style.outlineColor.startsWith("#")
      ? `rgba(${parseInt(style.outlineColor.slice(1, 3), 16)}, ${parseInt(style.outlineColor.slice(3, 5), 16)}, ${parseInt(style.outlineColor.slice(5, 7), 16)}, 1)`
      : style.outlineColor;

    return `
      font-family: ${style.fontName};
      font-size: ${style.fontSize}px;
      font-weight: ${style.fontWeight};
      color: ${fontColor};
      text-shadow: ${buildSubtitlePreviewTextShadow(style.outlineWidth, outlineColor)};
      padding: 4px 8px;
      border-radius: 4px;
      margin: ${style.marginV}px ${style.marginR}px ${style.marginV}px ${style.marginL}px;
      display: inline-block;
      white-space: nowrap;
    `;
  })();

  // 对齐方式选项
  const alignmentOptions = [
    { value: 1, label: "左下", description: "字幕显示在视频左下角" },
    { value: 2, label: "底部居中", description: "字幕显示在视频底部中间" },
    { value: 3, label: "右下", description: "字幕显示在视频右下角" },
    { value: 5, label: "左上", description: "字幕显示在视频左上角" },
    { value: 6, label: "顶部居中", description: "字幕显示在视频顶部中间" },
    { value: 7, label: "右上", description: "字幕显示在视频右上角" },
    { value: 9, label: "中间居左", description: "字幕显示在视频中间偏左" },
    { value: 10, label: "中间居中", description: "字幕显示在视频中间" },
    { value: 11, label: "中间居右", description: "字幕显示在视频中间偏右" },
  ];

  const subtitleFontOptions = [
    { label: "Arial", value: "Arial" },
    { label: "黑体", value: "SimHei" },
    { label: "微软雅黑", value: "Microsoft YaHei" },
    { label: "苹方", value: "PingFang SC" },
    { label: "冬青黑体", value: "Hiragino Sans GB" },
    { label: "思源黑体", value: "Noto Sans CJK SC" },
    { label: "宋体", value: "SimSun" },
    { label: "仿宋", value: "FangSong" },
    { label: "楷体", value: "KaiTi" },
    { label: "Times New Roman", value: "Times New Roman" },
  ] as const;

  let isSubtitleFontMenuVisible = false;
  let subtitleFontMenuHideTimer: number | null = null;

  function normalizeSubtitleFontValue(value: string) {
    const normalized = value.trim().toLowerCase();
    if (normalized.includes("simhei")) {
      return "__simhei__";
    }
    return normalized;
  }

  function getSubtitleFontLabel(fontName: string) {
    const normalized = normalizeSubtitleFontValue(fontName);
    const matched = subtitleFontOptions.find(
      (option) => normalizeSubtitleFontValue(option.value) === normalized
    );
    return matched?.label ?? (fontName || "字体");
  }

  function isSubtitleFontOptionSelected(fontName: string) {
    return normalizeSubtitleFontValue(style.fontName) === normalizeSubtitleFontValue(fontName);
  }

  function cancelSubtitleFontMenuHide() {
    if (subtitleFontMenuHideTimer !== null && typeof window !== "undefined") {
      window.clearTimeout(subtitleFontMenuHideTimer);
      subtitleFontMenuHideTimer = null;
    }
  }

  function closeSubtitleFontMenu() {
    cancelSubtitleFontMenuHide();
    isSubtitleFontMenuVisible = false;
  }

  function toggleSubtitleFontMenu() {
    if (isSubtitleFontMenuVisible) {
      closeSubtitleFontMenu();
      return;
    }
    cancelSubtitleFontMenuHide();
    isSubtitleFontMenuVisible = true;
  }

  function scheduleSubtitleFontMenuClose() {
    cancelSubtitleFontMenuHide();
    if (typeof window === "undefined") {
      isSubtitleFontMenuVisible = false;
      return;
    }
    subtitleFontMenuHideTimer = window.setTimeout(() => {
      subtitleFontMenuHideTimer = null;
      isSubtitleFontMenuVisible = false;
    }, 100);
  }

  function handleSubtitleFontSelect(fontName: string) {
    if (!fontName) {
      return;
    }
    style.fontName = fontName;
    closeSubtitleFontMenu();
  }

  function handleWindowClick(event: MouseEvent) {
    if (!isSubtitleFontMenuVisible) {
      return;
    }

    const target = event.target as HTMLElement | null;
    if (!target || target.closest(".subtitle-font-select")) {
      return;
    }

    closeSubtitleFontMenu();
  }

  function handleClose() {
    closeSubtitleFontMenu();
    saveStyle();
    onClose();
  }
</script>

<svelte:window on:click={handleWindowClick} />

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div
    class="fixed inset-0 bg-black/50 z-[1100] flex items-center justify-center"
    on:click|self={handleClose}
  >
    <div
      class="bg-[#1c1c1e] rounded-lg w-[600px] max-h-[80vh] overflow-y-auto sidebar-scrollbar"
    >
      <!-- 顶部标题栏 -->
      <div
        class="flex items-center justify-between p-4 border-b border-gray-800/50"
      >
        <h2 class="text-lg font-medium text-white">字幕压制样式设置</h2>
        <button
          class="text-gray-400 hover:text-white transition-colors duration-200"
          on:click={handleClose}
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- 内容区域 -->
      <div class="p-4 space-y-6">
        <!-- 字体设置 -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300">字体设置</h3>
          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-2">
              <!-- svelte-ignore a11y-label-has-associated-control -->
              <label class="block text-sm text-gray-400">字体名称</label>
              <div class="subtitle-font-select">
                <button
                  type="button"
                  class="subtitle-font-select-trigger"
                  aria-label="选择字幕字体"
                  aria-expanded={isSubtitleFontMenuVisible}
                  on:click={toggleSubtitleFontMenu}
                  on:blur={scheduleSubtitleFontMenuClose}
                >
                  <span class="subtitle-font-select-value">
                    {getSubtitleFontLabel(style.fontName)}
                  </span>
                  <span
                    class="subtitle-font-select-chevron"
                    class:is-open={isSubtitleFontMenuVisible}
                    aria-hidden="true"
                  >
                    ▼
                  </span>
                </button>
                {#if isSubtitleFontMenuVisible}
                  <div class="subtitle-font-select-menu">
                    {#each subtitleFontOptions as option}
                      <button
                        type="button"
                        class="subtitle-font-select-item"
                        class:bpx-state-active={isSubtitleFontOptionSelected(option.value)}
                        style={`font-family: ${option.value};`}
                        on:click={() => handleSubtitleFontSelect(option.value)}
                      >
                        {option.label}
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
            <div class="space-y-2">
              <!-- svelte-ignore a11y-label-has-associated-control -->
              <label class="block text-sm text-gray-400">字体大小</label>
              <input
                type="number"
                bind:value={style.fontSize}
                class="w-full px-3 py-2 bg-[#2c2c2e] text-white rounded-lg
                       border border-gray-800/50 focus:border-[#0A84FF]
                       transition duration-200 outline-none hover:border-gray-700/50"
              />
            </div>
          </div>
          <div class="space-y-2">
            <BuiProgressControl
              title="字体粗细"
              rootClass="is-title-wide"
              min={100}
              max={1000}
              step={100}
              bind:value={style.fontWeight}
              displayValue={`${style.fontWeight}`}
            />
          </div>
        </div>

        <!-- 颜色设置 -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300">颜色设置</h3>
          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-2">
              <!-- svelte-ignore a11y-label-has-associated-control -->
              <label class="block text-sm text-gray-400">字体颜色</label>
              <input
                type="color"
                bind:value={style.fontColor}
                class="w-full h-10 bg-[#2c2c2e] rounded-lg
                       border border-gray-800/50 focus:border-[#0A84FF]
                       transition duration-200 outline-none hover:border-gray-700/50"
              />
            </div>
            <div class="space-y-2">
              <!-- svelte-ignore a11y-label-has-associated-control -->
              <label class="block text-sm text-gray-400">描边颜色</label>
              <input
                type="color"
                bind:value={style.outlineColor}
                class="w-full h-10 bg-[#2c2c2e] rounded-lg
                       border border-gray-800/50 focus:border-[#0A84FF]
                       transition duration-200 outline-none hover:border-gray-700/50"
              />
            </div>
          </div>
        </div>

        <!-- 描边设置 -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300">描边设置</h3>
          <div class="space-y-2">
            <BuiProgressControl
              title="描边宽度"
              rootClass="is-title-wide"
              min={0}
              max={3}
              step={0.1}
              showSteps={true}
              stepsCount={4}
              bind:value={style.outlineWidth}
              displayValue={`${style.outlineWidth.toFixed(1)}px`}
            />
          </div>
        </div>

        <!-- 对齐和边距设置 -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300">对齐和边距</h3>
          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-2">
              <label for="alignment-select" class="block text-sm text-gray-400"
                >对齐方式</label
              >
              <select
                id="alignment-select"
                bind:value={style.alignment}
                class="w-full px-3 py-2 bg-[#2c2c2e] text-white rounded-lg
                       border border-gray-800/50 focus:border-[#0A84FF]
                       transition duration-200 outline-none hover:border-gray-700/50"
              >
                {#each alignmentOptions as option}
                  <option value={option.value} title={option.description}>
                    {option.label}
                  </option>
                {/each}
              </select>
            </div>
            <div class="space-y-2">
              <label for="margin-v-input" class="block text-sm text-gray-400"
                >垂直边距</label
              >
              <input
                id="margin-v-input"
                type="number"
                bind:value={style.marginV}
                class="w-full px-3 py-2 bg-[#2c2c2e] text-white rounded-lg
                       border border-gray-800/50 focus:border-[#0A84FF]
                       transition duration-200 outline-none hover:border-gray-700/50"
              />
            </div>
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-2">
              <label for="margin-l-input" class="block text-sm text-gray-400"
                >左边距</label
              >
              <input
                id="margin-l-input"
                type="number"
                bind:value={style.marginL}
                class="w-full px-3 py-2 bg-[#2c2c2e] text-white rounded-lg
                       border border-gray-800/50 focus:border-[#0A84FF]
                       transition duration-200 outline-none hover:border-gray-700/50"
              />
            </div>
            <div class="space-y-2">
              <label for="margin-r-input" class="block text-sm text-gray-400"
                >右边距</label
              >
              <input
                id="margin-r-input"
                type="number"
                bind:value={style.marginR}
                class="w-full px-3 py-2 bg-[#2c2c2e] text-white rounded-lg
                       border border-gray-800/50 focus:border-[#0A84FF]
                       transition duration-200 outline-none hover:border-gray-700/50"
              />
            </div>
          </div>
        </div>

        <!-- 预览区域 -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300">预览</h3>
          <div class="bg-black p-4 rounded-lg flex items-center justify-center">
            <div style={previewStyle}>这是一段示例字幕文本</div>
          </div>
        </div>

        <!-- FFmpeg 样式字符串 -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300">FFmpeg 样式字符串</h3>
          <div class="p-3 bg-[#2c2c2e] rounded-lg">
            <code class="text-sm text-gray-300 break-all">{styleString}</code>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .subtitle-font-select {
    position: relative;
    width: 100%;
  }

  .subtitle-font-select-trigger {
    width: 100%;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 11px;
    border: 1px solid rgba(255, 255, 255, 0.22);
    border-radius: 8px;
    background: rgba(44, 44, 46, 1);
    color: rgba(255, 255, 255, 0.9);
    font-size: 14px;
    line-height: 40px;
    text-align: left;
    transition:
      border-color 0.16s ease,
      background-color 0.16s ease;
  }

  .subtitle-font-select-trigger:hover,
  .subtitle-font-select-trigger:focus-visible,
  .subtitle-font-select-trigger[aria-expanded="true"] {
    border-color: rgba(10, 132, 255, 0.9);
    background: rgba(52, 52, 54, 1);
    outline: none;
  }

  .subtitle-font-select-value {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .subtitle-font-select-chevron {
    flex: 0 0 auto;
    font-size: 10px;
    line-height: 1;
    color: rgba(255, 255, 255, 0.72);
    transition: transform 0.15s ease;
    transform-origin: center;
  }

  .subtitle-font-select-chevron.is-open {
    transform: rotate(180deg);
  }

  .subtitle-font-select-menu {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    width: 100%;
    max-height: 220px;
    overflow-y: auto;
    padding: 4px 0;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    background: rgba(20, 20, 20, 0.96);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.32);
    z-index: 80;
  }

  .subtitle-font-select-item {
    width: 100%;
    height: 34px;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.78);
    font-size: 13px;
    line-height: 34px;
    text-align: left;
    padding: 0 11px;
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .subtitle-font-select-item:hover,
  .subtitle-font-select-item:focus-visible {
    background: rgba(255, 255, 255, 0.1);
    color: #ffffff;
    outline: none;
  }

  .subtitle-font-select-item.bpx-state-active {
    color: #00a1d6;
  }

  .subtitle-font-select-menu::-webkit-scrollbar {
    width: 6px;
  }

  .subtitle-font-select-menu::-webkit-scrollbar-thumb {
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.24);
  }

  .subtitle-font-select-menu::-webkit-scrollbar-track {
    background: transparent;
  }
</style>
