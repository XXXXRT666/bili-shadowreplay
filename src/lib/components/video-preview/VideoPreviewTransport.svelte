<script lang="ts">
  import "./transport.css";
  import VideoPreviewControlBar from "./VideoPreviewControlBar.svelte";
  import VideoPreviewDisplayMenu from "./VideoPreviewDisplayMenu.svelte";
  import VideoPreviewPbpOverlay from "./VideoPreviewPbpOverlay.svelte";
  import VideoPreviewSeekbarPopup from "./VideoPreviewSeekbarPopup.svelte";
  import VideoPreviewTimelinePanel from "./VideoPreviewTimelinePanel.svelte";
  import type { ClipSelection } from "./clip/clipSelectionRuntime";
  import type { SubtitleItem } from "./subtitle/subtitleRuntime";

  type HoverMenuController = {
    open: () => void;
    scheduleClose: () => void;
  };

  type PreviewDisplayMetrics = {
    top: number;
    left: number;
    width: number;
    height: number;
  };

  export let isPreviewDisplayMenuVisible = false;
  export let hidePreviewDisplayMenuDuringPanelTransition = false;
  export let previewDisplayMenuElement: HTMLDivElement | null = null;
  export let isPreviewDisplayMenuInteractionLocked = false;
  export let getPreviewDisplayMenuMetrics:
    | (() => PreviewDisplayMetrics | null)
    | undefined = undefined;
  export let previewDisplayMenuWidth = 156;
  export let previewDisplayMenuHeight = 108;
  export let openPreviewDisplayMenu: (() => void) | undefined = undefined;
  export let previewDisplayMenuController: HoverMenuController;
  export let showSubtitleTimeline = false;
  export let togglePreviewDisplaySubtitleTimeline:
    | (() => void | Promise<void>)
    | undefined = undefined;
  export let showWaveform = false;
  export let togglePreviewDisplayWaveform:
    | (() => void | Promise<void>)
    | undefined = undefined;
  export let showPbpOverlay = false;
  export let togglePreviewDisplayPbp:
    | (() => void | Promise<void>)
    | undefined = undefined;

  export let seekbarPbpVisible = false;
  export let handlePbpOverlayMouseDown:
    | ((event: MouseEvent) => void | Promise<void>)
    | undefined = undefined;
  export let seekbarPbpViewBoxValue = "";
  export let seekbarPbpCurveClipPathId = "";
  export let seekbarPbpPlayedClipPathId = "";
  export let seekbarPbpCurvePath = "";
  export let seekbarPbpPlayedWidth = 0;
  export let seekbarPbpViewboxWidth = 0;
  export let seekbarPbpViewboxHeight = 0;

  export let isPlaying = false;
  export let togglePlay: (() => void) | undefined = undefined;
  export let playIconSeed = 0;
  export let openTimeSeekInput: (() => void) | undefined = undefined;
  export let handleTimeSeekKeydown: ((event: KeyboardEvent) => void) | undefined =
    undefined;
  export let isTimeSeekEditing = false;
  export let timeSeekValue = "";
  export let timeSeekInput: HTMLInputElement | null = null;
  export let handleTimeSeekInputKeydown:
    | ((event: KeyboardEvent) => void | Promise<void>)
    | undefined = undefined;
  export let closeTimeSeekInput:
    | ((applyValue: boolean) => void | Promise<void>)
    | undefined = undefined;
  export let currentTime = 0;
  export let playbackDurationValue = 0;
  export let formatTimeForSeekInput: ((time: number) => string) | undefined = undefined;
  export let timelineZoomControlElement: HTMLElement | null = null;
  export let timelineSliderValue = 0;
  export let timelineZoomSteps = 0;
  export let timelineZoomNotchCount = 0;
  export let handleTimelineZoomControlWheel:
    | ((event: WheelEvent) => void)
    | undefined = undefined;
  export let handleScaleChange: (() => void) | undefined = undefined;
  export let handleScaleCommit: (() => void) | undefined = undefined;

  export let playbackRateTriggerElement: HTMLButtonElement | null = null;
  export let isPlaybackRateMenuVisible = false;
  export let canAdjustPlaybackRateValue = true;
  export let playbackRateMenuController: HoverMenuController;
  export let playbackRateDisplayLabelValue = "";
  export let playbackRateOptions: ReadonlyArray<{ value: number; label: string }> = [];
  export let getSelectedPlaybackRate: (() => number) | undefined = undefined;
  export let handlePlaybackRateSelect: ((nextRate: number) => void) | undefined =
    undefined;

  export let previewSettingsControlElement: HTMLDivElement | null = null;
  export let previewSettingsButtonElement: HTMLButtonElement | null = null;
  export let handleSettingsMouseEnter: (() => void) | undefined = undefined;
  export let handleSettingsMouseLeave: (() => void) | undefined = undefined;
  export let isSettingsHover = false;

  export let isVolumeMutedValue = false;
  export let isVolumeMenuVisible = false;
  export let volumeMenuController: HoverMenuController;
  export let toggleMute: (() => void) | undefined = undefined;
  export let isVolumeHover = false;
  export let isVolumeClickAnimating = false;
  export let isVolumeHoverSuppressed = false;
  export let volumeIconSeed = 0;
  export let effectiveVolumePercentValue = 100;
  export let volume = 1;
  export let handleVolumeChange: ((event: Event) => void) | undefined = undefined;
  export let handleVolumeMouseEnter: (() => void) | undefined = undefined;
  export let handleVolumeMouseLeave: (() => void) | undefined = undefined;

  export let isWindowWide = false;
  export let toggleWindowWide: (() => void | Promise<void>) | undefined = undefined;
  export let isWideHover = false;
  export let onWideHoverChange: ((value: boolean) => void) | undefined = undefined;

  export let isWebFullscreen = false;
  export let toggleWebFullscreen: (() => void) | undefined = undefined;
  export let isWebFullscreenHover = false;
  export let onWebFullscreenHoverChange:
    | ((value: boolean) => void)
    | undefined = undefined;

  export let isDocumentFullscreen = false;
  export let toggleFullscreen: (() => void | Promise<void>) | undefined = undefined;
  export let isFullscreenHover = false;
  export let onFullscreenHoverChange: ((value: boolean) => void) | undefined =
    undefined;

  export let timelineContainer: HTMLElement | null = null;
  export let handleWheel: ((event: WheelEvent) => void) | undefined = undefined;
  export let handleTimelineScroll: (() => void) | undefined = undefined;
  export let waveformGestureElement: HTMLElement | null = null;
  export let waveformScale = 1;
  export let showWaveformLayoutVisible = false;
  export let waveformPanelHeightPx = 60;
  export let isWaveformLoading = false;
  export let waveformContainer: HTMLElement | null = null;

  export let timelineElement: HTMLElement | null = null;
  export let timelineScale = 1;
  export let showSubtitleTimelineLayoutVisible = false;
  export let scheduleTimelineRefresh: (() => void) | undefined = undefined;
  export let isDraggingSeekbar = false;
  export let handleTimelineClick:
    | ((event: MouseEvent) => void | Promise<void>)
    | undefined = undefined;
  export let canClip = false;
  export let clipSelections: ClipSelection[] = [];
  export let activeClipSelectionId: string | null = null;
  export let hasPendingClipStartMarker = false;
  export let pendingClipStartTime = 0;
  export let videoDuration = 0;

  export let seekbarElement: HTMLElement | null = null;
  export let seekbarProgressElement: HTMLElement | null = null;
  export let handleSeekbarMouseDown:
    | ((event: MouseEvent) => void | Promise<void>)
    | undefined = undefined;
  export let handleSeekbarMouseEnter: ((event: MouseEvent) => void) | undefined =
    undefined;
  export let handleSeekbarMouseHoverMove:
    | ((event: MouseEvent) => void)
    | undefined = undefined;
  export let handleSeekbarMouseLeave: (() => void) | undefined = undefined;
  export let isSeekbarTrackHovering = false;
  export let seekbarCurrentRatioValue = 0;
  export let isSeekbarHovering = false;
  export let seekbarCurrentXValue = 0;
  export let seekbarThumbSize = 22;
  export let showSeekbarMoveIndicatorValue = false;
  export let seekbarPointerXValue = 0;

  export let timeMarkers: number[] = [];
  export let formatTimelineMarkerTime:
    | ((time: number) => string)
    | undefined = undefined;
  export let showSubtitleTimelineEffective = false;
  export let subtitles: SubtitleItem[] = [];
  export let getSubtitleStyle: ((subtitle: SubtitleItem) => string) | undefined =
    undefined;
  export let handleBlockMouseDown:
    | ((event: MouseEvent, index: number) => void)
    | undefined = undefined;
  export let handleTimelineMouseDown:
    | ((event: MouseEvent, index: number, isStart: boolean) => void)
    | undefined = undefined;

  export let showSeekbarPopupValue = false;
  export let seekbarPopupViewportLeftValue = 0;
  export let seekbarPopupViewportTopValue = 0;
  export let seekbarPreviewImageSrc = "";
  export let transparentSeekbarPreviewImage = "";
  export let seekbarPopupTimeValue = 0;
</script>

<VideoPreviewDisplayMenu
  visible={isPreviewDisplayMenuVisible}
  hiddenDuringPanelTransition={hidePreviewDisplayMenuDuringPanelTransition}
  bind:element={previewDisplayMenuElement}
  interactionLocked={isPreviewDisplayMenuInteractionLocked}
  getMetrics={getPreviewDisplayMenuMetrics}
  width={previewDisplayMenuWidth}
  height={previewDisplayMenuHeight}
  open={openPreviewDisplayMenu}
  controller={previewDisplayMenuController}
  {showSubtitleTimeline}
  toggleSubtitleTimeline={togglePreviewDisplaySubtitleTimeline}
  {showWaveform}
  toggleWaveform={togglePreviewDisplayWaveform}
  {showPbpOverlay}
  togglePbpOverlay={togglePreviewDisplayPbp}
/>

<div class="relative z-30 overflow-visible bg-[#1c1c1e] border-t border-gray-800/50">
  <VideoPreviewPbpOverlay
    visible={seekbarPbpVisible}
    onMouseDown={handlePbpOverlayMouseDown}
    viewBox={seekbarPbpViewBoxValue}
    curveClipPathId={seekbarPbpCurveClipPathId}
    playedClipPathId={seekbarPbpPlayedClipPathId}
    curvePath={seekbarPbpCurvePath}
    playedWidth={seekbarPbpPlayedWidth}
    viewboxWidth={seekbarPbpViewboxWidth}
    viewboxHeight={seekbarPbpViewboxHeight}
  />

  <div class="pointer-events-none absolute inset-0 bg-[#1c1c1e]"></div>

  <VideoPreviewControlBar
    {isPlaying}
    {togglePlay}
    {playIconSeed}
    {openTimeSeekInput}
    {handleTimeSeekKeydown}
    {isTimeSeekEditing}
    bind:timeSeekValue
    bind:timeSeekInput
    {handleTimeSeekInputKeydown}
    {closeTimeSeekInput}
    {currentTime}
    {playbackDurationValue}
    {formatTimeForSeekInput}
    bind:timelineZoomControlElement
    bind:timelineSliderValue
    {timelineZoomSteps}
    {timelineZoomNotchCount}
    {handleTimelineZoomControlWheel}
    {handleScaleChange}
    {handleScaleCommit}
    bind:playbackRateTriggerElement
    {isPlaybackRateMenuVisible}
    {canAdjustPlaybackRateValue}
    {playbackRateMenuController}
    {playbackRateDisplayLabelValue}
    {playbackRateOptions}
    {getSelectedPlaybackRate}
    {handlePlaybackRateSelect}
    bind:previewSettingsControlElement
    bind:previewSettingsButtonElement
    {handleSettingsMouseEnter}
    {handleSettingsMouseLeave}
    {isSettingsHover}
    {isPreviewDisplayMenuVisible}
    {openPreviewDisplayMenu}
    {previewDisplayMenuController}
    {isVolumeMutedValue}
    {isVolumeMenuVisible}
    {volumeMenuController}
    {toggleMute}
    {isVolumeHover}
    {isVolumeClickAnimating}
    {isVolumeHoverSuppressed}
    {volumeIconSeed}
    {effectiveVolumePercentValue}
    bind:volume
    {handleVolumeChange}
    {handleVolumeMouseEnter}
    {handleVolumeMouseLeave}
    {isWindowWide}
    {toggleWindowWide}
    {isWideHover}
    {onWideHoverChange}
    {isWebFullscreen}
    {toggleWebFullscreen}
    {isWebFullscreenHover}
    {onWebFullscreenHoverChange}
    {isDocumentFullscreen}
    {toggleFullscreen}
    {isFullscreenHover}
    {onFullscreenHoverChange}
  />
</div>

<VideoPreviewTimelinePanel
  bind:timelineContainer
  {handleWheel}
  {handleTimelineScroll}
  bind:waveformGestureElement
  {waveformScale}
  {showWaveformLayoutVisible}
  {waveformPanelHeightPx}
  {showWaveform}
  {isWaveformLoading}
  bind:waveformContainer
  bind:timelineElement
  {timelineScale}
  {showSubtitleTimelineLayoutVisible}
  {scheduleTimelineRefresh}
  {isDraggingSeekbar}
  {handleTimelineClick}
  {canClip}
  {clipSelections}
  {activeClipSelectionId}
  {hasPendingClipStartMarker}
  {pendingClipStartTime}
  {videoDuration}
  bind:seekbarElement
  bind:seekbarProgressElement
  {handleSeekbarMouseDown}
  {handleSeekbarMouseEnter}
  {handleSeekbarMouseHoverMove}
  {handleSeekbarMouseLeave}
  {isSeekbarTrackHovering}
  {seekbarCurrentRatioValue}
  {isSeekbarHovering}
  {seekbarCurrentXValue}
  {seekbarThumbSize}
  {showSeekbarMoveIndicatorValue}
  {seekbarPointerXValue}
  {timeMarkers}
  {formatTimelineMarkerTime}
  {showSubtitleTimelineEffective}
  {subtitles}
  {getSubtitleStyle}
  {handleBlockMouseDown}
  {handleTimelineMouseDown}
/>

<VideoPreviewSeekbarPopup
  visible={showSeekbarPopupValue}
  left={seekbarPopupViewportLeftValue}
  top={seekbarPopupViewportTopValue}
  imageSrc={seekbarPreviewImageSrc}
  transparentImage={transparentSeekbarPreviewImage}
  time={seekbarPopupTimeValue}
  formatTime={formatTimeForSeekInput}
/>
