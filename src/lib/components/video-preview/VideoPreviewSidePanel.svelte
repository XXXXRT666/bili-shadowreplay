<script lang="ts">
  import type { DanmakuStyle, Profile, VideoItem } from "../../interface";
  import type { SeekbarPbpGenerationMethod } from "./timeline/timelinePresentation";
  import type { ClipSelection } from "./clip/clipSelectionRuntime";
  import VideoPreviewClipSelectionPanel from "./VideoPreviewClipSelectionPanel.svelte";
  import VideoPreviewDanmuPanel from "./VideoPreviewDanmuPanel.svelte";
  import VideoPreviewSubtitlePanel from "./VideoPreviewSubtitlePanel.svelte";
  import VideoPreviewUploadPanel from "./VideoPreviewUploadPanel.svelte";

  export let activeTab = "subtitle";
  export let subtitles = [];
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
  export let clipSelections: ClipSelection[] = [];
  export let activeClipSelectionId: string | null = null;
  export let clipExportSelectionIds: string[] = [];
  export let mergeClipSelectionsOnExport = true;
  export let onSetActiveClipSelection:
    | ((id: string | null) => void)
    | undefined = undefined;

  export let video: VideoItem;
  export let profile: Profile;
  export let uidSelected = 0;
  export let accounts: Array<{ value: number; name: string }> = [];
  export let currentPostEventId: string | null = null;
  export let showCoverEditor = false;
  export let showDefaultCoverIcon = false;
  export let handleCoverError: ((event: Event) => void) | undefined = undefined;
  export let doPost: (() => void) | undefined = undefined;
  export let cancelPost: (() => void) | undefined = undefined;

  export let danmuEnabled = true;
  export let renderDanmuEmotes = true;
  export let danmuPreventSubtitleOcclusionEnabled = true;
  export let danmuSyncWithPlaybackRateEnabled = true;
  export let danmakuStyle: DanmakuStyle;
  export let danmakuDisplayAreaIndex = 0;
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
  export let onRenderDanmuEmotesToggle: ((event: Event) => void) | undefined =
    undefined;
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
</script>

<div
  class="clip-side-panel w-80 shrink-0 border-l border-gray-800/50 bg-[#2c2c2e] overflow-y-auto sidebar-scrollbar"
>
  <div class="sticky top-0 z-20 flex border-b border-gray-800/50 bg-[#1c1c1e]">
    <button
      class="flex-1 px-3 py-3 text-sm font-medium transition-all duration-200 relative"
      class:text-white={activeTab === "subtitle"}
      class:text-gray-400={activeTab !== "subtitle"}
      class:bg-[#2c2c2e]={activeTab === "subtitle"}
      class:bg-transparent={activeTab !== "subtitle"}
      on:click={() => (activeTab = "subtitle")}
    >
      字幕
      {#if activeTab === "subtitle"}
        <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-[#0A84FF]"></div>
      {/if}
    </button>

    <button
      class="flex-1 px-3 py-3 text-sm font-medium transition-all duration-200 relative"
      class:text-white={activeTab === "danmu"}
      class:text-gray-400={activeTab !== "danmu"}
      class:bg-[#2c2c2e]={activeTab === "danmu"}
      class:bg-transparent={activeTab !== "danmu"}
      on:click={() => (activeTab = "danmu")}
    >
      弹幕
      {#if activeTab === "danmu"}
        <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-[#0A84FF]"></div>
      {/if}
    </button>

    <button
      class="flex-1 px-3 py-3 text-sm font-medium transition-all duration-200 relative"
      class:text-white={activeTab === "clips"}
      class:text-gray-400={activeTab !== "clips"}
      class:bg-[#2c2c2e]={activeTab === "clips"}
      class:bg-transparent={activeTab !== "clips"}
      on:click={() => (activeTab = "clips")}
    >
      选区
      {#if activeTab === "clips"}
        <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-[#0A84FF]"></div>
      {/if}
    </button>

    <button
      class="flex-1 px-3 py-3 text-sm font-medium transition-all duration-200 relative"
      class:text-white={activeTab === "upload"}
      class:text-gray-400={activeTab !== "upload"}
      class:bg-[#2c2c2e]={activeTab === "upload"}
      class:bg-transparent={activeTab !== "upload"}
      on:click={() => (activeTab = "upload")}
    >
      快速投稿
      {#if activeTab === "upload"}
        <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-[#0A84FF]"></div>
      {/if}
    </button>
  </div>

  {#if activeTab === "subtitle"}
    <VideoPreviewSubtitlePanel
      {subtitles}
      {currentSubtitleIndex}
      {formatTime}
      currentGenerateEventId={currentGenerateEventId}
      onOpenStyleEditor={onOpenStyleEditor}
      onClearSubtitles={onClearSubtitles}
      onGenerateSubtitles={onGenerateSubtitles}
      onAddSubtitle={onAddSubtitle}
      onSeekToTime={onSeekToTime}
      onAdjustTime={onAdjustTime}
      onRemoveSubtitle={onRemoveSubtitle}
    />
  {:else if activeTab === "clips"}
    <VideoPreviewClipSelectionPanel
      {clipSelections}
      {activeClipSelectionId}
      bind:clipExportSelectionIds
      bind:mergeClipSelectionsOnExport
      {formatTime}
      onSetActiveClipSelection={onSetActiveClipSelection}
      onSeekToTime={onSeekToTime}
    />
  {:else if activeTab === "upload"}
    <VideoPreviewUploadPanel
      {video}
      bind:title={profile.title}
      bind:tid={profile.tid}
      bind:uidSelected={uidSelected}
      bind:desc={profile.desc}
      bind:tag={profile.tag}
      bind:dynamic={profile.dynamic}
      bind:showCoverEditor
      {accounts}
      currentPostEventId={currentPostEventId}
      {showDefaultCoverIcon}
      {handleCoverError}
      doPost={doPost}
      cancelPost={cancelPost}
    />
  {:else if activeTab === "danmu"}
    <VideoPreviewDanmuPanel
      {danmuEnabled}
      {renderDanmuEmotes}
      {danmuPreventSubtitleOcclusionEnabled}
      {danmuSyncWithPlaybackRateEnabled}
      danmakuBold={danmakuStyle.bold}
      danmakuFontFamily={danmakuStyle.fontFamily}
      bind:danmakuDisplayAreaIndex
      bind:danmakuFontScale={danmakuStyle.fontScale}
      bind:danmakuOpacity={danmakuStyle.opacity}
      bind:danmakuSpeedPresetIndex
      bind:danmakuMaxOnScreenIndex
      {isDanmuFontMenuVisible}
      {isPbpMethodMenuVisible}
      {seekbarPbpGenerationMethod}
      danmakuDisplayAreaOptions={danmakuDisplayAreaOptions}
      danmakuSpeedPresetOptions={danmakuSpeedPresetOptions}
      danmakuMaxOnScreenOptions={danmakuMaxOnScreenOptions}
      danmakuFontOptions={danmakuFontOptions}
      seekbarPbpMethodOptions={seekbarPbpMethodOptions}
      {danmuFontMenuController}
      {pbpMethodMenuController}
      {onDanmuToggle}
      {onRenderDanmuEmotesToggle}
      {onDanmuPreventSubtitleOcclusionToggle}
      {onDanmuSyncWithPlaybackRateToggle}
      {onDanmuBoldToggle}
      {onToggleDanmuFontMenu}
      {onHandleDanmuFontSelect}
      {onTogglePbpMethodMenu}
      {onHandleSeekbarPbpMethodSelect}
    />
  {/if}
</div>

<style>
  .clip-side-panel {
    overscroll-behavior-y: none;
  }

  :global(.clip-web-fullscreen) .clip-side-panel {
    display: none;
  }
</style>
