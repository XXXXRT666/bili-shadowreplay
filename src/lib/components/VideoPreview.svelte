<script lang="ts">
  import {
    DANMAKU_DURATION_SECONDS,
    DANMAKU_EMOTE_SCALE,
    DANMAKU_FONT_SIZE_PX,
    DANMAKU_FONT_FAMILY,
    DANMAKU_FONT_WEIGHT,
    DANMAKU_LINE_HEIGHT,
    DANMAKU_EMOTE_VERTICAL_OFFSET_EM,
    DANMAKU_TEXT_SHADOW,
    loadDanmakuStyle,
    saveDanmakuStyle,
    getDanmakuEmoteTextGap,
    loadDanmakuEmoteMap,
    type DanmakuSegment,
  } from "../danmaku-emotes";
  import {
    generateEventId,
    parseSubtitleStyle,
    type DanmuEntry,
    type DanmuRenderOptions,
    type DanmakuEmoteMap,
    type DanmakuStyle,
    type SubtitleStyle,
    type VideoItem,
    type VideoPbpData,
    type Profile,
    type Config,
    default_profile,
  } from "../interface";
  import SubtitleStyleEditor from "./SubtitleStyleEditor.svelte";
  import CoverEditor from "./CoverEditor.svelte";
  import LottieIcon from "./LottieIcon.svelte";
  import { invoke, TAURI_ENV, listen, log, close_window } from "../invoker";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onDestroy, onMount, tick } from "svelte";
  import skipLottieData from "../../assets/lottie/player-three-playrate-hint.json";
  import muteLottieData from "../../assets/lottie/player-ctrl-muted.json";
  import volumeLottieData from "../../assets/lottie/player-ctrl-volume.json";
  import {
    DEFAULT_MACOS_NATIVE_PLAYER_WINDOWED_PREVIEW_COMPENSATION,
    MACOS_NATIVE_PLAYER_POPUP_FALLBACK_ENABLED,
    MACOS_NATIVE_PLAYER_ID,
    computeMacOSNativePlayerTargetRect,
    computeMacOSNativePlayerWindowedYOffset,
    createMacOSNativeClipPlayerRuntime,
    detectMacOSNativePlayerSupport,
    getMacOSNativePlayerViewportSignatureParts,
    pauseMacOSNativePlayer,
    seekMacOSNativePlayer,
    setMacOSNativePlayerRate,
    shouldHideMacOSNativeDomPreview,
  } from "./video-preview/macos-native/macosNativeClipPlayer";
  import {
    BILIBILI_PBP_METHOD_OPTION,
    SEEKBAR_PBP_METHOD_OPTIONS,
    SEEKBAR_PBP_VIEWBOX_HEIGHT,
    SEEKBAR_PBP_VIEWBOX_WIDTH,
    buildBilibiliSeekbarPbpCurvePath,
    buildSeekbarPbpCurvePath,
    formatTime,
    formatTimeForSeekInput,
    formatTimelineMarkerTime,
    parseTimeInput,
    resolveSeekbarPbpGlobalMaxDensity,
    resolveSeekbarPbpSampleCount,
    resolveTimelineMarkers,
    type SeekbarPbpGenerationMethod,
  } from "./video-preview/timeline/timelinePresentation";
  import {
    applyClipRegionLabel,
    canBeClipped as canBeClippedRuntime,
    clearAllClipSelections as clearAllClipSelectionsRuntime,
    clearClipSelectionState,
    createClipSelectionId as createClipSelectionIdRuntime,
    generateClip as generateClipRuntime,
    removeWaveformRegion,
    setActiveClipSelection as setActiveClipSelectionRuntime,
    setClipEndTime as setClipEndTimeRuntime,
    setClipStartTime as setClipStartTimeRuntime,
    syncClipWaveformRegions as syncClipWaveformRegionsRuntime,
    updateClipSelectionFromRegion as updateClipSelectionFromRegionRuntime,
    type ClipSelection,
    type ClipSelectionState,
  } from "./video-preview/clip/clipSelectionRuntime";
  import {
    adjustSubtitleTime as adjustSubtitleTimeRuntime,
    clearSubtitles as clearSubtitlesRuntime,
    createSubtitleAtTime,
    generateSubtitles as generateSubtitlesRuntime,
    getSubtitleStyle as getSubtitleStyleRuntime,
    loadSubtitles as loadSubtitlesRuntime,
    moveSubtitle as moveSubtitleRuntime,
    removeSubtitle as removeSubtitleRuntime,
    saveSubtitles as saveSubtitlesRuntime,
    type SubtitleItem,
    updateSubtitleTime as updateSubtitleTimeRuntime,
  } from "./video-preview/subtitle/subtitleRuntime";
  import { createSubtitleTimelineRuntime } from "./video-preview/subtitle/subtitleTimelineRuntime";
  import { createSeekbarThumbnailRuntime } from "./video-preview/timeline/seekbarThumbnailRuntime";
  import {
    createSeekbarInteractionRuntime,
    getEventClientPoint,
  } from "./video-preview/timeline/seekbarInteractionRuntime";
  import {
    createPlaybackHotkeyRuntime,
    isBlockedHotkeyTarget as isBlockedHotkeyTargetRuntime,
    isTextEntryHotkeyTarget as isTextEntryHotkeyTargetRuntime,
  } from "./video-preview/playback/playbackHotkeyRuntime";
  import { createTimelineZoomRuntime } from "./video-preview/timeline/timelineZoomRuntime";
  import {
    clearActiveDanmusState,
    getDanmuMaxActiveDurationMs,
    loadDanmuRecords,
    rebuildDanmuPlaybackState,
    removeActiveDanmuById,
    resetDanmuPlaybackState,
    syncDanmuPlaybackState,
    type ActiveDanmu,
    type DanmuLayoutConfig,
    type DanmuPlaybackState,
  } from "./video-preview/danmu/danmuRuntime";
  import {
    createWaveSurferRuntime,
    destroyWaveSurferRuntime,
    ensureWaveformData as ensureWaveformDataRuntime,
    redrawWaveformAtCurrentWidth as redrawWaveformAtCurrentWidthRuntime,
    syncWaveformWithVideo as syncWaveformWithVideoRuntime,
    type AudioWaveformData,
  } from "./video-preview/waveform/waveformRuntime";
  import {
    applyTimelinePanelLayoutChange as applyTimelinePanelLayoutChangeRuntime,
    cancelClipWindowHeightSync as cancelClipWindowHeightSyncRuntime,
    cancelPreviewLayoutSync as cancelPreviewLayoutSyncRuntime,
    createClipWindowLayoutState,
    getAdaptiveClipWindowHeightForState as getAdaptiveClipWindowHeightForStateRuntime,
    resolvePreviewDisplayMetrics,
    shouldSuppressClipWindowResizeRefresh as shouldSuppressClipWindowResizeRefreshRuntime,
    suppressClipWindowResizeRefresh as suppressClipWindowResizeRefreshRuntime,
    syncClipWindowHeight as syncClipWindowHeightRuntime,
    syncPreviewLayoutAfterUiChange as syncPreviewLayoutAfterUiChangeRuntime,
  } from "./video-preview/clip/clipWindowLayoutRuntime";
  import {
    createClipWindowLifecycleState,
    disposeClipWindowLifecycle,
    setupClipWindowLifecycle,
  } from "./video-preview/clip/clipWindowLifecycleRuntime";
  import {
    createClipWindowPlatformRuntime,
    createClipWindowPlatformState,
  } from "./video-preview/clip/clipWindowPlatformRuntime";
  import VideoPreviewDanmuOverlay from "./video-preview/VideoPreviewDanmuOverlay.svelte";
  import VideoPreviewEncodeModal from "./video-preview/VideoPreviewEncodeModal.svelte";
  import VideoPreviewHotkeyOverlay from "./video-preview/VideoPreviewHotkeyOverlay.svelte";
  import VideoPreviewSidePanel from "./video-preview/VideoPreviewSidePanel.svelte";
  import VideoPreviewSubtitleOverlay from "./video-preview/VideoPreviewSubtitleOverlay.svelte";
  import VideoPreviewTopBar from "./video-preview/VideoPreviewTopBar.svelte";
  import VideoPreviewTransport from "./video-preview/VideoPreviewTransport.svelte";
  import VideoPreviewLetterboxMask from "./video-preview/VideoPreviewLetterboxMask.svelte";
  import {
    captureElementRect,
    computePlaybackRateButtonLabel,
    createHoverMenuController,
    resolveAnchoredMenuMetrics,
  } from "./video-preview/playback/previewControlsState";

  export let show = false;
  export let video: VideoItem;
  export let roomId: string;
  export let videos: any[] = [];
  export let onVideoChange: ((video: VideoItem) => void) | undefined =
    undefined;
  export let onVideoListUpdate: (() => Promise<void>) | undefined = undefined;

  type Subtitle = SubtitleItem;

  const DANMU_LOOKBACK_MS = 1000;
  const DANMU_ACTIVE_DURATION_MS = DANMAKU_DURATION_SECONDS * 1000;
  const CLIP_START_MARKER_REGION_ID = "clip-start-marker";
  const CLIP_SELECTION_REGION_ID_PREFIX = "clip-selection-region-";
  const CLIP_REGION_COLOR = "rgba(34, 197, 94, 0.3)";
  const FAST_FORWARD_HOLD_DELAY_MS = 180;
  const FAST_FORWARD_PLAYBACK_RATE = 3;
  const PLAYBACK_RATE_OPTIONS = [
    { value: 0.5, label: "0.5x" },
    { value: 1, label: "1.0x" },
    { value: 1.25, label: "1.25x" },
    { value: 1.5, label: "1.5x" },
    { value: 2, label: "2.0x" },
  ] as const;
  const SEEKBAR_PREVIEW_WIDTH = 160;
  const SEEKBAR_PREVIEW_HEIGHT = 90;
  const SEEKBAR_THUMB_SIZE = 22;
  const SEEKBAR_PULL_INDICATOR_SIZE = 18;
  const DANMAKU_DISPLAY_AREA_OPTIONS = [10, 25, 50, 75, 100] as const;
  const DANMAKU_SPEED_PRESET_OPTIONS = [
    "极慢",
    "较慢",
    "适中",
    "较快",
    "极快",
  ] as const;
  const DANMAKU_MAX_ON_SCREEN_OPTIONS = [-1, 25, 50, 100, 200] as const;
  const DANMAKU_FONT_OPTIONS = [
    { label: "黑体", value: DANMAKU_FONT_FAMILY },
    {
      label: "微软雅黑",
      value: '"Microsoft YaHei", "PingFang SC", sans-serif',
    },
    { label: "苹方", value: '"PingFang SC", "Hiragino Sans GB", sans-serif' },
    {
      label: "冬青黑体",
      value: '"Hiragino Sans GB", "PingFang SC", sans-serif',
    },
    {
      label: "思源黑体",
      value: '"Noto Sans CJK SC", "Source Han Sans SC", sans-serif',
    },
    { label: "Arial", value: 'Arial, "Helvetica Neue", Helvetica, sans-serif' },
    {
      label: "Helvetica",
      value: '"Helvetica Neue", Helvetica, Arial, "PingFang SC", sans-serif',
    },
    { label: "Tahoma", value: 'Tahoma, "Microsoft YaHei", sans-serif' },
    { label: "Verdana", value: 'Verdana, "Microsoft YaHei", sans-serif' },
    { label: "宋体", value: 'SimSun, "Songti SC", serif' },
    { label: "仿宋", value: "FangSong, STFangsong, serif" },
    { label: "楷体", value: "KaiTi, STKaiti, serif" },
    {
      label: "思源宋体",
      value: '"Noto Serif CJK SC", "Source Han Serif SC", serif',
    },
  ] as const;
  const SEEKBAR_HOVER_HIT_TOP_EXTEND = 6;
  const SEEKBAR_HOVER_HIT_BOTTOM_EXTEND = 8;
  const TIMELINE_GESTURE_SCALE_DELTA_PER_STEP = 0.1;
  const TIMELINE_GESTURE_WHEEL_DELTA_PER_STEP = 50;
  const TIMELINE_GESTURE_COMMIT_DELAY_MS = 120;
  const CLIP_WINDOW_BASE_HEIGHT = 736;
  const CLIP_WINDOW_SUBTITLE_TIMELINE_EXTRA_HEIGHT = 16;
  const CLIP_WINDOW_WAVEFORM_EXTRA_HEIGHT = 60;
  const WAVEFORM_PANEL_HEIGHT_PX = 60;
  const WAVEFORM_BAR_HEIGHT_RATIO = 0.7;
  const CLIP_WINDOW_HEIGHT_SYNC_THRESHOLD_PX = 1;
  const CLIP_WINDOW_RESIZE_REFRESH_SUPPRESS_MS = 240;
  const TIMELINE_AXIS_SIDE_INSET_PX = 12;
  const TRANSPARENT_SEEKBAR_PREVIEW_IMAGE =
    "data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs=";
  const seekbarPbpCurveClipPathId = `bpx-player-pbp-curve-path-${Math.random()
    .toString(36)
    .slice(2, 10)}`;
  const seekbarPbpPlayedClipPathId = `bpx-player-pbp-played-path-${Math.random()
    .toString(36)
    .slice(2, 10)}`;

  function getBrowserWindow() {
    return typeof window === "undefined" ? null : window;
  }

  let subtitles: Subtitle[] = [];
  let danmuRecords: DanmuEntry[] = [];
  let activeDanmus: ActiveDanmu[] = [];
  let currentTime = 0;
  let currentSubtitle = "";
  let videoElement: HTMLVideoElement;
  let macOSNativePlayerSlotElement: HTMLDivElement | null = null;
  let previewStageElement: HTMLDivElement | null = null;
  let previewStageHeightLockPx: number | null = null;
  let showDefaultCoverIcon = false;
  let timelineWidth = 0;

  // 当视频改变时重置封面错误状态
  $: if (video) {
    showDefaultCoverIcon = false;
  }
  let timelineElement: HTMLElement;
  let draggingSubtitle: { index: number; isStart: boolean } | null = null;
  let draggingBlock: number | null = null;
  let timelineScale = 1; // 时间轴缩放比例，1 表示正常大小
  let timelineSliderValue = 0;
  let maxTimelineScale = 10;
  let isPlaying = false;
  let macOSNativePauseRequestPending = false;
  let timeMarkers: number[] = [];
  let dragOffset: number = 0; // 添加拖动偏移量
  let isVideoLoaded = false;
  let showStyleEditor = false;
  let volume = 1;
  let previousVolume = 1;
  let isMuted = false;
  let playIconSeed = 0;
  let lastIsPlayingState = false;
  let volumeIconSeed = 0;
  let lastVolumeMutedState = false;
  let isVolumeHover = false;
  let isVolumeHoverSuppressed = false;
  let isVolumeClickAnimating = false;
  let isSettingsHover = false;
  let isFullscreenHover = false;
  let isWebFullscreenHover = false;
  let isWideHover = false;
  let isTimeSeekEditing = false;
  let timeSeekValue = "";
  let timeSeekInput: HTMLInputElement | null = null;
  let isWebFullscreen = false;
  let isWindowWide = false;
  let currentSubtitleIndex = -1;
  let timelineContainer: HTMLElement | null = null;
  let timelineZoomControlElement: HTMLElement | null = null;
  let waveformGestureElement: HTMLElement | null = null;
  let showEncodeModal = false;
  let showClipGenerateModal = false;
  let encodeIncludeSubtitle = true;
  let encodeIncludeDanmu = false;
  let clipIncludeSubtitle = false;
  let clipIncludeDanmu = false;
  let videoWidth = 0;
  let videoHeight = 0;
  let videoDisplayHeight = 0;
  let previewDisplayHeight = 720;
  let videoResizeObserver: ResizeObserver | null = null;
  let macOSNativePlayerAvailable = false;
  let isMacOSNativePlayerActive = false;
  let macOSNativePlayerMountFailed = false;
  let macOSNativePlayerMountPending = false;
  let forceDomPreviewHidden = false;
  let previewStageViewportTop = 0;
  let previewStageViewportLeft = 0;
  let previewStageViewportWidth = 0;
  let previewStageViewportHeight = 0;
  let previewVideoFrameTop = 0;
  let previewVideoFrameLeft = 0;
  let previewVideoFrameWidth = 0;
  let previewVideoFrameHeight = 0;
  let stablePreviewVideoFrameStyle = "display:none;";
  let arePreviewDisplayMetricsFrozen = false;
  let shouldUseMacOSNativePlayerValue = false;
  let shouldRenderMacOSNativePlayerPresentationValue = false;
  let shouldPreferMacOSNativePlayerPresentationValue = false;
  let showMacOSNativePlayerUnderlayValue = false;
  let suppressPreviewOverlayLayersValue = false;
  let usingMacOSNativePlayerValue = false;
  let isPlaybackRateMenuVisible = false;
  let playbackRateMenuHideTimer: number | null = null;
  let isPreviewDisplayMenuVisible = false;
  let previewDisplayMenuHideTimer: number | null = null;
  let isPreviewDisplayMenuInteractionLocked = false;
  let hidePreviewDisplayMenuDuringPanelTransition = false;
  let playbackRateTriggerElement: HTMLButtonElement | null = null;
  let previewSettingsControlElement: HTMLDivElement | null = null;
  let previewSettingsButtonElement: HTMLButtonElement | null = null;
  let previewDisplayMenuElement: HTMLDivElement | null = null;
  let previewDisplayMenuLockedRect: {
    top: number;
    left: number;
    width: number;
    height: number;
  } | null = null;
  let reserveSubtitleTimelineLayoutDuringResize = false;
  let reserveWaveformLayoutDuringResize = false;
  const PREVIEW_DISPLAY_MENU_WIDTH = 156;
  const PREVIEW_DISPLAY_MENU_HEIGHT = 108;
  let isPbpMethodMenuVisible = false;
  let pbpMethodMenuHideTimer: number | null = null;
  let isVolumeMenuVisible = false;
  let volumeMenuHideTimer: number | null = null;
  let isDanmuFontMenuVisible = false;
  let danmuFontMenuHideTimer: number | null = null;
  let isVideoSelectMenuVisible = false;
  let videoSelectMenuHideTimer: number | null = null;
  let volumeClickAnimationTimer: number | null = null;
  let showFastForwardOverlayValue = false;
  let playbackRateDisplayLabelValue = "倍速";
  let canAdjustPlaybackRateValue = true;
  let isDocumentFullscreen = false;
  let suspendMacOSNativePlayerUnderlayForPopupValue = false;
  let playbackDurationValue = 0;
  let effectiveVolumePercentValue = 100;
  let threePlayrateHintTopValue = 18;
  let macOSNativePlayerWindowedPreviewCompensationValue =
    DEFAULT_MACOS_NATIVE_PLAYER_WINDOWED_PREVIEW_COMPENSATION;
  const danmuFontMenuController = createHoverMenuController({
    getTimer: () => danmuFontMenuHideTimer,
    setTimer: (value) => {
      danmuFontMenuHideTimer = value;
    },
    setVisible: (value) => {
      isDanmuFontMenuVisible = value;
    },
    canOpen: () => danmuEnabled,
  });
  const videoSelectMenuController = createHoverMenuController({
    getTimer: () => videoSelectMenuHideTimer,
    setTimer: (value) => {
      videoSelectMenuHideTimer = value;
    },
    setVisible: (value) => {
      isVideoSelectMenuVisible = value;
    },
  });
  const playbackRateMenuController = createHoverMenuController({
    getTimer: () => playbackRateMenuHideTimer,
    setTimer: (value) => {
      playbackRateMenuHideTimer = value;
    },
    setVisible: (value) => {
      isPlaybackRateMenuVisible = value;
    },
    canOpen: () => canAdjustPlaybackRateValue,
    closeDelayMs: 80,
  });
  const previewDisplayMenuController = createHoverMenuController({
    getTimer: () => previewDisplayMenuHideTimer,
    setTimer: (value) => {
      previewDisplayMenuHideTimer = value;
    },
    setVisible: (value) => {
      isPreviewDisplayMenuVisible = value;
    },
    canOpen: () => !isPreviewDisplayMenuInteractionLocked,
  });
  const pbpMethodMenuController = createHoverMenuController({
    getTimer: () => pbpMethodMenuHideTimer,
    setTimer: (value) => {
      pbpMethodMenuHideTimer = value;
    },
    setVisible: (value) => {
      isPbpMethodMenuVisible = value;
    },
  });
  const volumeMenuController = createHoverMenuController({
    getTimer: () => volumeMenuHideTimer,
    setTimer: (value) => {
      volumeMenuHideTimer = value;
    },
    setVisible: (value) => {
      isVolumeMenuVisible = value;
    },
  });
  const macOSNativeClipPlayerRuntime = createMacOSNativeClipPlayerRuntime({
    playerId: MACOS_NATIVE_PLAYER_ID,
    tauriEnv: TAURI_ENV,
    getVideoSource: () => video?.file,
    getSlotElement: () => macOSNativePlayerSlotElement,
    getTargetRect: () => getMacOSNativePlayerTargetRect(),
    getViewportSignatureParts: () => getMacOSNativePlayerViewportSignatureParts(),
    getIsDocumentFullscreen: () => isDocumentFullscreen,
    getShouldUseMacOSNativePlayer: () => shouldUseMacOSNativePlayer(),
    getUsingMacOSNativePlayer: () => usingMacOSNativePlayer(),
    getIsMacOSNativePlayerActive: () => isMacOSNativePlayerActive,
    getMacOSNativePlayerAvailable: () => macOSNativePlayerAvailable,
    setMacOSNativePlayerMountFailed: (value) => {
      macOSNativePlayerMountFailed = value;
    },
    setIsMacOSNativePlayerActive: (value) => {
      isMacOSNativePlayerActive = value;
    },
    setForceDomPreviewHidden: (value) => {
      forceDomPreviewHidden = value;
    },
    getVolume: () => getEffectiveVolume(),
    getVideoElement: () => videoElement ?? null,
    shouldHideDomPreview: () => shouldHideDomPreview(),
    onPlaybackState: (state) => {
      const wasPlaying = isPlaying;
      const timeDelta = Math.abs(state.currentTime - currentTime);
      let nextPlaying = state.playing;
      let ignoredStaleNativePlayingState = false;
      if (macOSNativePauseRequestPending) {
        if (state.playing) {
          nextPlaying = false;
          ignoredStaleNativePlayingState = true;
        } else {
          macOSNativePauseRequestPending = false;
        }
      }
      currentPlaybackRate = nextPlaying
        ? Math.max(0.1, state.rate || getSelectedPlaybackRate())
        : getSelectedPlaybackRate();
      isPlaying = nextPlaying;
      const shouldSyncDanmuOnNativeState =
        !ignoredStaleNativePlayingState &&
        (nextPlaying || timeDelta > 0.5);
      const isNoopPausedNativeState =
        !wasPlaying &&
        !nextPlaying &&
        timeDelta <= 0.001;
      if (isNoopPausedNativeState) {
        return;
      }
      const isPausedNativeTimeDrift =
        !nextPlaying && timeDelta <= 0.5;
      if (isPausedNativeTimeDrift) {
        return;
      }
      applyPlaybackProgress(
        state.currentTime,
        !isRightArrowFastForwardActive,
        shouldSyncDanmuOnNativeState,
      );

      if (
        wasPlaying &&
        !nextPlaying &&
        state.duration > 0 &&
        state.currentTime >= Math.max(0, state.duration - 0.05)
      ) {
        handleNativePlaybackEnded();
      }
    },
    onPlaybackEnded: () => {
      handleNativePlaybackEnded();
    },
  });
  const seekbarThumbnailRuntime = createSeekbarThumbnailRuntime({
    getShouldGenerate: () =>
      TAURI_ENV && Boolean(config?.use_seekbar_thumbnail_cache),
    getVideoId: () => {
      const videoId = Number(video?.id);
      return Number.isFinite(videoId) && videoId > 0 ? videoId : null;
    },
    getDuration: () => getSeekbarDuration(),
    getShowSeekbarPopup: () => showSeekbarPopupValue,
    getSeekbarPopupTime: () => seekbarPopupTimeValue,
    setPreviewImageSrc: (value) => {
      seekbarPreviewImageSrc = value;
    },
  });
  let subtitleStyle: SubtitleStyle = {
    fontName: "Arial",
    fontSize: 18,
    fontWeight: 700,
    fontColor: "#FFFFFF",
    outlineColor: "#000000",
    outlineWidth: 2,
    alignment: 2,
    marginV: 20,
    marginL: 20,
    marginR: 20,
  };

  let current_encode_event_id = null;
  let current_generate_event_id = null;
  let activeTab = "subtitle"; // 添加当前激活的 tab
  let showSubtitleTimeline = false;
  let showSubtitleTimelineEffective = false;
  let showSubtitleTimelineLayoutVisible = false;
  let showWaveform = false;
  let showWaveformLayoutVisible = false;
  let showPbpOverlay = true;
  let seekbarPbpGenerationMethod: SeekbarPbpGenerationMethod = "conv_curve";
  let bilibiliPbpData: VideoPbpData | null = null;
  let waveformScale = 1;
  const timelineZoomSteps = 1000;
  const TIMELINE_ZOOM_NOTCH_PERCENTS = [
    0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1,
  ] as const;
  const TIMELINE_ZOOM_NOTCH_VALUES = TIMELINE_ZOOM_NOTCH_PERCENTS.map(
    (percent) => percent * timelineZoomSteps,
  );

  // 切片功能相关变量
  let clipStartTime = 0;
  let clipEndTime = 0;
  let clipTitle = "";
  let clipping = false;
  let current_clip_event_id = null;
  let show_detail = false; // 控制快捷键说明的展开
  let lastVideoId = -1; // 记录上一个视频ID，避免重复初始化
  let lastSeekbarThumbnailVideoSource = "";
  let clipTimesSet = false; // 标记用户是否主动设置过切片时间
  let clipSelections: ClipSelection[] = [];
  let activeClipSelectionId: string | null = null;
  let clipExportSelectionIds: string[] = [];
  let previousClipSelectionIds: string[] = [];
  let mergeClipSelectionsOnExport = true;
  let clipSelectionIdCounter = 0;
  let clipSelectionPersistenceVideoId: number | null = null;
  let clipSelectionPersistenceReady = false;
  let hasPendingClipStartMarker = false;
  let pendingClipStartTime = 0;
  let clipRegionColor = "";

  // 进度条拖动相关变量
  let isDraggingSeekbar = false;
  let seekbarElement: HTMLElement;
  let seekbarProgressElement: HTMLElement;
  let previewTime = 0; // 拖动时预览的时间
  let wasPlayingBeforeDrag = false; // 拖动前的播放状态
  let isSeekbarHovering = false;
  let isSeekbarTrackHovering = false;
  let seekbarHoverTime = 0;
  let seekbarPreviewImageSrc = "";
  let seekbarMetricsWidth = 0;
  let seekbarViewportLeft = 0;
  let seekbarViewportTop = 0;
  let seekbarViewportHeight = 6;
  let seekbarCurrentRatioValue = 0;
  let seekbarCurrentXValue = 0;
  let seekbarPointerRatioValue = 0;
  let seekbarPointerXValue = 0;
  let seekbarPopupLeftValue = 0;
  let seekbarPopupTimeValue = 0;
  let seekbarPopupViewportLeftValue = 0;
  let seekbarPopupViewportTopValue = 0;
  let seekbarPointerClientX: number | null = null;
  let seekbarPointerClientY: number | null = null;
  let showSeekbarPopupValue = false;
  let showSeekbarMoveIndicatorValue = false;
  let seekbarPbpCurvePath = "";
  let seekbarPbpPlayedWidth = 0;
  let seekbarPbpVisible = false;
  let seekbarPbpCurveCacheKey = "";
  let seekbarPbpGlobalMaxDensity = 0;
  let seekbarPbpGlobalMaxDensityCacheKey = "";
  let seekbarPbpZoomVersion = 0;
  let seekbarPbpViewBoxX = 0;
  let seekbarPbpViewBoxWidth = SEEKBAR_PBP_VIEWBOX_WIDTH;
  let seekbarPbpViewBoxValue = `0 0 ${SEEKBAR_PBP_VIEWBOX_WIDTH} ${SEEKBAR_PBP_VIEWBOX_HEIGHT}`;
  $: hasBilibiliPbpData =
    Boolean(bilibiliPbpData?.values?.some((value) => Number(value) > 0)) &&
    Number(bilibiliPbpData?.stepSec) > 0;
  $: seekbarPbpMethodOptions = hasBilibiliPbpData
    ? [BILIBILI_PBP_METHOD_OPTION, ...SEEKBAR_PBP_METHOD_OPTIONS]
    : SEEKBAR_PBP_METHOD_OPTIONS;
  $: if (!hasBilibiliPbpData && seekbarPbpGenerationMethod === "bilibili_pbp") {
    seekbarPbpGenerationMethod = "conv_curve";
    seekbarPbpZoomVersion += 1;
  }

  // 投稿相关变量
  let current_post_event_id = null;
  let config: Config = null;
  let accounts: any[] = [];
  let uid_selected = 0;
  let show_cover_editor = false;

  // WaveSurfer.js 相关变量
  let wavesurfer: any = null;
  let waveformRegions: any = null;
  let clipStartMarkerRegion: any = null;
  let clipSelectionRegions: Record<string, any> = {};
  let waveformContainer: HTMLElement;
  let isWaveformLoaded = false;
  let isWaveformLoading = false;
  let waveformDataPromise: Promise<AudioWaveformData> | null = null;
  let waveformDataVideoId: number | null = null;
  let seekbarHoverSyncFrame: number | null = null;
  let waveformRenderFrame: number | null = null;

  function getClipSelectionState(): ClipSelectionState {
    return {
      clipSelections,
      activeClipSelectionId,
      clipStartTime,
      clipEndTime,
      clipTimesSet,
      hasPendingClipStartMarker,
      pendingClipStartTime,
      clipRegionColor,
    };
  }

  function applyClipSelectionState(state: ClipSelectionState) {
    clipSelections = state.clipSelections;
    activeClipSelectionId = state.activeClipSelectionId;
    clipStartTime = state.clipStartTime;
    clipEndTime = state.clipEndTime;
    clipTimesSet = state.clipTimesSet;
    hasPendingClipStartMarker = state.hasPendingClipStartMarker;
    pendingClipStartTime = state.pendingClipStartTime;
    clipRegionColor = state.clipRegionColor;
  }

  function getClipSelectionStorageKey(videoId: number) {
    return `clip_selections:${videoId}`;
  }

  function sanitizePersistedClipSelections(value: unknown): ClipSelection[] {
    if (!Array.isArray(value)) {
      return [];
    }

    return value
      .filter((selection): selection is ClipSelection => {
        if (!selection || typeof selection !== "object") {
          return false;
        }
        const item = selection as ClipSelection;
        return (
          typeof item.id === "string" &&
          Number.isFinite(item.startTime) &&
          Number.isFinite(item.endTime) &&
          item.endTime > item.startTime
        );
      })
      .map((selection) => ({
        id: selection.id,
        startTime: Math.max(0, selection.startTime),
        endTime: Math.max(0, selection.endTime),
        color: selection.color || CLIP_REGION_COLOR,
      }));
  }

  function inferClipSelectionCounter(selections: ClipSelection[]) {
    return selections.reduce((counter, selection) => {
      const match = selection.id.match(/(\d+)$/);
      return Math.max(counter, match ? Number(match[1]) : 0);
    }, 0);
  }

  function loadPersistedClipSelections(videoId: number) {
    clipSelectionPersistenceReady = false;
    clipSelectionPersistenceVideoId = videoId;

    let savedState: any = null;
    try {
      const raw = window.localStorage.getItem(getClipSelectionStorageKey(videoId));
      savedState = raw ? JSON.parse(raw) : null;
    } catch (error) {
      log.warn("failed to load clip selections", { videoId, error });
    }

    const savedSelections = sanitizePersistedClipSelections(
      savedState?.clipSelections,
    );
    const savedSelectionIds = new Set(savedSelections.map((selection) => selection.id));
    clipSelections = savedSelections;
    activeClipSelectionId =
      typeof savedState?.activeClipSelectionId === "string" &&
      savedSelectionIds.has(savedState.activeClipSelectionId)
        ? savedState.activeClipSelectionId
        : (savedSelections[savedSelections.length - 1]?.id ?? null);
    clipExportSelectionIds = Array.isArray(savedState?.clipExportSelectionIds)
      ? savedState.clipExportSelectionIds.filter((id: unknown) =>
          typeof id === "string" && savedSelectionIds.has(id),
        )
      : savedSelections.map((selection) => selection.id);
    previousClipSelectionIds = savedSelections.map((selection) => selection.id);
    mergeClipSelectionsOnExport =
      typeof savedState?.mergeClipSelectionsOnExport === "boolean"
        ? savedState.mergeClipSelectionsOnExport
        : true;
    clipSelectionIdCounter = Math.max(
      Number(savedState?.clipSelectionIdCounter) || 0,
      inferClipSelectionCounter(savedSelections),
    );
    applyClipSelectionState(
      setActiveClipSelectionRuntime({
        state: {
          ...getClipSelectionState(),
          clipSelections,
          activeClipSelectionId,
          hasPendingClipStartMarker: false,
          pendingClipStartTime: 0,
        },
        id: activeClipSelectionId,
      }),
    );
    clipSelectionPersistenceReady = true;
  }

  function savePersistedClipSelections() {
    if (
      typeof window === "undefined" ||
      !clipSelectionPersistenceReady ||
      !clipSelectionPersistenceVideoId
    ) {
      return;
    }

    window.localStorage.setItem(
      getClipSelectionStorageKey(clipSelectionPersistenceVideoId),
      JSON.stringify({
        clipSelections,
        activeClipSelectionId,
        clipExportSelectionIds,
        mergeClipSelectionsOnExport,
        clipSelectionIdCounter,
      }),
    );
  }

  $: {
    const currentIds = clipSelections.map((selection) => selection.id);
    const previousIds = new Set(previousClipSelectionIds);
    const selectedIds = new Set(clipExportSelectionIds);
    const nextExportSelectionIds = currentIds.filter(
      (id) => selectedIds.has(id) || !previousIds.has(id),
    );

    if (
      nextExportSelectionIds.length !== clipExportSelectionIds.length ||
      nextExportSelectionIds.some((id, index) => id !== clipExportSelectionIds[index])
    ) {
      clipExportSelectionIds = nextExportSelectionIds;
    }

    previousClipSelectionIds = currentIds;
  }

  $: {
    clipSelections;
    activeClipSelectionId;
    clipExportSelectionIds;
    mergeClipSelectionsOnExport;
    clipSelectionIdCounter;
    savePersistedClipSelections();
  }

  const clipWindowLifecycleState = createClipWindowLifecycleState();
  const clipWindowLayoutState = createClipWindowLayoutState();
  const clipWindowPlatformState = createClipWindowPlatformState();
  const subtitleTimelineRuntime = createSubtitleTimelineRuntime({
    timelineAxisSideInsetPx: TIMELINE_AXIS_SIDE_INSET_PX,
    getTimelineElement: () => timelineElement,
    getVideoDuration: () => videoElement?.duration ?? 0,
    getSubtitles: () => subtitles,
    getDraggingSubtitle: () => draggingSubtitle,
    setDraggingSubtitle: (value) => {
      draggingSubtitle = value;
    },
    getDraggingBlock: () => draggingBlock,
    setDraggingBlock: (value) => {
      draggingBlock = value;
    },
    getDragOffset: () => dragOffset,
    setDragOffset: (value) => {
      dragOffset = value;
    },
    updateSubtitleTime: (index, isStart, time) => {
      updateSubtitleTime(index, isStart, time);
    },
    moveSubtitle: (index, newStartTime) => {
      moveSubtitle(index, newStartTime);
    },
  });
  const seekbarInteractionRuntime = createSeekbarInteractionRuntime({
    hoverHitTopExtend: SEEKBAR_HOVER_HIT_TOP_EXTEND,
    hoverHitBottomExtend: SEEKBAR_HOVER_HIT_BOTTOM_EXTEND,
    getSeekbarElement: () => seekbarElement,
    getSeekbarProgressElement: () => seekbarProgressElement,
    getTimelineWidth: () => timelineWidth,
    getWindowObject: getBrowserWindow,
    getIsDraggingSeekbar: () => isDraggingSeekbar,
    setIsDraggingSeekbar: (value) => {
      isDraggingSeekbar = value;
    },
    getIsPlaying: () => isPlaying,
    setIsPlaying: (value) => {
      isPlaying = value;
    },
    getWasPlayingBeforeDrag: () => wasPlayingBeforeDrag,
    setWasPlayingBeforeDrag: (value) => {
      wasPlayingBeforeDrag = value;
    },
    getPreviewTimeValue: () => previewTime,
    setPreviewTimeValue: (value) => {
      previewTime = value;
    },
    setIsSeekbarHovering: (value) => {
      isSeekbarHovering = value;
    },
    setIsSeekbarTrackHovering: (value) => {
      isSeekbarTrackHovering = value;
    },
    setSeekbarHoverTime: (value) => {
      seekbarHoverTime = value;
    },
    setSeekbarMetricsWidth: (value) => {
      seekbarMetricsWidth = value;
    },
    setSeekbarViewportLeft: (value) => {
      seekbarViewportLeft = value;
    },
    setSeekbarViewportTop: (value) => {
      seekbarViewportTop = value;
    },
    setSeekbarViewportHeight: (value) => {
      seekbarViewportHeight = value;
    },
    getSeekbarPointerClientX: () => seekbarPointerClientX,
    getSeekbarPointerClientY: () => seekbarPointerClientY,
    setSeekbarPointerClientPoint: (clientX, clientY) => {
      seekbarPointerClientX = clientX;
      seekbarPointerClientY = clientY;
    },
    getSeekbarHoverSyncFrame: () => seekbarHoverSyncFrame,
    setSeekbarHoverSyncFrame: (value) => {
      seekbarHoverSyncFrame = value;
    },
    getSeekbarDuration,
    getPlaybackCurrentTime,
    normalizeSeekbarPreviewTime,
    resetThumbnails: (resetCache) => {
      seekbarThumbnailRuntime.reset(resetCache);
    },
    pausePlayback: pausePrimaryPlayback,
    playPlayback: playPrimaryPlayback,
    commitPreviewTime: (time) => {
      setPreviewTime(time);
    },
    resetFastForward: async (reason) => {
      await resetRightArrowFastForward(false, reason);
    },
  });
  const timelineZoomRuntime = createTimelineZoomRuntime({
    gestureScaleDeltaPerStep: TIMELINE_GESTURE_SCALE_DELTA_PER_STEP,
    gestureWheelDeltaPerStep: TIMELINE_GESTURE_WHEEL_DELTA_PER_STEP,
    gestureCommitDelayMs: TIMELINE_GESTURE_COMMIT_DELAY_MS,
    seekbarPbpViewBoxWidth: SEEKBAR_PBP_VIEWBOX_WIDTH,
    getWindowObject: getBrowserWindow,
    getTimelineElement: () => timelineElement,
    getTimelineContainer: () => timelineContainer,
    getTimelineSliderValue: () => timelineSliderValue,
    setTimelineSliderValue: (value) => {
      timelineSliderValue = value;
    },
    getTimelineScale: () => timelineScale,
    setTimelineScale: (value) => {
      timelineScale = value;
    },
    getTimelineWidth: () => timelineWidth,
    setTimelineWidth: (value) => {
      timelineWidth = value;
    },
    getTimelineZoomSteps: () => timelineZoomSteps,
    getSeekbarPbpViewBoxX: () => seekbarPbpViewBoxX,
    setSeekbarPbpViewBoxX: (value) => {
      seekbarPbpViewBoxX = value;
    },
    setSeekbarPbpViewBoxWidth: (value) => {
      seekbarPbpViewBoxWidth = value;
    },
    incrementSeekbarPbpZoomVersion: () => {
      seekbarPbpZoomVersion += 1;
    },
    getWaveformZoomInputThreshold: () => waveformZoomInputThreshold,
    snapTimelineSliderValue,
    getTimelineScaleForSliderValue,
    getNearestTimelineZoomNotchIndex,
    getTimelineZoomNotchValue,
    refreshSeekbarMetrics,
    updateTimeMarkers,
    scheduleSeekbarHoverSync,
    scheduleSeekbarHoverSyncFromLastPoint,
    syncSeekbarHoverFromClientPoint,
    rememberSeekbarPointerClientPoint,
    commitWaveformScale,
  });
  const clipWindowPlatformRuntime = createClipWindowPlatformRuntime({
    tauriEnv: TAURI_ENV,
    state: clipWindowPlatformState,
    getShow: () => show,
    getIsWindowWide: () => isWindowWide,
    setIsWindowWide: (value) => {
      isWindowWide = value;
    },
    getIsDocumentFullscreen: () => isDocumentFullscreen,
    setIsDocumentFullscreen: (value) => {
      isDocumentFullscreen = value;
    },
    focusPreviewStage,
    scheduleTimelineRefresh,
    syncPreviewLayoutAfterUiChange,
    updateVideoDisplayMetrics,
  });
  let rightArrowHoldTimeout: ReturnType<typeof setTimeout> | null = null;
  let isRightArrowFastForwardActive = false;
  let rightArrowFastForwardWasPlaying = false;
  let rightArrowPreviousPlaybackRate = 1;
  const playbackHotkeyRuntime = createPlaybackHotkeyRuntime({
    fastForwardHoldDelayMs: FAST_FORWARD_HOLD_DELAY_MS,
    fastForwardPlaybackRate: FAST_FORWARD_PLAYBACK_RATE,
    getShow: () => show,
    getIsVideoLoaded: () => isVideoLoaded,
    getVideoElement: () => videoElement,
    getVideo: () => video,
    getIsPlaying: () => isPlaying,
    setIsPlaying: (value) => {
      isPlaying = value;
    },
    getCurrentPlaybackRate: () => currentPlaybackRate,
    setCurrentPlaybackRateValue: (value) => {
      currentPlaybackRate = value;
    },
    getSelectedPlaybackRate,
    getIsRightArrowFastForwardActive: () => isRightArrowFastForwardActive,
    setIsRightArrowFastForwardActive: (value) => {
      isRightArrowFastForwardActive = value;
    },
    getRightArrowFastForwardWasPlaying: () => rightArrowFastForwardWasPlaying,
    setRightArrowFastForwardWasPlaying: (value) => {
      rightArrowFastForwardWasPlaying = value;
    },
    getRightArrowPreviousPlaybackRate: () => rightArrowPreviousPlaybackRate,
    setRightArrowPreviousPlaybackRate: (value) => {
      rightArrowPreviousPlaybackRate = value;
    },
    getRightArrowHoldTimeout: () => rightArrowHoldTimeout,
    setRightArrowHoldTimeout: (value) => {
      rightArrowHoldTimeout = value;
    },
    usingMacOSNativePlayer,
    playPrimaryPlayback,
    pausePrimaryPlayback,
    getPlaybackCurrentTime,
    setPreviewTime,
    syncWaveformWithVideo,
    setCurrentPlaybackRate,
    canBeClipped: (currentVideo) => canBeClipped(currentVideo as VideoItem),
    setClipStartTime,
    setClipEndTime,
    seekToClipStart,
    seekToClipEnd,
    generateClip: openClipGenerateModal,
    clearClipSelection,
    getShowDetail: () => show_detail,
    setShowDetail: (value) => {
      show_detail = value;
    },
  });
  function toggleDanmuFontMenu() {
    if (!danmuEnabled) {
      return;
    }
    danmuFontMenuController.open();
  }

  function handleDanmuFontSelect(fontFamily: string) {
    if (!fontFamily) {
      return;
    }
    danmakuStyle.fontFamily = fontFamily;
    danmuFontMenuController.close();
  }

  function togglePbpMethodMenu() {
    pbpMethodMenuController.open();
  }

  function handleSeekbarPbpMethodSelect(method: SeekbarPbpGenerationMethod) {
    if (seekbarPbpGenerationMethod === method) {
      pbpMethodMenuController.close();
      return;
    }
    seekbarPbpGenerationMethod = method;
    seekbarPbpZoomVersion += 1;
    pbpMethodMenuController.close();
  }

  function handleDanmuToggle(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    danmuEnabled = target?.checked ?? false;
    if (!danmuEnabled) {
      clearActiveDanmus();
      return;
    }
    rebuildDanmuPlayback(Math.floor(getPlaybackCurrentTime() * 1000));
  }

  function handleRenderDanmuEmotesToggle(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    renderDanmuEmotes = target?.checked ?? false;
    if (danmuEnabled) {
      rebuildDanmuPlayback(Math.floor(getPlaybackCurrentTime() * 1000));
    }
  }

  function handleDanmuPreventSubtitleOcclusionToggle(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    danmuPreventSubtitleOcclusionEnabled = target?.checked ?? true;
    if (danmuEnabled) {
      rebuildDanmuPlayback(Math.floor(getPlaybackCurrentTime() * 1000));
    }
  }

  function handleDanmuSyncWithPlaybackRateToggle(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    danmuSyncWithPlaybackRateEnabled = target?.checked ?? true;
  }

  function handleDanmuBoldToggle(event: Event) {
    const target = event.currentTarget as HTMLInputElement | null;
    danmakuStyle.bold = target?.checked ?? true;
  }

  let nextDanmuIndex = 0;
  let lastDanmuTimeMs = 0;
  let nextDanmuRenderId = 0;
  let recentDanmuPositions: number[] = [];
  let lastDanmuVideoId = -1;
  let danmuEnabled = true;
  let renderDanmuEmotes = true;
  let danmuPreventSubtitleOcclusionEnabled = true;
  let danmuSyncWithPlaybackRateEnabled = true;
  let selectedPlaybackRate = 1;
  let currentPlaybackRate = 1;
  let danmakuStyle: DanmakuStyle = loadDanmakuStyle(roomId);
  let danmakuDisplayAreaIndex = 4;
  let danmakuSpeedPresetIndex = 2;
  let danmakuMaxOnScreenIndex = 0;
  let lastSavedDanmakuStyleSignature = "";
  let danmakuEmoteMap: DanmakuEmoteMap = {};
  const previewScaleBaseHeight = 720;
  const waveformZoomInputThreshold = 24;
  let danmakuBaseFontPx = DANMAKU_FONT_SIZE_PX * danmakuStyle.fontScale;
  let videoDanmuFontSize = `${danmakuBaseFontPx}px`;
  let videoDanmuFontFamily = danmakuStyle.fontFamily || DANMAKU_FONT_FAMILY;
  let videoDanmuFontWeight = danmakuStyle.bold ? DANMAKU_FONT_WEIGHT : "normal";
  const videoDanmuLineHeight = `${DANMAKU_LINE_HEIGHT}`;
  const videoDanmuEmoteScale = `${DANMAKU_EMOTE_SCALE}`;
  const videoDanmuEmoteOffset = `${DANMAKU_EMOTE_VERTICAL_OFFSET_EM}em`;
  let videoDanmuOpacity = `${danmakuStyle.opacity}`;
  const videoDanmuTextShadow = DANMAKU_TEXT_SHADOW;
  let lastDanmuLayoutSignature = "";
  let danmuLayoutConfig: DanmuLayoutConfig = {
    containerWidthPx: 1280,
    containerHeightPx: 720,
    fontSizePx: DANMAKU_FONT_SIZE_PX,
    lineHeight: DANMAKU_LINE_HEIGHT,
    displayArea: danmakuStyle.displayArea,
    speedPreset: danmakuStyle.speedPreset,
    maxOnScreen: danmakuStyle.maxOnScreen,
    preventSubtitleOcclusion: danmuPreventSubtitleOcclusionEnabled,
  };
  let danmuCanonicalLookbackConfig: DanmuLayoutConfig = {
    ...danmuLayoutConfig,
  };
  let danmuActiveDurationMs = DANMU_ACTIVE_DURATION_MS;
  let danmuAnimationRate = 1;
  $: timelineSliderValue = getTimelineSliderValue();
  $: showSubtitleTimelineEffective =
    showSubtitleTimeline && subtitles.length > 0;
  $: showSubtitleTimelineLayoutVisible =
    showSubtitleTimelineEffective || reserveSubtitleTimelineLayoutDuringResize;
  $: showWaveformLayoutVisible =
    showWaveform || reserveWaveformLayoutDuringResize;
  $: danmakuBaseFontPx = DANMAKU_FONT_SIZE_PX * danmakuStyle.fontScale;
  $: videoDanmuOpacity = `${danmakuStyle.opacity}`;
  $: videoDanmuFontFamily = danmakuStyle.fontFamily || DANMAKU_FONT_FAMILY;
  $: videoDanmuFontWeight = danmakuStyle.bold ? DANMAKU_FONT_WEIGHT : "normal";
  $: {
    const nextDisplayAreaIndex = DANMAKU_DISPLAY_AREA_OPTIONS.indexOf(
      danmakuStyle.displayArea,
    );
    if (
      nextDisplayAreaIndex >= 0 &&
      danmakuDisplayAreaIndex !== nextDisplayAreaIndex
    ) {
      danmakuDisplayAreaIndex = nextDisplayAreaIndex;
    }
  }
  $: {
    const clampedIndex = Math.max(
      0,
      Math.min(
        DANMAKU_DISPLAY_AREA_OPTIONS.length - 1,
        Math.round(danmakuDisplayAreaIndex),
      ),
    );
    const mappedDisplayArea = DANMAKU_DISPLAY_AREA_OPTIONS[clampedIndex];
    if (danmakuStyle.displayArea !== mappedDisplayArea) {
      danmakuStyle.displayArea = mappedDisplayArea;
    }
    if (danmakuDisplayAreaIndex !== clampedIndex) {
      danmakuDisplayAreaIndex = clampedIndex;
    }
  }
  $: {
    const nextSpeedPresetIndex = Math.max(
      0,
      Math.min(
        DANMAKU_SPEED_PRESET_OPTIONS.length - 1,
        Math.round(danmakuStyle.speedPreset),
      ),
    );
    if (danmakuSpeedPresetIndex !== nextSpeedPresetIndex) {
      danmakuSpeedPresetIndex = nextSpeedPresetIndex;
    }
  }
  $: {
    const clampedIndex = Math.max(
      0,
      Math.min(
        DANMAKU_SPEED_PRESET_OPTIONS.length - 1,
        Math.round(danmakuSpeedPresetIndex),
      ),
    );
    if (danmakuStyle.speedPreset !== clampedIndex) {
      danmakuStyle.speedPreset = clampedIndex as DanmakuStyle["speedPreset"];
    }
    if (danmakuSpeedPresetIndex !== clampedIndex) {
      danmakuSpeedPresetIndex = clampedIndex;
    }
  }
  $: {
    const nextMaxOnScreenIndex = DANMAKU_MAX_ON_SCREEN_OPTIONS.indexOf(
      danmakuStyle.maxOnScreen,
    );
    if (
      nextMaxOnScreenIndex >= 0 &&
      danmakuMaxOnScreenIndex !== nextMaxOnScreenIndex
    ) {
      danmakuMaxOnScreenIndex = nextMaxOnScreenIndex;
    }
  }
  $: {
    const clampedIndex = Math.max(
      0,
      Math.min(
        DANMAKU_MAX_ON_SCREEN_OPTIONS.length - 1,
        Math.round(danmakuMaxOnScreenIndex),
      ),
    );
    const mappedMaxOnScreen = DANMAKU_MAX_ON_SCREEN_OPTIONS[clampedIndex];
    if (danmakuStyle.maxOnScreen !== mappedMaxOnScreen) {
      danmakuStyle.maxOnScreen =
        mappedMaxOnScreen as DanmakuStyle["maxOnScreen"];
    }
    if (danmakuMaxOnScreenIndex !== clampedIndex) {
      danmakuMaxOnScreenIndex = clampedIndex;
    }
  }
  $: {
    const signature = [
      roomId,
      danmakuStyle.fontScale.toFixed(3),
      danmakuStyle.opacity.toFixed(3),
      danmakuStyle.displayArea,
      danmakuStyle.speedPreset,
      danmakuStyle.maxOnScreen,
      danmakuStyle.bold ? 1 : 0,
      danmakuStyle.fontFamily,
    ].join("|");
    if (roomId && signature !== lastSavedDanmakuStyleSignature) {
      lastSavedDanmakuStyleSignature = signature;
      saveDanmakuStyle(roomId, danmakuStyle);
    }
  }
  $: previewDisplayHeight =
    videoDisplayHeight || videoHeight || previewScaleBaseHeight;
  $: videoDanmuFontSize = `${previewDisplayHeight * (danmakuBaseFontPx / previewScaleBaseHeight)}px`;
  $: danmuLayoutConfig = {
    containerWidthPx: previewVideoFrameWidth || videoWidth || 1280,
    containerHeightPx: previewVideoFrameHeight || previewDisplayHeight || 720,
    fontSizePx: previewDisplayHeight * (danmakuBaseFontPx / previewScaleBaseHeight),
    lineHeight: DANMAKU_LINE_HEIGHT,
    displayArea: danmakuStyle.displayArea,
    speedPreset: danmakuStyle.speedPreset,
    maxOnScreen: danmakuStyle.maxOnScreen,
    preventSubtitleOcclusion: danmuPreventSubtitleOcclusionEnabled,
  };
  $: danmuCanonicalLookbackConfig = {
    ...danmuLayoutConfig,
    containerWidthPx: 1280,
    containerHeightPx: 720,
    fontSizePx: danmakuBaseFontPx,
  };
  $: danmuActiveDurationMs = Math.max(
    getDanmuMaxActiveDurationMs({
      layout: danmuLayoutConfig,
    }),
    getDanmuMaxActiveDurationMs({
      layout: danmuCanonicalLookbackConfig,
    }),
  );
  $: danmuAnimationRate = Math.max(
    danmuSyncWithPlaybackRateEnabled ? currentPlaybackRate : 1,
    0.1,
  );
  $: {
    const layoutSignature = [
      danmuLayoutConfig.containerWidthPx.toFixed(2),
      danmuLayoutConfig.containerHeightPx.toFixed(2),
      danmuLayoutConfig.fontSizePx.toFixed(2),
      danmuLayoutConfig.displayArea,
      danmuLayoutConfig.speedPreset,
      danmuLayoutConfig.maxOnScreen,
      danmuLayoutConfig.preventSubtitleOcclusion ? 1 : 0,
      renderDanmuEmotes ? 1 : 0,
    ].join("|");
    if (
      danmuEnabled &&
      danmuRecords.length > 0 &&
      lastDanmuLayoutSignature &&
      layoutSignature !== lastDanmuLayoutSignature
    ) {
      rebuildDanmuPlayback(Math.floor(getPlaybackCurrentTime() * 1000));
    }
    lastDanmuLayoutSignature = layoutSignature;
  }
  $: {
    const duration = videoElement?.duration ?? 0;
    if (duration > 0) {
      const finestVisibleSeconds = Math.min(duration / 10, 120);
      maxTimelineScale = Math.max(1, duration / finestVisibleSeconds);
    } else {
      maxTimelineScale = 10;
    }

    if (timelineScale > maxTimelineScale) {
      timelineScale = maxTimelineScale;
    }

    if (waveformScale > maxTimelineScale) {
      waveformScale = maxTimelineScale;
    }
  }

  function getVideoDanmuEmoteStyle(
    segments: DanmakuSegment[],
    index: number,
  ): string {
    const spacing = getDanmakuEmoteTextGap(segments, index);
    return `margin-left: ${spacing.marginLeftEm}em; margin-right: ${spacing.marginRightEm}em;`;
  }

  function updateVideoDisplayMetrics(reason = "unknown") {
    void syncFullscreenState();

    if (arePreviewDisplayMetricsFrozen) {
      return;
    }
    const metrics = resolvePreviewDisplayMetrics({
      previewStageElement,
      videoElement,
      videoWidth,
      videoHeight,
    });
    previewStageViewportTop = metrics.previewStageViewportTop;
    previewStageViewportLeft = metrics.previewStageViewportLeft;
    previewStageViewportWidth = metrics.previewStageViewportWidth;
    previewStageViewportHeight = metrics.previewStageViewportHeight;
    previewVideoFrameTop = metrics.previewVideoFrameTop;
    previewVideoFrameLeft = metrics.previewVideoFrameLeft;
    previewVideoFrameWidth = metrics.previewVideoFrameWidth;
    previewVideoFrameHeight = metrics.previewVideoFrameHeight;
    videoDisplayHeight = metrics.videoDisplayHeight;

    if (isMacOSNativePlayerActive) {
      void macOSNativeClipPlayerRuntime.syncBounds(
        false,
        `display-metrics:${reason}`,
      );
    }
  }

  function watchVideoDisplayMetrics() {
    const observedElement = previewStageElement ?? videoElement;
    if (!observedElement) {
      return;
    }

    if (videoResizeObserver) {
      videoResizeObserver.disconnect();
    }

    if (typeof ResizeObserver !== "undefined") {
      videoResizeObserver = new ResizeObserver(() => {
        updateVideoDisplayMetrics("resize-observer");
      });
      videoResizeObserver.observe(observedElement);
    }

    updateVideoDisplayMetrics("watch-init");
  }

  $: stablePreviewVideoFrameStyle =
    previewVideoFrameWidth > 0 && previewVideoFrameHeight > 0
      ? `top:${previewStageViewportTop + previewVideoFrameTop}px;left:${previewStageViewportLeft + previewVideoFrameLeft}px;width:${previewVideoFrameWidth}px;height:${previewVideoFrameHeight}px;`
      : "display:none;";

  function getStablePreviewStageOverlayStyle() {
    return `top:${previewStageViewportTop + 8}px;left:${previewStageViewportLeft + 8}px;`;
  }

  function setPreviewDisplayMetricsFrozen(frozen: boolean) {
    arePreviewDisplayMetricsFrozen = frozen;
  }

  function lockPreviewStageHeightForPanelTransition() {
    if (previewStageHeightLockPx !== null || !previewStageElement) {
      return;
    }
    const rect = previewStageElement.getBoundingClientRect();
    if (!Number.isFinite(rect.height) || rect.height <= 0) {
      return;
    }
    previewStageHeightLockPx = Number(rect.height.toFixed(2));
  }

  function unlockPreviewStageHeightForPanelTransition() {
    if (previewStageHeightLockPx === null) {
      return;
    }
    previewStageHeightLockPx = null;
  }

  function cancelPreviewLayoutSync() {
    cancelPreviewLayoutSyncRuntime(clipWindowLayoutState);
  }

  async function syncPreviewLayoutAfterUiChange() {
    await syncPreviewLayoutAfterUiChangeRuntime({
      state: clipWindowLayoutState,
      tick,
      updateVideoDisplayMetrics,
    });
  }

  function suppressClipWindowResizeRefresh(durationMs: number) {
    suppressClipWindowResizeRefreshRuntime({
      tauriEnv: TAURI_ENV,
      durationMs,
      state: clipWindowLayoutState,
    });
  }

  function shouldSuppressClipWindowResizeRefresh() {
    return shouldSuppressClipWindowResizeRefreshRuntime(clipWindowLayoutState);
  }

  function getAdaptiveClipWindowHeightForState(
    subtitleTimelineVisible: boolean,
    waveformVisible: boolean,
  ) {
    return getAdaptiveClipWindowHeightForStateRuntime({
      baseHeight: CLIP_WINDOW_BASE_HEIGHT,
      subtitleExtraHeight: CLIP_WINDOW_SUBTITLE_TIMELINE_EXTRA_HEIGHT,
      waveformExtraHeight: CLIP_WINDOW_WAVEFORM_EXTRA_HEIGHT,
      subtitleTimelineVisible,
      waveformVisible,
    });
  }

  function getAdaptiveClipWindowHeight() {
    return getAdaptiveClipWindowHeightForState(
      showSubtitleTimelineEffective,
      showWaveform,
    );
  }

  function cancelClipWindowHeightSync() {
    cancelClipWindowHeightSyncRuntime(clipWindowLayoutState);
  }

  function syncClipWindowHeightAfterLayout(
    force = false,
    targetHeightOverride?: number,
  ) {
    void (async () => {
      await tick();
      await syncClipWindowHeight(force, targetHeightOverride);
    })();
  }

  async function syncClipWindowHeight(
    force = false,
    targetHeightOverride?: number,
  ) {
    await syncClipWindowHeightRuntime({
      tauriEnv: TAURI_ENV,
      show,
      isWindowWide,
      isWebFullscreen,
      isDocumentFullscreen,
      targetHeight: Math.round(
        targetHeightOverride ?? getAdaptiveClipWindowHeight(),
      ),
      thresholdPx: CLIP_WINDOW_HEIGHT_SYNC_THRESHOLD_PX,
      resizeKeepTopLeft: (width, height) =>
        invoke("macos_native_player_set_host_window_inner_size_keep_top_left", {
          width,
          height,
        }),
    });
  }

  function usingMacOSNativePlayer() {
    return usingMacOSNativePlayerValue;
  }

  function shouldUseMacOSNativePlayer() {
    return shouldUseMacOSNativePlayerValue;
  }

  function shouldPreferMacOSNativePlayerPresentation() {
    return shouldPreferMacOSNativePlayerPresentationValue;
  }

  async function ensureMacOSNativePlayerMounted(reason: string) {
    void reason;

    if (
      macOSNativePlayerMountPending ||
      !show ||
      !isVideoLoaded ||
      !shouldUseMacOSNativePlayer() ||
      isMacOSNativePlayerActive ||
      !video?.file
    ) {
      return false;
    }

    macOSNativePlayerMountPending = true;

    try {
      await tick();

      if (
        !show ||
        !isVideoLoaded ||
        !shouldUseMacOSNativePlayer() ||
        isMacOSNativePlayerActive ||
        !video?.file
      ) {
        return false;
      }

      return await macOSNativeClipPlayerRuntime.mount();
    } finally {
      macOSNativePlayerMountPending = false;
    }
  }

  $: shouldUseMacOSNativePlayerValue =
    TAURI_ENV &&
    macOSNativePlayerAvailable &&
    (config?.use_native_clip_player ?? true);

  $: shouldRenderMacOSNativePlayerPresentationValue =
    shouldUseMacOSNativePlayerValue && !macOSNativePlayerMountFailed;

  $: shouldPreferMacOSNativePlayerPresentationValue =
    (shouldUseMacOSNativePlayerValue && !macOSNativePlayerMountFailed) ||
    forceDomPreviewHidden ||
    isMacOSNativePlayerActive;

  $: showMacOSNativePlayerUnderlayValue =
    shouldRenderMacOSNativePlayerPresentationValue && isMacOSNativePlayerActive;

  $: usingMacOSNativePlayerValue =
    shouldUseMacOSNativePlayerValue && isMacOSNativePlayerActive;

  $: if (
    show &&
    isVideoLoaded &&
    shouldUseMacOSNativePlayerValue &&
    !isMacOSNativePlayerActive &&
    !macOSNativePlayerMountPending &&
    video?.file
  ) {
    void ensureMacOSNativePlayerMounted("reactive-ensure");
  }

  $: isVolumeMutedValue = isMuted || volume === 0;

  $: if (lastIsPlayingState !== isPlaying) {
    playIconSeed += 1;
    lastIsPlayingState = isPlaying;
  }

  $: if (lastVolumeMutedState !== isVolumeMutedValue) {
    volumeIconSeed += 1;
    lastVolumeMutedState = isVolumeMutedValue;
  }

  $: showFastForwardOverlayValue =
    rightArrowHoldTimeout !== null || isRightArrowFastForwardActive;

  $: canAdjustPlaybackRateValue = !(
    isRightArrowFastForwardActive || rightArrowHoldTimeout !== null
  );

  $: {
    const isFastForwarding =
      isRightArrowFastForwardActive || rightArrowHoldTimeout !== null;
    const selectedRate = Math.max(0.1, selectedPlaybackRate || 1);
    const runtimeRate = Math.max(0.1, currentPlaybackRate || 1);
    playbackRateDisplayLabelValue = computePlaybackRateButtonLabel({
      selectedRate,
      runtimeRate,
      isFastForwarding,
      fastForwardPlaybackRate: FAST_FORWARD_PLAYBACK_RATE,
    });
  }

  $: {
    videoElement;
    isVideoLoaded;
    playbackDurationValue = Math.max(0, videoElement?.duration ?? 0);
  }

  $: {
    const duration = getSeekbarDuration();
    const playbackTime = isDraggingSeekbar ? previewTime : currentTime;
    const pointerTime = isDraggingSeekbar ? previewTime : seekbarHoverTime;

    seekbarCurrentRatioValue =
      duration > 0 ? clamp01(playbackTime / duration) : 0;
    seekbarCurrentXValue = seekbarMetricsWidth * seekbarCurrentRatioValue;

    seekbarPointerRatioValue =
      duration > 0 ? clamp01(pointerTime / duration) : 0;
    seekbarPointerXValue = seekbarMetricsWidth * seekbarPointerRatioValue;

    showSeekbarPopupValue = isDraggingSeekbar || isSeekbarHovering;
    showSeekbarMoveIndicatorValue = isSeekbarHovering && !isDraggingSeekbar;
    seekbarPopupTimeValue =
      duration > 0 ? normalizeSeekbarPreviewTime(pointerTime, duration) : 0;

    const maxPopupLeft = Math.max(
      0,
      seekbarMetricsWidth - SEEKBAR_PREVIEW_WIDTH,
    );
    seekbarPopupLeftValue = Math.max(
      0,
      Math.min(maxPopupLeft, seekbarPointerXValue - SEEKBAR_PREVIEW_WIDTH / 2),
    );
  }

  $: seekbarPopupViewportLeftValue =
    seekbarViewportLeft + seekbarPopupLeftValue;
  $: seekbarPopupViewportTopValue =
    seekbarViewportTop +
    seekbarViewportHeight -
    SEEKBAR_PULL_INDICATOR_SIZE -
    SEEKBAR_PREVIEW_HEIGHT;
  $: seekbarPbpPlayedWidth =
    clamp01(seekbarCurrentRatioValue) * SEEKBAR_PBP_VIEWBOX_WIDTH;
  $: seekbarPbpVisible =
    showPbpOverlay &&
    getSeekbarDuration() > 0 &&
    (danmuRecords.length > 0 || hasBilibiliPbpData);
  $: if (timelineContainer) {
    syncTimelineScrollMetrics();
  }
  $: seekbarPbpViewBoxValue = `${seekbarPbpViewBoxX.toFixed(3)} 0 ${seekbarPbpViewBoxWidth.toFixed(3)} ${SEEKBAR_PBP_VIEWBOX_HEIGHT}`;
  $: {
    const duration = getSeekbarDuration();
    const durationKey = Number.isFinite(duration) ? duration.toFixed(3) : "0";
    const firstTs = danmuRecords[0]?.ts ?? -1;
    const lastTs = danmuRecords[danmuRecords.length - 1]?.ts ?? -1;
    const densityCacheKey = [
      video?.id ?? "none",
      durationKey,
      seekbarPbpGenerationMethod,
      danmuRecords.length,
      firstTs,
      lastTs,
      hasBilibiliPbpData ? `${bilibiliPbpData?.cid}:${bilibiliPbpData?.values.length}` : "no-pbp",
    ].join("|");
    if (
      seekbarPbpGenerationMethod !== "bilibili_pbp" &&
      densityCacheKey !== seekbarPbpGlobalMaxDensityCacheKey
    ) {
      seekbarPbpGlobalMaxDensityCacheKey = densityCacheKey;
      seekbarPbpGlobalMaxDensity = resolveSeekbarPbpGlobalMaxDensity(
        danmuRecords,
        duration,
        seekbarPbpGenerationMethod,
      );
    }

    const timelineScaleKey =
      Number.isFinite(timelineScale) && timelineScale > 0
        ? timelineScale.toFixed(4)
        : "1";
    const sampleCount = resolveSeekbarPbpSampleCount(duration, timelineScale);
    const nextCacheKey = [
      densityCacheKey,
      timelineScaleKey,
      sampleCount,
      seekbarPbpZoomVersion,
      seekbarPbpGlobalMaxDensity.toFixed(4),
    ].join("|");
    if (nextCacheKey !== seekbarPbpCurveCacheKey) {
      seekbarPbpCurveCacheKey = nextCacheKey;
      seekbarPbpCurvePath =
        seekbarPbpGenerationMethod === "bilibili_pbp" && hasBilibiliPbpData
          ? buildBilibiliSeekbarPbpCurvePath(bilibiliPbpData, duration)
          : buildSeekbarPbpCurvePath(
              danmuRecords,
              duration,
              timelineScale,
              seekbarPbpGlobalMaxDensity,
              seekbarPbpGenerationMethod,
            );
    }
  }

  $: if (showSeekbarPopupValue) {
    seekbarThumbnailRuntime.queueForTime(seekbarPopupTimeValue);
  }

  $: effectiveVolumePercentValue = Math.round(
    Math.max(0, Math.min(1, isMuted ? 0 : volume)) * 100,
  );

  $: macOSNativePlayerWindowedPreviewCompensationValue = Math.round(
    config?.native_clip_player_windowed_offset ??
      DEFAULT_MACOS_NATIVE_PLAYER_WINDOWED_PREVIEW_COMPENSATION,
  );

  $: threePlayrateHintTopValue =
    18 +
    (showMacOSNativePlayerUnderlayValue && !isDocumentFullscreen
      ? macOSNativePlayerWindowedPreviewCompensationValue
      : 0);

  $: suspendMacOSNativePlayerUnderlayForPopupValue =
    MACOS_NATIVE_PLAYER_POPUP_FALLBACK_ENABLED &&
    (isPlaybackRateMenuVisible ||
      isPreviewDisplayMenuVisible ||
      isVolumeMenuVisible ||
      isDanmuFontMenuVisible);

  function shouldHideDomPreview() {
    return shouldHideMacOSNativeDomPreview({
      forceDomPreviewHidden,
      usingMacOSNativePlayer: usingMacOSNativePlayer(),
      shouldPreferMacOSNativePlayerPresentation:
        shouldPreferMacOSNativePlayerPresentation(),
    });
  }

  function getEffectiveVolume() {
    return isMuted ? 0 : volume;
  }

  function applyVolumeToPlayback() {
    if (videoElement) {
      videoElement.volume = getEffectiveVolume();
    }
    void macOSNativeClipPlayerRuntime.syncVolume();
  }

  function getSelectedPlaybackRate() {
    return Math.max(0.1, selectedPlaybackRate || 1);
  }

  function getPreviewDisplayMenuMetrics() {
    return resolveAnchoredMenuMetrics({
      locked: isPreviewDisplayMenuInteractionLocked,
      lockedRect: previewDisplayMenuLockedRect,
      anchor: previewSettingsControlElement ?? previewSettingsButtonElement,
      width: PREVIEW_DISPLAY_MENU_WIDTH,
      height: PREVIEW_DISPLAY_MENU_HEIGHT,
    });
  }

  function openPreviewDisplayMenu() {
    previewDisplayMenuController.open();
  }

  function handleSettingsMouseEnter() {
    if (isPreviewDisplayMenuInteractionLocked) {
      isSettingsHover = false;
      return;
    }
    isSettingsHover = true;
    openPreviewDisplayMenu();
  }

  function handleSettingsMouseLeave() {
    isSettingsHover = false;
    previewDisplayMenuController.scheduleClose();
  }

  function setPreviewDisplayMenuInteractionLocked(locked: boolean) {
    if (locked && isPreviewDisplayMenuVisible) {
      previewDisplayMenuLockedRect = captureElementRect(
        previewDisplayMenuElement,
      );
    }
    isPreviewDisplayMenuInteractionLocked = locked;
    previewDisplayMenuController.cancelHide();
    if (locked) {
      if (!isPreviewDisplayMenuVisible) {
        isSettingsHover = false;
      }
    } else {
      previewDisplayMenuLockedRect = null;
    }
  }

  function setPreviewOverlayLayersSuppressed(suppressed: boolean) {
    suppressPreviewOverlayLayersValue = suppressed;
  }

  async function togglePreviewDisplaySubtitleTimeline() {
    await tick();
    toggleSubtitleTimeline();
  }

  async function togglePreviewDisplayWaveform() {
    await tick();
    toggleWaveform();
  }

  async function togglePreviewDisplayPbp() {
    await tick();
    showPbpOverlay = !showPbpOverlay;
  }

  async function handlePbpOverlayMouseDown(event: MouseEvent) {
    await resetRightArrowFastForward(false, "pbp-click");
    event.preventDefault();
    event.stopPropagation();

    if (!videoElement) {
      return;
    }

    const target = event.currentTarget as HTMLElement | null;
    const rect = target?.getBoundingClientRect();
    if (!rect || rect.width <= 0) {
      return;
    }

    const clampedX = Math.max(
      0,
      Math.min(event.clientX - rect.left, rect.width),
    );
    const duration = getSeekbarDuration();
    if (!Number.isFinite(duration) || duration <= 0) {
      return;
    }

    const localRatio = clamp01(clampedX / rect.width);
    const visibleStartRatio = clamp01(
      seekbarPbpViewBoxX / SEEKBAR_PBP_VIEWBOX_WIDTH,
    );
    const visibleWidthRatio = Math.max(
      0.0001,
      Math.min(1, seekbarPbpViewBoxWidth / SEEKBAR_PBP_VIEWBOX_WIDTH),
    );
    const timeRatio = clamp01(
      visibleStartRatio + localRatio * visibleWidthRatio,
    );
    const time = timeRatio * duration;
    setPreviewTime(time);
    seekbarHoverTime = normalizeSeekbarPreviewTime(time, duration);
    rememberSeekbarPointerClientPoint(event.clientX, event.clientY);
  }

  async function syncFullscreenState() {
    await clipWindowPlatformRuntime.syncFullscreenState();
  }

  async function requestFullscreenLayoutRefresh() {
    await clipWindowPlatformRuntime.requestFullscreenLayoutRefresh();
  }

  async function waitForWindowBoundsStable(maxMs = 320, settleMs = 80) {
    return clipWindowPlatformRuntime.waitForWindowBoundsStable(maxMs, settleMs);
  }

  function handleViewportResize() {
    void requestFullscreenLayoutRefresh();
    scheduleTimelineRefresh();
  }

  function handleDocumentFullscreenChange() {
    void requestFullscreenLayoutRefresh();
  }

  function cancelFullscreenExitFocusRestore() {
    clipWindowPlatformRuntime.cancelFullscreenExitFocusRestore();
  }

  function restorePreviewHotkeyFocusAfterFullscreenExit() {
    clipWindowPlatformRuntime.restorePreviewHotkeyFocusAfterFullscreenExit();
  }

  function clearVolumeClickAnimationTimer() {
    const windowObject = getBrowserWindow();
    if (volumeClickAnimationTimer !== null && windowObject) {
      windowObject.clearTimeout(volumeClickAnimationTimer);
    }
    volumeClickAnimationTimer = null;
  }

  function triggerVolumeClickAnimation(nextMuted: boolean) {
    const windowObject = getBrowserWindow();
    if (!windowObject) {
      isVolumeClickAnimating = false;
      return;
    }

    clearVolumeClickAnimationTimer();
    isVolumeClickAnimating = true;
    const durationMs = getLottieDurationMs(
      nextMuted ? muteLottieData : volumeLottieData,
      500,
    );
    volumeClickAnimationTimer = windowObject.setTimeout(() => {
      isVolumeClickAnimating = false;
      volumeClickAnimationTimer = null;
    }, durationMs);
  }

  function handleVolumeMouseEnter() {
    isVolumeHover = true;
    volumeMenuController.open();
  }

  function handleVolumeMouseLeave() {
    isVolumeHover = false;
    isVolumeHoverSuppressed = false;
    volumeMenuController.scheduleClose();
  }
  function toggleWebFullscreen() {
    isWebFullscreen = !isWebFullscreen;
    scheduleTimelineRefresh();
    void syncPreviewLayoutAfterUiChange();
  }

  function clamp01(value: number) {
    return Math.max(0, Math.min(1, value));
  }

  function getLottieDurationMs(
    animationData: Record<string, unknown>,
    fallbackMs = 500,
  ) {
    const data = animationData as { fr?: number; ip?: number; op?: number };
    const frameRate = Number(data.fr);
    const outPoint = Number(data.op);
    const inPoint = Number(data.ip ?? 0);
    if (
      Number.isFinite(frameRate) &&
      frameRate > 0 &&
      Number.isFinite(outPoint)
    ) {
      const durationFrames = Math.max(0, outPoint - inPoint);
      if (durationFrames > 0) {
        return Math.max(80, Math.round((durationFrames / frameRate) * 1000));
      }
    }
    return fallbackMs;
  }

  async function toggleFullscreen() {
    await clipWindowPlatformRuntime.toggleFullscreen();
  }

  async function toggleWindowWide() {
    await clipWindowPlatformRuntime.toggleWindowWide();
  }

  function applySelectedPlaybackRate(refreshDanmu = true) {
    const nextRate = getSelectedPlaybackRate();

    if (usingMacOSNativePlayer() && !isPlaying) {
      currentPlaybackRate = nextRate;

      if (refreshDanmu && danmuEnabled) {
        rebuildDanmuPlayback(Math.floor(getPlaybackCurrentTime() * 1000));
      }
      return;
    }

    setCurrentPlaybackRate(nextRate, refreshDanmu);
  }

  function handlePlaybackRateSelect(nextRate: number) {
    if (!Number.isFinite(nextRate)) {
      return;
    }

    selectedPlaybackRate = nextRate;

    if (!canAdjustPlaybackRateValue) {
      return;
    }

    applySelectedPlaybackRate(false);
    playbackRateMenuController.close();
  }

  $: if (!canAdjustPlaybackRateValue && isPlaybackRateMenuVisible) {
    playbackRateMenuController.close();
  }

  $: if (
    (!danmuEnabled || activeTab !== "danmu" || !show) &&
    isDanmuFontMenuVisible
  ) {
    danmuFontMenuController.close();
  }

  $: if ((activeTab !== "danmu" || !show) && isPbpMethodMenuVisible) {
    pbpMethodMenuController.close();
  }

  $: if (!show && isVideoSelectMenuVisible) {
    videoSelectMenuController.close();
  }

  $: {
    forceDomPreviewHidden;
    isMacOSNativePlayerActive;
    macOSNativePlayerAvailable;
    macOSNativePlayerMountFailed;
    config;
    videoElement;
    macOSNativeClipPlayerRuntime.syncDomPreviewVisibility();
  }

  $: {
    suspendMacOSNativePlayerUnderlayForPopupValue;
    isMacOSNativePlayerActive;
    macOSNativePlayerAvailable;
    macOSNativePlayerMountFailed;
    void macOSNativeClipPlayerRuntime.syncPresentationMode(
      TAURI_ENV &&
        macOSNativePlayerAvailable &&
        isMacOSNativePlayerActive &&
        suspendMacOSNativePlayerUnderlayForPopupValue,
    );
  }

  function getPlaybackCurrentTime() {
    return Math.max(
      0,
      usingMacOSNativePlayer()
        ? currentTime
        : (videoElement?.currentTime ?? currentTime),
    );
  }

  function getSeekbarDuration() {
    return Math.max(0, playbackDurationValue || videoElement?.duration || 0);
  }

  function normalizeSeekbarPreviewTime(
    time: number,
    durationOverride = getSeekbarDuration(),
  ) {
    const duration = Math.max(0, durationOverride);
    if (!Number.isFinite(duration) || duration <= 0) {
      return 0;
    }
    if (!Number.isFinite(time)) {
      return 0;
    }
    return Math.max(0, Math.min(duration, time));
  }

  function resetSeekbarPreviewState(resetCache = false) {
    seekbarInteractionRuntime.resetSeekbarPreviewState(resetCache);
  }

  function refreshSeekbarMetrics() {
    seekbarInteractionRuntime.refreshSeekbarMetrics();
  }

  function rememberSeekbarPointerClientPoint(clientX: number, clientY: number) {
    seekbarInteractionRuntime.rememberSeekbarPointerClientPoint(
      clientX,
      clientY,
    );
  }

  function scheduleSeekbarHoverSync(clientX: number, clientY: number) {
    seekbarInteractionRuntime.scheduleSeekbarHoverSync(clientX, clientY);
  }

  function scheduleSeekbarHoverSyncFromLastPoint() {
    seekbarInteractionRuntime.scheduleSeekbarHoverSyncFromLastPoint();
  }

  function playPrimaryPlayback() {
    macOSNativePauseRequestPending = false;
    if (usingMacOSNativePlayer()) {
      const targetRate = isRightArrowFastForwardActive
        ? FAST_FORWARD_PLAYBACK_RATE
        : getSelectedPlaybackRate();
      void setMacOSNativePlayerRate(targetRate, MACOS_NATIVE_PLAYER_ID).catch((error) => {
        console.warn("Failed to start macOS native playback:", error);
      });
      return;
    }

    if (!videoElement) {
      return;
    }

    void videoElement.play().catch((error) => {
      console.warn("Failed to start playback:", error);
    });
  }

  function pausePrimaryPlayback() {
    if (usingMacOSNativePlayer()) {
      macOSNativePauseRequestPending = true;
      isPlaying = false;
      void pauseMacOSNativePlayer(MACOS_NATIVE_PLAYER_ID).catch((error) => {
        macOSNativePauseRequestPending = false;
        console.warn("Failed to pause macOS native playback:", error);
      });
      return;
    }

    videoElement?.pause();
  }

  function getMacOSNativePlayerWindowedYOffset() {
    return computeMacOSNativePlayerWindowedYOffset({
      tauriEnv: TAURI_ENV,
      isDocumentFullscreen,
      compensation: macOSNativePlayerWindowedPreviewCompensationValue,
    });
  }

  function getMacOSNativePlayerTargetRect() {
    return computeMacOSNativePlayerTargetRect({
      previewStageElement,
      previewVideoFrameLeft,
      previewVideoFrameTop,
      previewVideoFrameWidth,
      previewVideoFrameHeight,
      shouldUseMacOSNativePlayer: shouldUseMacOSNativePlayerValue,
      windowedYOffset: getMacOSNativePlayerWindowedYOffset(),
    });
  }

  function suspendMacOSNativePlayerBoundsSync(reason: string) {
    macOSNativeClipPlayerRuntime.suspendBoundsSync(reason);
  }

  function resumeMacOSNativePlayerBoundsSync(reason: string) {
    macOSNativeClipPlayerRuntime.resumeBoundsSync(reason);
  }

  // 获取 profile 从 localStorage
  function get_profile(): Profile {
    const profile_str = window.localStorage.getItem("profile-" + roomId);
    if (profile_str && profile_str.includes("videos")) {
      return JSON.parse(profile_str);
    }
    return default_profile();
  }

  let profile: Profile = get_profile();

  $: {
    window.localStorage.setItem("profile-" + roomId, JSON.stringify(profile));
  }

  // 初始化 WaveSurfer.js
  async function initWaveSurfer() {
    if (typeof window === "undefined" || !video?.file) return;

    isWaveformLoading = true;

    try {
      await createWaveSurfer();
    } catch (error) {
      console.error("Failed to initialize WaveSurfer.js:", error);
      isWaveformLoading = false;
    }
  }

  function ensureWaveformData(): Promise<AudioWaveformData> {
    const state = {
      waveformDataPromise,
      waveformDataVideoId,
    };
    const nextPromise = ensureWaveformDataRuntime({
      videoId: video?.id,
      state,
      invoke,
    });
    waveformDataPromise = state.waveformDataPromise;
    waveformDataVideoId = state.waveformDataVideoId;
    return nextPromise;
  }

  async function createWaveSurfer() {
    const container = document.querySelector(
      "[data-waveform-container]",
    ) as HTMLElement | null;

    try {
      const runtime = await createWaveSurferRuntime({
        container,
        videoFile: video?.file,
        showWaveform,
        panelHeightPx: WAVEFORM_PANEL_HEIGHT_PX,
        barHeightRatio: WAVEFORM_BAR_HEIGHT_RATIO,
        formatTimelineMarkerTime,
        ensureWaveformData,
        setPreviewTime,
        getVideoDuration: () => videoElement?.duration ?? 0,
        hasManagedClipSelection: (id) =>
          clipSelections.some((selection) => selection.id === id),
        hasPendingClipStartMarker: () => hasPendingClipStartMarker,
        setActiveClipSelection,
        updateClipSelectionFromRegion,
        syncClipWaveformRegionAppearance,
        syncClipWaveformRegions,
        onLoadingStateChange: (loading) => {
          isWaveformLoading = loading;
        },
        onReadyStateChange: (loaded) => {
          isWaveformLoaded = loaded;
        },
      });

      if (!runtime) {
        return;
      }

      wavesurfer = runtime.wavesurfer;
      waveformRegions = runtime.waveformRegions;
    } catch (error) {
      console.error("Failed to create WaveSurfer:", error);
    }
  }

  // 同步波形图与视频进度
  function syncWaveformWithVideo() {
    syncWaveformWithVideoRuntime({
      wavesurfer,
      videoDuration: videoElement?.duration ?? 0,
      isWaveformLoaded,
      currentTime: getPlaybackCurrentTime(),
    });
  }

  // 销毁 WaveSurfer 实例
  function destroyWaveSurfer() {
    const state = {
      wavesurfer,
      waveformRegions,
      isWaveformLoaded,
      isWaveformLoading,
      waveformRenderFrame,
    };
    destroyWaveSurferRuntime(state);
    wavesurfer = state.wavesurfer;
    waveformRegions = state.waveformRegions;
    isWaveformLoaded = state.isWaveformLoaded;
    isWaveformLoading = state.isWaveformLoading;
    waveformRenderFrame = state.waveformRenderFrame;
    clipStartMarkerRegion = null;
    clipSelectionRegions = {};
  }

  function createClipSelectionId() {
    const nextId = createClipSelectionIdRuntime({
      prefix: CLIP_SELECTION_REGION_ID_PREFIX,
      counter: clipSelectionIdCounter,
    });
    clipSelectionIdCounter = nextId.nextCounter;
    return nextId.id;
  }

  function syncClipWaveformRegionAppearance() {
    clipSelections.forEach((selection) => {
      const region = clipSelectionRegions[selection.id];
      if (!region) {
        return;
      }

      applyClipRegionLabel(region, selection.id === activeClipSelectionId);
    });
  }

  function updateClipSelectionFromRegion(region: any) {
    applyClipSelectionState(
      updateClipSelectionFromRegionRuntime({
        state: getClipSelectionState(),
        region,
      }),
    );
  }

  function setActiveClipSelection(id: string | null) {
    applyClipSelectionState(
      setActiveClipSelectionRuntime({
        state: getClipSelectionState(),
        id,
      }),
    );
    syncClipWaveformRegionAppearance();
  }

  function clearAllClipSelections() {
    applyClipSelectionState(
      clearAllClipSelectionsRuntime(getClipSelectionState()),
    );
    clipStartMarkerRegion = removeWaveformRegion(clipStartMarkerRegion);

    Object.keys(clipSelectionRegions).forEach((id) => {
      removeWaveformRegion(clipSelectionRegions[id]);
    });

    clipSelectionRegions = {};
  }

  function syncClipWaveformRegions() {
    if (hasPendingClipStartMarker && !clipRegionColor) {
      clipRegionColor = CLIP_REGION_COLOR;
    }

    const result = syncClipWaveformRegionsRuntime({
      waveformRegions,
      isWaveformLoaded,
      state: getClipSelectionState(),
      clipStartMarkerRegion,
      clipSelectionRegions,
      clipStartMarkerRegionId: CLIP_START_MARKER_REGION_ID,
    });
    clipStartMarkerRegion = result.clipStartMarkerRegion;
    clipSelectionRegions = result.clipSelectionRegions;
  }

  function clampTimelineScale(scale: number) {
    return Math.max(1, Math.min(scale, maxTimelineScale));
  }

  function getTimelineMinVisibleSeconds() {
    const duration = videoElement?.duration ?? 0;
    if (duration <= 0) {
      return 0;
    }

    return Math.min(duration / 10, 60);
  }

  function getTimelineSliderValueForScale(scale: number) {
    const duration = videoElement?.duration ?? 0;
    const minVisibleSeconds = getTimelineMinVisibleSeconds();

    if (duration <= 0 || duration <= minVisibleSeconds) {
      return 0;
    }

    const visibleSeconds = duration / clampTimelineScale(scale);
    const clampedVisibleSeconds = Math.max(
      minVisibleSeconds,
      Math.min(duration, visibleSeconds),
    );
    const zoomProgress =
      (duration - clampedVisibleSeconds) / (duration - minVisibleSeconds);

    return zoomProgress * timelineZoomSteps;
  }

  function getTimelineSliderValue() {
    return getTimelineSliderValueForScale(timelineScale);
  }

  function getTimelineScaleForSliderValue(sliderValue: number) {
    const duration = videoElement?.duration ?? 0;
    const minVisibleSeconds = getTimelineMinVisibleSeconds();

    if (duration <= 0 || duration <= minVisibleSeconds) {
      return 1;
    }

    const clampedSliderValue = Math.max(
      0,
      Math.min(timelineZoomSteps, sliderValue),
    );
    const zoomProgress = clampedSliderValue / timelineZoomSteps;
    const visibleSeconds =
      duration - zoomProgress * (duration - minVisibleSeconds);

    return clampTimelineScale(duration / visibleSeconds);
  }

  function snapTimelineSliderValue(sliderValue: number) {
    const clampedSliderValue = Math.max(
      0,
      Math.min(timelineZoomSteps, sliderValue),
    );
    let nearestValue = TIMELINE_ZOOM_NOTCH_VALUES[0] ?? clampedSliderValue;
    let nearestDistance = Math.abs(clampedSliderValue - nearestValue);

    for (const notchValue of TIMELINE_ZOOM_NOTCH_VALUES) {
      const distance = Math.abs(clampedSliderValue - notchValue);
      if (distance < nearestDistance) {
        nearestValue = notchValue;
        nearestDistance = distance;
      }
    }

    return nearestValue;
  }

  function getNearestTimelineZoomNotchIndex(sliderValue: number) {
    const clampedSliderValue = Math.max(
      0,
      Math.min(timelineZoomSteps, sliderValue),
    );
    let nearestIndex = 0;
    let nearestDistance = Infinity;

    for (let i = 0; i < TIMELINE_ZOOM_NOTCH_VALUES.length; i += 1) {
      const notchValue = TIMELINE_ZOOM_NOTCH_VALUES[i];
      const distance = Math.abs(clampedSliderValue - notchValue);
      if (distance < nearestDistance) {
        nearestIndex = i;
        nearestDistance = distance;
      }
    }

    return nearestIndex;
  }

  function getTimelineZoomNotchValue(index: number) {
    const clampedIndex = Math.max(
      0,
      Math.min(TIMELINE_ZOOM_NOTCH_VALUES.length - 1, Math.round(index)),
    );
    return TIMELINE_ZOOM_NOTCH_VALUES[clampedIndex] ?? 0;
  }

  async function redrawWaveformAtCurrentWidth() {
    await tick();
    await redrawWaveformAtCurrentWidthRuntime({
      showWaveform,
      wavesurfer,
      isWaveformLoaded,
      waveformRenderFrame,
      videoDuration: videoElement?.duration ?? 0,
      renderWidth: waveformContainer?.clientWidth ?? 0,
      currentTime,
      syncWaveformWithVideo,
      onFrameChange: (frame) => {
        waveformRenderFrame = frame;
      },
    });
  }

  async function commitWaveformScale() {
    waveformScale = clampTimelineScale(timelineScale);
    await redrawWaveformAtCurrentWidth();
  }

  // on window close, save subtitles
  onMount(async () => {
    await setupClipWindowLifecycle({
      state: clipWindowLifecycleState,
      tauriEnv: TAURI_ENV,
      loadDanmakuEmoteMap,
      onDanmakuEmoteMapLoaded: (value) => {
        danmakuEmoteMap = value as DanmakuEmoteMap;
      },
      logInfo: (message, payload) => {
        void log.info(message, payload);
      },
      logWarn: (message, payload) => {
        void log.warn(message, payload);
      },
      invoke,
      onConfigLoaded: (value) => {
        config = value as Config;
        if (show && isVideoLoaded) {
          void ensureMacOSNativePlayerMounted("config-loaded");
        }
      },
      getIsVideoLoaded: () => isVideoLoaded,
      prepareSeekbarThumbnails: () => {
        void seekbarThumbnailRuntime.prepare();
      },
      detectMacOSNativePlayerSupport,
      onMacOSNativePlayerSupportResolved: (available) => {
        macOSNativePlayerAvailable = available;
        if (available && show && isVideoLoaded) {
          void ensureMacOSNativePlayerMounted("support-resolved");
        }
      },
      shouldMountMacOSNativePlayer: () => shouldUseMacOSNativePlayer() && isVideoLoaded,
      tick,
      mountMacOSNativePlayer: async () => {
        await macOSNativeClipPlayerRuntime.mount();
      },
      onCloseCleanup: async () => {
        macOSNativeClipPlayerRuntime.stopBoundsFollowLoop();
        macOSNativeClipPlayerRuntime.stopPolling();
        isMacOSNativePlayerActive = false;
        forceDomPreviewHidden = false;
        macOSNativeClipPlayerRuntime.syncDomPreviewVisibility();
        macOSNativeClipPlayerRuntime.restoreBackdrop();
        await resetRightArrowFastForward(false, "close");
        pausePrimaryPlayback();
        isPlaying = false;
        await saveSubtitles();
      },
      updateVideoDisplayMetrics,
      shouldSuppressClipWindowResizeRefresh,
      getIsDocumentFullscreen: () => isDocumentFullscreen,
      getIsWebFullscreen: () => isWebFullscreen,
      requestFullscreenLayoutRefresh,
      addWindowEventListener: (type, listener) => {
        window.addEventListener(type, listener);
      },
      addDocumentEventListener: (type, listener) => {
        document.addEventListener(type, listener);
      },
      handleViewportResize,
      handleWindowFocus,
      handleDocumentFullscreenChange,
      syncClipWindowHeightAfterLayout,
      saveSubtitles,
      setAccounts: (value) => {
        accounts = value;
      },
    });
  });

  onDestroy(() => {
    window.removeEventListener("resize", handleViewportResize);
    window.removeEventListener("focus", handleWindowFocus);
    document.removeEventListener(
      "fullscreenchange",
      handleDocumentFullscreenChange,
    );
    disposeClipWindowLifecycle(clipWindowLifecycleState);
    playbackRateMenuController.close();
    previewDisplayMenuController.close();
    volumeMenuController.close();
    danmuFontMenuController.close();
    cancelClipWindowHeightSync();
    cancelFullscreenExitFocusRestore();
    clearVolumeClickAnimationTimer();
    void resetRightArrowFastForward(false, "destroy");
    macOSNativeClipPlayerRuntime.stopBoundsFollowLoop();
    macOSNativeClipPlayerRuntime.stopPolling();
    isMacOSNativePlayerActive = false;
    forceDomPreviewHidden = false;
    macOSNativeClipPlayerRuntime.restoreBackdrop();
    macOSNativeClipPlayerRuntime.syncDomPreviewVisibility();
    if (waveformRenderFrame !== null) {
      cancelAnimationFrame(waveformRenderFrame);
    }
    cancelPreviewLayoutSync();
    if (videoResizeObserver) {
      videoResizeObserver.disconnect();
      videoResizeObserver = null;
    }
    // 清理 WaveSurfer 实例
    destroyWaveSurfer();
    clearActiveDanmus();
    subtitleTimelineRuntime.cleanup();
    seekbarInteractionRuntime.cleanup();
    seekbarThumbnailRuntime.dispose();
    timelineZoomRuntime.cleanup();
  });

  function updateTaskPrompt(elementId: string, content: string) {
    const element = document.getElementById(elementId);
    if (element) {
      element.textContent = content;
    }
  }

  function update_encode_prompt(content: string) {
    updateTaskPrompt("encode-prompt", content);
  }

  function update_generate_prompt(content: string) {
    updateTaskPrompt("generate-prompt", content);
  }

  function update_post_prompt(content: string) {
    updateTaskPrompt("post-prompt", content);
  }

  // 投稿相关函数
  async function do_post() {
    if (!video) {
      return;
    }

    const eventId = generateEventId();
    current_post_event_id = eventId;
    update_post_prompt("投稿上传中");

    const clearUpdateListener = await listen(
      `progress-update:${eventId}`,
      (event) => {
        update_post_prompt(event.payload.content);
      },
    );

    const clearFinishedListener = await listen(
      `progress-finished:${eventId}`,
      (event) => {
        update_post_prompt("投稿");
        if (!event.payload.success) {
          alert(event.payload.message);
        }

        current_post_event_id = null;
        clearUpdateListener();
        clearFinishedListener();
      },
    );

    window.localStorage.setItem(`profile-${roomId}`, JSON.stringify(profile));

    invoke("upload_procedure", {
      uid: uid_selected,
      eventId,
      roomId,
      videoId: video.id,
      profile,
    }).then(async () => {
      uid_selected = 0;
      await onVideoListUpdate?.();
    });
  }

  async function cancel_post() {
    if (!current_post_event_id) {
      return;
    }

    await invoke("cancel", { eventId: current_post_event_id });
  }

  async function saveSubtitles() {
    await saveSubtitlesRuntime({
      videoFile: video?.file,
      videoId: video?.id,
      subtitles,
      invoke,
    });
  }

  async function generateSubtitles() {
    subtitles = await generateSubtitlesRuntime({
      videoFile: video?.file,
      videoId: video?.id,
      generateEventId,
      listen,
      invoke,
      setCurrentGenerateEventId: (value) => {
        current_generate_event_id = value;
      },
      updateGeneratePrompt: update_generate_prompt,
      reportError: (message) => {
        alert(message);
      },
    });
  }

  async function loadSubtitles() {
    subtitles = await loadSubtitlesRuntime({
      videoFile: video?.file,
      videoId: video?.id,
      invoke,
    });
  }

  async function loadDanmu() {
    danmuRecords = await loadDanmuRecords({
      invoke,
      videoId: video?.id,
    });
    resetDanmuPlayback();
  }

  async function loadBilibiliPbp() {
    if (!video?.id) {
      bilibiliPbpData = null;
      return;
    }

    try {
      bilibiliPbpData = await invoke<VideoPbpData | null>("get_video_pbp", {
        id: video.id,
      });
    } catch (error) {
      console.warn("Failed to load Bilibili PBP data:", error);
      bilibiliPbpData = null;
    }
  }

  function clearActiveDanmus() {
    applyDanmuPlaybackState(
      clearActiveDanmusState(getDanmuPlaybackState()),
      "clear",
    );
  }

  function removeActiveDanmu(id: number) {
    activeDanmus = removeActiveDanmuById(activeDanmus, id);
  }

  function setCurrentPlaybackRate(rate: number, refreshDanmu = true) {
    const nextRate = Math.max(0.1, rate);
    currentPlaybackRate = nextRate;

    if (usingMacOSNativePlayer()) {
      void setMacOSNativePlayerRate(nextRate, MACOS_NATIVE_PLAYER_ID).catch(
        (error) => {
          console.warn("Failed to update macOS native playback rate:", error);
        },
      );
    } else if (videoElement) {
      videoElement.playbackRate = nextRate;
    }

    if (refreshDanmu && danmuEnabled) {
      rebuildDanmuPlayback(Math.floor(getPlaybackCurrentTime() * 1000));
    }
  }

  function resetDanmuPlayback(positionMs = 0) {
    applyDanmuPlaybackState(
      resetDanmuPlaybackState({
        danmuRecords,
        positionMs,
        state: getDanmuPlaybackState(),
      }),
      "reset",
    );
  }

  function rebuildDanmuPlayback(positionMs = 0) {
    applyDanmuPlaybackState(
      rebuildDanmuPlaybackState({
        danmuRecords,
        positionMs,
        danmuActiveDurationMs,
        renderDanmuEmotes,
        danmakuEmoteMap,
        layout: danmuLayoutConfig,
        state: getDanmuPlaybackState(),
      }),
      "rebuild",
    );
  }

  function syncDanmuPlayback(currentTimeMs: number) {
    applyDanmuPlaybackState(
      syncDanmuPlaybackState({
        danmuRecords,
        currentTimeMs,
        danmuLookbackMs: DANMU_LOOKBACK_MS,
        danmuActiveDurationMs,
        renderDanmuEmotes,
        danmakuEmoteMap,
        layout: danmuLayoutConfig,
        state: getDanmuPlaybackState(),
        preserveActiveDanmus: isRightArrowFastForwardActive,
      }),
      "sync",
    );
  }

  // 加载字幕样式
  function loadSubtitleStyle() {
    const savedStyle = localStorage.getItem(`subtitle_style_${roomId}`);
    if (savedStyle) {
      subtitleStyle = { ...subtitleStyle, ...JSON.parse(savedStyle) };
    }
  }

  function loadDanmakuStyleSettings() {
    danmakuStyle = loadDanmakuStyle(roomId);
  }

  function getDanmuPlaybackState(): DanmuPlaybackState {
    return {
      activeDanmus,
      recentDanmuPositions,
      nextDanmuRenderId,
      lastDanmuTimeMs,
      nextDanmuIndex,
    };
  }

  function applyDanmuPlaybackState(
    state: DanmuPlaybackState,
    _reason = "unknown",
  ) {
    activeDanmus = state.activeDanmus;
    recentDanmuPositions = state.recentDanmuPositions;
    nextDanmuRenderId = state.nextDanmuRenderId;
    lastDanmuTimeMs = state.lastDanmuTimeMs;
    nextDanmuIndex = state.nextDanmuIndex;
  }

  function syncPreviewState(time: number) {
    currentTime = time;
    currentSubtitleIndex = getCurrentSubtitleIndex();
    currentSubtitle = subtitles[currentSubtitleIndex]?.text || "";

    if (danmuEnabled) {
      rebuildDanmuPlayback(Math.floor(time * 1000));
    }

    syncWaveformWithVideo();
  }

  function applyPlaybackProgress(
    time: number,
    syncWaveform = true,
    syncDanmu = true,
  ) {
    const previousTime = currentTime;
    currentTime = Math.max(0, time);
    currentSubtitleIndex = getCurrentSubtitleIndex();
    currentSubtitle = subtitles[currentSubtitleIndex]?.text || "";
    const shouldSkipPausedDanmuDrift =
      !isPlaying && Math.abs(currentTime - previousTime) <= 0.5;
    if (syncDanmu && !shouldSkipPausedDanmuDrift) {
      syncDanmuPlayback(Math.floor(currentTime * 1000));
    }

    if (syncWaveform) {
      syncWaveformWithVideo();
    }
  }

  async function setPreviewTime(time: number) {
    if (!videoElement) {
      return;
    }

    const duration = videoElement.duration || time;
    const clampedTime = Math.max(0, Math.min(duration, time));

    if (usingMacOSNativePlayer()) {
      try {
        await seekMacOSNativePlayer(clampedTime, MACOS_NATIVE_PLAYER_ID);
      } catch (error) {
        console.warn("Failed to seek macOS native player:", error);
      }
    } else {
      videoElement.currentTime = clampedTime;
    }

    syncPreviewState(clampedTime);
  }

  function focusPreviewStage() {
    if (typeof document === "undefined") {
      return;
    }

    const activeElement = document.activeElement as HTMLElement | null;
    if (
      activeElement &&
      activeElement !== document.body &&
      activeElement !== previewStageElement &&
      isBlockedHotkeyTarget(activeElement)
    ) {
      activeElement.blur();
    }

    previewStageElement?.focus({ preventScroll: true });
  }

  function resetPreviewForOpenState() {
    isVideoLoaded = false;
    subtitles = [];
    showSubtitleTimeline = false;
    danmuRecords = [];
    bilibiliPbpData = null;
    clearActiveDanmus();
    resetDanmuPlayback();
    currentSubtitleIndex = -1;
    loadSubtitleStyle();
    loadDanmakuStyleSettings();
    waveformDataPromise = null;
    waveformDataVideoId = null;
    if (rightArrowHoldTimeout !== null) {
      clearTimeout(rightArrowHoldTimeout);
      rightArrowHoldTimeout = null;
    }
    isRightArrowFastForwardActive = false;
    currentPlaybackRate = getSelectedPlaybackRate();
    resetSeekbarPreviewState(true);
    if (usingMacOSNativePlayer()) {
      pausePrimaryPlayback();
    }
    void macOSNativeClipPlayerRuntime.teardown();
    destroyWaveSurfer();
  }

  $: if (show) {
    resetPreviewForOpenState();
  }

  $: if (video?.id && video.id !== lastDanmuVideoId) {
    lastDanmuVideoId = video.id;
    danmuRecords = [];
    bilibiliPbpData = null;
    resetDanmuPlayback();
    waveformDataPromise = null;
    waveformDataVideoId = null;
  }

  $: {
    const nextVideoSource = video?.file ?? "";
    if (nextVideoSource !== lastSeekbarThumbnailVideoSource) {
      lastSeekbarThumbnailVideoSource = nextVideoSource;
      resetSeekbarPreviewState(true);
    }
  }

  // 当视频改变时重新初始化切片时间（只在视频ID改变时触发）
  $: if (video && videoElement?.duration && video.id !== lastVideoId) {
    lastVideoId = video.id;
    loadPersistedClipSelections(video.id);
    syncClipWaveformRegions();
    clipTitle = "";
  }

  // 监听样式编辑器关闭，重新加载样式
  $: if (!showStyleEditor) {
    loadSubtitleStyle();
  }

  async function handleVideoLoaded() {
    isVideoLoaded = true;
    waveformScale = timelineScale;
    currentPlaybackRate = getSelectedPlaybackRate();
    isRightArrowFastForwardActive = false;

    if (videoElement) {
      videoElement.currentTime = 0;
      videoElement.pause();
      videoElement.volume = getEffectiveVolume();
      videoElement.playbackRate = getSelectedPlaybackRate();
      isPlaying = false;
      currentTime = 0;
      currentSubtitle = "";
      currentSubtitleIndex = -1;
      resetDanmuPlayback();
      videoWidth = videoElement.videoWidth;
      videoHeight = videoElement.videoHeight;
      watchVideoDisplayMetrics();
    }

    resetSeekbarPreviewState(false);
    scheduleTimelineRefresh();
    resetWaveformZoomInputTracking(getTimelineSliderValue());
    await loadSubtitles();
    showSubtitleTimeline = subtitles.length > 0;
    syncClipWindowHeightAfterLayout(true);
    scheduleTimelineRefresh();
    void syncPreviewLayoutAfterUiChange();
    await loadDanmu();
    await loadBilibiliPbp();
    await ensureMacOSNativePlayerMounted("video-loaded");
    focusPreviewStage();

    ensureWaveformData().catch((error) => {
      console.warn("Failed to prepare waveform cache:", error);
    });

    if (showWaveform) {
      setTimeout(() => {
        void initWaveSurfer();
      }, 100);
    }

    await seekbarThumbnailRuntime.prepare();
  }

  function updateTimeMarkers() {
    if (!isVideoLoaded || !videoElement?.duration || !timelineWidth) {
      timeMarkers = [];
      return;
    }

    timeMarkers = resolveTimelineMarkers(
      videoElement.duration,
      clampTimelineScale(timelineScale),
    );
  }

  function syncTimelineScrollMetrics() {
    timelineZoomRuntime.syncTimelineScrollMetrics();
  }

  function handleTimelineScroll() {
    timelineZoomRuntime.handleTimelineScroll();
  }

  function scheduleTimelineRefresh() {
    timelineZoomRuntime.scheduleTimelineRefresh();
  }

  function resetWaveformZoomInputTracking(sliderValue: number | null = null) {
    timelineZoomRuntime.resetWaveformZoomInputTracking(sliderValue);
  }

  $: {
    const nextTarget = timelineContainer ?? null;
    timelineZoomRuntime.syncGestureTarget("timeline", nextTarget);
  }

  $: {
    const nextTarget = timelineZoomControlElement ?? null;
    timelineZoomRuntime.syncGestureTarget("zoomControl", nextTarget);
  }

  $: {
    const nextTarget = waveformGestureElement ?? null;
    timelineZoomRuntime.syncGestureTarget("waveform", nextTarget);
  }

  function openTimeSeekInput() {
    if (isTimeSeekEditing) {
      return;
    }

    isTimeSeekEditing = true;
    timeSeekValue = formatTimeForSeekInput(Math.max(0, currentTime));

    void tick().then(() => {
      timeSeekInput?.focus();
      timeSeekInput?.select();
    });
  }

  async function closeTimeSeekInput(applyValue: boolean) {
    if (!isTimeSeekEditing) {
      return;
    }

    if (applyValue) {
      const parsed = parseTimeInput(timeSeekValue);
      if (parsed !== null) {
        const duration =
          playbackDurationValue > 0
            ? playbackDurationValue
            : videoElement?.duration || parsed;
        await setPreviewTime(Math.max(0, Math.min(duration, parsed)));
      }
    }

    isTimeSeekEditing = false;
  }

  async function handleTimeSeekInputKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      await closeTimeSeekInput(true);
    } else if (event.key === "Escape") {
      event.preventDefault();
      await closeTimeSeekInput(false);
    }
  }

  function handleTimeSeekKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      openTimeSeekInput();
    }
  }

  // 切片功能相关函数
  function setClipStartTime() {
    if (videoElement) {
      applyClipSelectionState(
        setClipStartTimeRuntime({
          state: getClipSelectionState(),
          currentTime: getPlaybackCurrentTime(),
          color: CLIP_REGION_COLOR,
        }),
      );
      syncClipWaveformRegions();
    }
  }

  function setClipEndTime() {
    if (videoElement) {
      const result = setClipEndTimeRuntime({
        state: getClipSelectionState(),
        currentTime: getPlaybackCurrentTime(),
        nextId: createClipSelectionId(),
        color: CLIP_REGION_COLOR,
      });
      applyClipSelectionState(result.state);

      syncClipWaveformRegions();
    }
  }

  function seekToClipStart() {
    if (videoElement && clipTimesSet) {
      setPreviewTime(clipStartTime);
    }
  }

  function seekToClipEnd() {
    if (videoElement && clipTimesSet) {
      setPreviewTime(clipEndTime);
    }
  }

  function clearClipSelection() {
    const hadPendingClipStartMarker = hasPendingClipStartMarker;
    const activeSelectionId = activeClipSelectionId;
    applyClipSelectionState(
      clearClipSelectionState({
        state: getClipSelectionState(),
      }),
    );
    if (hadPendingClipStartMarker) {
      clipStartMarkerRegion = removeWaveformRegion(clipStartMarkerRegion);
    }
    if (activeSelectionId) {
      if (clipSelectionRegions[activeSelectionId]) {
        removeWaveformRegion(clipSelectionRegions[activeSelectionId]);
        delete clipSelectionRegions[activeSelectionId];
      }
      syncClipWaveformRegionAppearance();
    }
  }

  async function generateClip(options?: {
    includeSubtitle?: boolean;
    includeDanmu?: boolean;
  }) {
    const includeSubtitle = options?.includeSubtitle ?? false;
    const includeDanmu = options?.includeDanmu ?? false;
    if (includeSubtitle) {
      await saveSubtitles();
    }
    await generateClipRuntime({
      video,
      clipSelections,
      clipExportSelectionIds,
      mergeClipSelectionsOnExport,
      clipTitle,
      includeSubtitle,
      includeDanmu,
      renderDanmuEmotes,
      danmuRenderOptions: buildDanmuRenderOptions(),
      srtStyle: parseSubtitleStyle(subtitleStyle),
      generateEventId,
      listen,
      invoke,
      onPrompt: update_clip_prompt,
      onClipTitleResolved: (value) => {
        clipTitle = value;
      },
      onClippingChange: (value) => {
        clipping = value;
      },
      onCurrentEventIdChange: (value) => {
        current_clip_event_id = value;
      },
      onSuccess: () => {
        if (onVideoListUpdate) {
          onVideoListUpdate();
        }
        clipTitle = "";
      },
      reportError: (message) => {
        alert(message);
      },
    });
  }

  function update_clip_prompt(text: string) {
    let span = document.getElementById("generate-clip-prompt");
    if (span) {
      span.textContent = text;
    }
  }

  function buildDanmuRenderOptions(): DanmuRenderOptions {
    return {
      fontScale: danmakuStyle.fontScale,
      opacity: danmakuStyle.opacity,
      displayArea: danmakuStyle.displayArea,
      speedPreset: danmakuStyle.speedPreset,
      maxOnScreen: danmakuStyle.maxOnScreen,
      bold: danmakuStyle.bold,
      fontFamily: danmakuStyle.fontFamily,
      preventSubtitleOcclusion: danmuPreventSubtitleOcclusionEnabled,
    };
  }

  function canBeClipped(video: VideoItem): boolean {
    return canBeClippedRuntime(video);
  }

  function isBlockedHotkeyTarget(target: EventTarget | null): boolean {
    return isBlockedHotkeyTargetRuntime(target);
  }

  async function resetRightArrowFastForward(
    shouldSeekOnTap: boolean,
    reason = "reset",
  ) {
    await playbackHotkeyRuntime.resetRightArrowFastForward(
      shouldSeekOnTap,
      reason,
    );
  }

  // 键盘快捷键处理
  function handleKeydown(event: KeyboardEvent) {
    playbackHotkeyRuntime.handleKeydown(event);
  }

  function handleKeyup(event: KeyboardEvent) {
    playbackHotkeyRuntime.handleKeyup(event);
  }

  function handleWindowBlur() {
    videoSelectMenuController.close();
    playbackRateMenuController.close();
    previewDisplayMenuController.close();
    danmuFontMenuController.close();
    pbpMethodMenuController.close();
    if (isRightArrowFastForwardActive || rightArrowHoldTimeout !== null) {
      void resetRightArrowFastForward(false, "window-blur");
    }
  }

  function handleWindowClick(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (
      isVideoSelectMenuVisible &&
      (!target || !target.closest(".clip-video-select"))
    ) {
      videoSelectMenuController.close();
    }
    if (
      isDanmuFontMenuVisible &&
      (!target || !target.closest(".danmu-font-select"))
    ) {
      danmuFontMenuController.close();
    }
    if (
      isPbpMethodMenuVisible &&
      (!target || !target.closest(".pbp-method-select"))
    ) {
      pbpMethodMenuController.close();
    }
  }

  function handleWindowFocus() {
    if (!show || isDocumentFullscreen) {
      return;
    }
    restorePreviewHotkeyFocusAfterFullscreenExit();
  }

  function togglePlay() {
    playbackHotkeyRuntime.togglePlay();
  }

  function handleTimeUpdate() {
    if (usingMacOSNativePlayer() || !videoElement) {
      return;
    }

    applyPlaybackProgress(videoElement.currentTime, !isRightArrowFastForwardActive);
  }

  function handleVideoEnded() {
    playbackHotkeyRuntime.handleVideoEnded();
  }

  function handleNativePlaybackEnded() {
    playbackHotkeyRuntime.handleNativePlaybackEnded();
  }

  async function handleTimelineClick(e: MouseEvent) {
    await resetRightArrowFastForward(false, "timeline-click");
    e.preventDefault();
    e.stopPropagation();

    // 如果正在拖动进度条，不处理点击事件
    if (isDraggingSeekbar) return;

    if (!videoElement) return;
    const time = getTimelineTimeFromClientX(e.clientX, videoElement.duration);
    setPreviewTime(time);
  }

  function getTimelineTimeFromClientX(clientX: number, duration: number) {
    return subtitleTimelineRuntime.getTimelineTimeFromClientX(
      clientX,
      duration,
    );
  }

  // 进度条拖动事件处理
  async function handleSeekbarMouseDown(e: MouseEvent) {
    await seekbarInteractionRuntime.handleSeekbarMouseDown(e);
  }

  function handleSeekbarMouseEnter(e: MouseEvent) {
    seekbarInteractionRuntime.handleSeekbarMouseEnter(e);
  }

  function handleSeekbarMouseHoverMove(e: MouseEvent) {
    seekbarInteractionRuntime.handleSeekbarMouseHoverMove(e);
  }

  function handleSeekbarMouseLeave() {
    seekbarInteractionRuntime.handleSeekbarMouseLeave();
  }

  function syncSeekbarHoverFromClientPoint(clientX: number, clientY: number) {
    seekbarInteractionRuntime.syncSeekbarHoverFromClientPoint(clientX, clientY);
  }

  function addSubtitle() {
    const hadNoSubtitles = subtitles.length === 0;
    const duration = videoElement?.duration ?? 0;
    subtitles = createSubtitleAtTime({
      subtitles,
      currentTime,
      duration,
    });
    if (hadNoSubtitles) {
      showSubtitleTimeline = true;
      scheduleTimelineRefresh();
      void syncPreviewLayoutAfterUiChange();
    }
  }

  function updateSubtitleTime(index: number, isStart: boolean, time: number) {
    if (!videoElement?.duration) return;
    subtitles = updateSubtitleTimeRuntime({
      subtitles,
      index,
      isStart,
      time,
      duration: videoElement.duration,
    });
  }

  function moveSubtitle(index: number, newStartTime: number) {
    if (!videoElement?.duration) return;
    subtitles = moveSubtitleRuntime({
      subtitles,
      index,
      newStartTime,
      duration: videoElement.duration,
    });
  }

  async function removeSubtitle(index: number) {
    subtitles = removeSubtitleRuntime(subtitles, index);
    await saveSubtitles(); // 删除字幕时保存
  }

  async function clearSubtitles() {
    subtitles = clearSubtitlesRuntime();
    await saveSubtitles(); // 清空字幕时保存
  }

  function seekToTime(time: number) {
    setPreviewTime(time);
  }

  function adjustTime(index: number, isStart: boolean, delta: number) {
    if (!videoElement?.duration) {
      return;
    }
    subtitles = adjustSubtitleTimeRuntime({
      subtitles,
      index,
      isStart,
      delta,
      duration: videoElement.duration,
    });
  }

  function handleTimelineMouseDown(
    e: MouseEvent,
    index: number,
    isStart: boolean,
  ) {
    subtitleTimelineRuntime.handleTimelineMouseDown(e, index, isStart);
  }

  function handleBlockMouseDown(e: MouseEvent, index: number) {
    subtitleTimelineRuntime.handleBlockMouseDown(e, index);
  }

  function getSubtitleStyle(subtitle: Subtitle) {
    return getSubtitleStyleRuntime({
      subtitle,
      isVideoLoaded,
      duration: videoElement?.duration ?? 0,
    });
  }

  function handleVolumeChange(e: Event) {
    const input = e.target as HTMLInputElement;
    const nextVolume = Math.max(0, Math.min(1, parseFloat(input.value)));
    if (!Number.isFinite(nextVolume)) {
      return;
    }

    volume = nextVolume;
    if (nextVolume > 0) {
      previousVolume = nextVolume;
      isMuted = false;
    } else {
      isMuted = true;
    }

    applyVolumeToPlayback();
  }

  function handleCoverError(event: Event) {
    console.error("Cover image load failed:", event);
    showDefaultCoverIcon = true;
  }

  function toggleMute() {
    if (isMuted || getEffectiveVolume() === 0) {
      volume = Math.max(0.05, previousVolume || 1);
      isMuted = false;
    } else {
      previousVolume = Math.max(volume, 0.05);
      isMuted = true;
    }

    const nextMuted = isMuted || volume === 0;
    triggerVolumeClickAnimation(nextMuted);
    if (isVolumeHover) {
      isVolumeHoverSuppressed = true;
    }

    applyVolumeToPlayback();
  }

  function getCurrentSubtitleIndex(): number {
    return subtitles.findIndex(
      (sub) => currentTime >= sub.startTime && currentTime < sub.endTime,
    );
  }

  function handleScaleChange() {
    timelineZoomRuntime.handleScaleChange();
  }

  function handleScaleCommit() {
    timelineZoomRuntime.handleScaleCommit();
  }

  function applyTimelinePanelLayoutChange(
    reason: string,
    targetHeight: number,
    options: {
      applyStateChange?: () => void | Promise<void>;
      applyStateBeforeResize?: boolean;
      prepareLayoutReserve?: () => void | Promise<void>;
      cleanupLayoutReserve?: () => void | Promise<void>;
    } = {},
  ) {
    void applyTimelinePanelLayoutChangeRuntime({
      reason,
      targetHeight,
      applyStateBeforeResize: options.applyStateBeforeResize,
      applyStateChange: options.applyStateChange,
      prepareLayoutReserve: options.prepareLayoutReserve,
      cleanupLayoutReserve: options.cleanupLayoutReserve,
      isPreviewDisplayMenuVisible,
      setHidePreviewDisplayMenuDuringPanelTransition: (value) => {
        hidePreviewDisplayMenuDuringPanelTransition = value;
      },
      getHidePreviewDisplayMenuDuringPanelTransition: () =>
        hidePreviewDisplayMenuDuringPanelTransition,
      setPreviewDisplayMenuInteractionLocked,
      setPreviewOverlayLayersSuppressed,
      setPreviewDisplayMetricsFrozen,
      suppressClipWindowResizeRefresh,
      resizeRefreshSuppressMs: CLIP_WINDOW_RESIZE_REFRESH_SUPPRESS_MS,
      lockPreviewStageHeightForPanelTransition,
      unlockPreviewStageHeightForPanelTransition,
      tick,
      syncClipWindowHeight,
      updateVideoDisplayMetrics,
      usingMacOSNativePlayer: usingMacOSNativePlayer(),
      syncMacOSNativePlayerBounds: (force, syncReason) =>
        macOSNativeClipPlayerRuntime.syncBounds(force, syncReason),
      tauriEnv: TAURI_ENV,
      waitForWindowBoundsStable,
      syncPreviewLayoutAfterUiChange,
      resumeMacOSNativePlayerBoundsSync,
    });
  }

  function toggleSubtitleTimeline() {
    const nextShowSubtitleTimeline = !showSubtitleTimeline;
    const nextShowSubtitleTimelineEffective =
      nextShowSubtitleTimeline && subtitles.length > 0;
    const nextTargetHeight = getAdaptiveClipWindowHeightForState(
      nextShowSubtitleTimelineEffective,
      showWaveform,
    );
    const isExpanding =
      !showSubtitleTimelineEffective && nextShowSubtitleTimelineEffective;
    suspendMacOSNativePlayerBoundsSync("toggle-subtitle");
    applyTimelinePanelLayoutChange("toggle-subtitle", nextTargetHeight, {
      applyStateBeforeResize: !isExpanding,
      prepareLayoutReserve: isExpanding
        ? () => {
            reserveSubtitleTimelineLayoutDuringResize = true;
            scheduleTimelineRefresh();
          }
        : undefined,
      cleanupLayoutReserve: isExpanding
        ? () => {
            reserveSubtitleTimelineLayoutDuringResize = false;
            scheduleTimelineRefresh();
          }
        : undefined,
      applyStateChange: () => {
        showSubtitleTimeline = nextShowSubtitleTimeline;
        scheduleTimelineRefresh();
      },
    });
  }

  function toggleWaveform() {
    const nextShowWaveform = !showWaveform;
    const nextTargetHeight = getAdaptiveClipWindowHeightForState(
      showSubtitleTimelineEffective,
      nextShowWaveform,
    );
    const isExpanding = !showWaveform && nextShowWaveform;
    suspendMacOSNativePlayerBoundsSync("toggle-waveform");
    applyTimelinePanelLayoutChange("toggle-waveform", nextTargetHeight, {
      applyStateBeforeResize: !isExpanding,
      prepareLayoutReserve: isExpanding
        ? () => {
            reserveWaveformLayoutDuringResize = true;
            scheduleTimelineRefresh();
          }
        : undefined,
      cleanupLayoutReserve: isExpanding
        ? () => {
            reserveWaveformLayoutDuringResize = false;
            scheduleTimelineRefresh();
          }
        : undefined,
      applyStateChange: async () => {
        showWaveform = nextShowWaveform;
        scheduleTimelineRefresh();

        if (!showWaveform) {
          return;
        }

        waveformScale = timelineScale;
        await tick();
        if (!wavesurfer) {
          requestAnimationFrame(() => {
            initWaveSurfer();
            void syncPreviewLayoutAfterUiChange();
          });
          return;
        }

        void redrawWaveformAtCurrentWidth().finally(() => {
          void syncPreviewLayoutAfterUiChange();
        });
        syncClipWaveformRegions();
      },
    });
  }

  function handleTimelineZoomControlWheel(e: WheelEvent) {
    timelineZoomRuntime.handleTimelineZoomControlWheel(e);
  }

  function handleWheel(e: WheelEvent) {
    timelineZoomRuntime.handleWheel(e);
  }

  async function encodeVideoMedia(options: {
    includeSubtitle: boolean;
    includeDanmu: boolean;
  }) {
    if (!video?.id) {
      return;
    }
    if (!options.includeSubtitle && !options.includeDanmu) {
      alert("请至少选择字幕或弹幕");
      return;
    }

    if (options.includeSubtitle) {
      await saveSubtitles();
    }
    const eventId = generateEventId();
    current_encode_event_id = eventId;
    let taskSettled = false;
    let clearUpdateListener: (() => void) | undefined;
    let clearFinishedListener: (() => void) | undefined;

    function finishEncodeTask() {
      update_encode_prompt("压制");
      current_encode_event_id = null;
      clearUpdateListener?.();
      clearFinishedListener?.();
    }

    try {
      clearUpdateListener = await listen(
        `progress-update:${eventId}`,
        (event) => {
          update_encode_prompt(event.payload.content);
        },
      );

      clearFinishedListener = await listen(
        `progress-finished:${eventId}`,
        (event) => {
          if (taskSettled) {
            return;
          }
          taskSettled = true;
          if (!event.payload.success) {
            alert(`压制失败: ${event.payload.message}`);
          }

          finishEncodeTask();
        },
      );

      await invoke("encode_video_media", {
        eventId,
        id: video.id,
        includeSubtitle: options.includeSubtitle,
        includeDanmu: options.includeDanmu,
        renderDanmuEmotes,
        danmuRenderOptions: buildDanmuRenderOptions(),
        srtStyle: parseSubtitleStyle(subtitleStyle),
      });

      await onVideoListUpdate?.();
    } catch (error) {
      if (taskSettled) {
        return;
      }
      taskSettled = true;
      alert(`压制失败: ${error}`);
      finishEncodeTask();
    }
  }

  function selectVideo(selectedVideo: VideoItem | undefined) {
    if (selectedVideo) {
      // 清空字幕列表
      subtitles = [];
      currentSubtitleIndex = -1;
      currentSubtitle = "";
      // 重置视频状态
      if (usingMacOSNativePlayer()) {
        pausePrimaryPlayback();
        void macOSNativeClipPlayerRuntime.teardown();
      }
      if (videoElement) {
        videoElement.currentTime = 0;
        pausePrimaryPlayback();
        isPlaying = false;
        currentTime = 0;
      }
      // 调用父组件的回调
      onVideoChange?.(selectedVideo);
    }
  }

  function toggleVideoMenu() {
    if (isVideoSelectMenuVisible) {
      videoSelectMenuController.close();
    } else {
      videoSelectMenuController.open();
    }
  }

  async function deleteCurrentVideo() {
    if (!video) {
      return;
    }

    try {
      await invoke("delete_video", { id: video.id });
      await onVideoListUpdate?.();
      if (videos.length > 0) {
        onVideoChange?.(videos[0]);
      } else {
        close_window();
      }
    } catch (error) {
      console.error(error);
      alert("删除失败：" + error);
    }
  }

  function openEncodeModal() {
    showEncodeModal = true;
  }

  function confirmEncodeModal() {
    showEncodeModal = false;
    encodeVideoMedia({
      includeSubtitle: encodeIncludeSubtitle,
      includeDanmu: encodeIncludeDanmu,
    });
  }

  function openClipGenerateModal() {
    showClipGenerateModal = true;
  }

  function confirmClipGenerateModal() {
    showClipGenerateModal = false;
    generateClip({
      includeSubtitle: clipIncludeSubtitle,
      includeDanmu: clipIncludeDanmu,
    });
  }

  function getVideoSelectLabel() {
    const matchedVideo = videos.find((item) => item.id === video?.id);
    return matchedVideo?.name || video?.title || video?.file || "选择视频";
  }

  async function saveVideo() {
    if (!video) return;
    const video_url = video.file;
    const video_name = video.file;
    const a = document.createElement("a");
    a.href = video_url;
    a.download = video_name;
    a.click();
  }
</script>

{#if show}
  <div
    class="fixed inset-0 z-[1000] flex flex-col transition-opacity duration-200"
    class:clip-web-fullscreen={isWebFullscreen}
    style:background={showMacOSNativePlayerUnderlayValue ? "transparent" : "#1c1c1e"}
    class:opacity-0={!show}
    class:opacity-100={show}
  >
    {#if shouldRenderMacOSNativePlayerPresentationValue}
      <div
        class="pointer-events-none fixed -left-[200vw] -top-[200vh] h-px w-px overflow-hidden opacity-0"
        aria-hidden="true"
      >
        <!-- svelte-ignore a11y-media-has-caption -->
        <video
          bind:this={videoElement}
          src={video?.file}
          class="block h-px w-px"
          playsinline
          on:timeupdate={handleTimeUpdate}
          on:ended={handleVideoEnded}
          on:loadedmetadata={handleVideoLoaded}
          on:click={togglePlay}
        />
      </div>
    {/if}

    <VideoPreviewLetterboxMask
      visible={showMacOSNativePlayerUnderlayValue}
      top={previewStageViewportTop}
      left={previewStageViewportLeft}
      width={previewStageViewportWidth}
      height={previewStageViewportHeight}
    />

    {#if showMacOSNativePlayerUnderlayValue && previewVideoFrameWidth > 0 && previewVideoFrameHeight > 0}
      {#if danmuEnabled && activeDanmus.length > 0}
        <div
          class="pointer-events-none absolute z-[12] overflow-hidden clip-preview-stable-overlay"
          style={stablePreviewVideoFrameStyle}
        >
          <VideoPreviewDanmuOverlay
            {activeDanmus}
            {isPlaying}
            animationRate={danmuAnimationRate}
            playbackTimeMs={currentTime * 1000}
            {videoDanmuFontSize}
            {videoDanmuFontFamily}
            {videoDanmuFontWeight}
            {videoDanmuLineHeight}
            {videoDanmuEmoteScale}
            {videoDanmuEmoteOffset}
            {videoDanmuOpacity}
            {videoDanmuTextShadow}
            {getVideoDanmuEmoteStyle}
            onRemoveActiveDanmu={removeActiveDanmu}
          />
        </div>
      {/if}

      {#if canBeClipped(video) && !suppressPreviewOverlayLayersValue}
        <VideoPreviewHotkeyOverlay
          showDetail={show_detail}
          style={`${getStablePreviewStageOverlayStyle()}background-color: rgba(0, 0, 0, 0.5); color: white; font-size: 0.8em;`}
        />
      {/if}
    {/if}

    <VideoPreviewTopBar
      {video}
      {videos}
      videoSelectLabel={getVideoSelectLabel()}
      {isVideoSelectMenuVisible}
      {videoSelectMenuController}
      onToggleVideoMenu={toggleVideoMenu}
      onSelectVideo={selectVideo}
      onDeleteVideo={deleteCurrentVideo}
      showDownloadButton={!TAURI_ENV}
      onDownloadVideo={saveVideo}
      canClip={canBeClipped(video)}
      {clipping}
      currentClipEventId={current_clip_event_id}
      onGenerateClip={openClipGenerateModal}
      currentEncodeEventId={current_encode_event_id}
      onOpenEncodeModal={openEncodeModal}
    />

    <VideoPreviewEncodeModal
      bind:show={showClipGenerateModal}
      title="生成切片"
      description="选择需要烧录进切片画面的内容；不选择时只生成普通切片。"
      confirmLabel="生成"
      requireSelection={false}
      bind:includeSubtitle={clipIncludeSubtitle}
      bind:includeDanmu={clipIncludeDanmu}
      onCancel={() => (showClipGenerateModal = false)}
      onConfirm={confirmClipGenerateModal}
    />

    <VideoPreviewEncodeModal
      bind:show={showEncodeModal}
      title="压制"
      description="选择需要烧录进当前视频画面的内容。"
      confirmLabel="压制"
      requireSelection={true}
      bind:includeSubtitle={encodeIncludeSubtitle}
      bind:includeDanmu={encodeIncludeDanmu}
      onCancel={() => (showEncodeModal = false)}
      onConfirm={confirmEncodeModal}
    />

    <div class="clip-main relative z-10 flex flex-1 min-h-0 overflow-hidden">
      <!-- 视频区域 -->
      <div class="flex-1 min-w-0 min-h-0 flex flex-col overflow-hidden">
        <!-- 切片控制信息条 -->
        {#if canBeClipped(video)}
          <div
            class="bg-black px-4 py-2 flex items-center gap-6 text-sm flex-wrap"
          >
            <div class="flex items-center space-x-6">
              <div class="text-gray-300">
                切片起点: <span class="text-[#0A84FF] font-mono"
                  >{formatTime(clipStartTime)}</span
                >
              </div>
              <div class="text-gray-300">
                切片终点: <span class="text-[#0A84FF] font-mono"
                  >{formatTime(clipEndTime)}</span
                >
              </div>
              <div class="text-gray-300">
                时长: <span class="text-white font-mono"
                  >{formatTime(clipEndTime - clipStartTime)}</span
                >
              </div>
            </div>
          </div>
        {/if}
        <!-- 视频容器 -->
        <div
          bind:this={previewStageElement}
          class="flex-1 min-h-0 relative overflow-hidden outline-none"
          style:flex={previewStageHeightLockPx !== null
            ? "0 0 auto"
            : undefined}
          style:height={previewStageHeightLockPx !== null
            ? `${previewStageHeightLockPx}px`
            : undefined}
          style:min-height={previewStageHeightLockPx !== null
            ? `${previewStageHeightLockPx}px`
            : undefined}
          style:background={showMacOSNativePlayerUnderlayValue
            ? "transparent"
            : "black"}
          tabindex="-1"
        >
          <div
            class="absolute inset-0 z-0 bg-black transition-opacity duration-75"
            style:opacity={showMacOSNativePlayerUnderlayValue ? 0 : 1}
          >
            <div
              bind:this={macOSNativePlayerSlotElement}
              class="absolute outline-none"
              class:pointer-events-none={!usingMacOSNativePlayerValue}
              style={shouldRenderMacOSNativePlayerPresentationValue &&
              previewVideoFrameWidth > 0 &&
              previewVideoFrameHeight > 0
                ? `top:${previewVideoFrameTop}px;left:${previewVideoFrameLeft}px;width:${previewVideoFrameWidth}px;height:${previewVideoFrameHeight}px;`
                : "inset: 0;"}
              role="button"
              aria-label="Toggle preview playback"
              tabindex={usingMacOSNativePlayerValue ? 0 : -1}
              on:click={togglePlay}
              on:keydown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  event.stopPropagation();
                  togglePlay();
                }
              }}
            />

            {#if !shouldRenderMacOSNativePlayerPresentationValue}
              <!-- svelte-ignore a11y-media-has-caption -->
              <video
                bind:this={videoElement}
                src={video?.file}
                class={shouldHideDomPreview()
                  ? "hidden"
                  : "absolute inset-x-0 bottom-0 w-full h-auto max-h-full cursor-pointer transition-opacity duration-75"}
                playsinline
                on:timeupdate={handleTimeUpdate}
                on:ended={handleVideoEnded}
                on:loadedmetadata={handleVideoLoaded}
                on:click={togglePlay}
              />
            {/if}
          </div>

          <VideoPreviewLetterboxMask
            visible={showMacOSNativePlayerUnderlayValue}
            top={previewVideoFrameTop}
            left={previewVideoFrameLeft}
            width={previewVideoFrameWidth}
            height={previewVideoFrameHeight}
            zIndex={1}
          />

          {#if !suppressPreviewOverlayLayersValue}
            <div class="pointer-events-none absolute inset-0 z-10">
              {#if showFastForwardOverlayValue}
                <div
                  class="three-playrate-hint"
                  style:top={`${threePlayrateHintTopValue}px`}
                >
                  <span class="three-playrate-hint-icon">
                    <LottieIcon
                      animationData={skipLottieData}
                      autoplay={true}
                      loop={true}
                      className="lottie-icon lottie-icon--hint"
                    />
                  </span>
                  <span class="three-playrate-hint-text">倍速播放中</span>
                </div>
              {/if}

              {#if !showMacOSNativePlayerUnderlayValue && danmuEnabled && activeDanmus.length > 0}
                <div
                  class="absolute inset-0 pointer-events-none overflow-hidden z-15"
                >
                  <VideoPreviewDanmuOverlay
                    {activeDanmus}
                    {isPlaying}
                    animationRate={danmuAnimationRate}
                    playbackTimeMs={currentTime * 1000}
                    {videoDanmuFontSize}
                    {videoDanmuFontFamily}
                    {videoDanmuFontWeight}
                    {videoDanmuLineHeight}
                    {videoDanmuEmoteScale}
                    {videoDanmuEmoteOffset}
                    {videoDanmuOpacity}
                    {videoDanmuTextShadow}
                    {getVideoDanmuEmoteStyle}
                    onRemoveActiveDanmu={removeActiveDanmu}
                  />
                </div>
              {/if}

              {#if !showMacOSNativePlayerUnderlayValue && canBeClipped(video)}
                <VideoPreviewHotkeyOverlay
                  showDetail={show_detail}
                  style="top: 0.5rem; left: 0.5rem; background-color: rgba(0, 0, 0, 0.5); color: white; font-size: 0.8em;"
                />
              {/if}
              <!-- 字幕显示 -->
              <VideoPreviewSubtitleOverlay
                {currentSubtitle}
                {subtitleStyle}
                {previewDisplayHeight}
                {previewScaleBaseHeight}
              />
            </div>
          {/if}
        </div>

        <VideoPreviewTransport
          {isPreviewDisplayMenuVisible}
          {hidePreviewDisplayMenuDuringPanelTransition}
          bind:previewDisplayMenuElement
          {isPreviewDisplayMenuInteractionLocked}
          {getPreviewDisplayMenuMetrics}
          previewDisplayMenuWidth={PREVIEW_DISPLAY_MENU_WIDTH}
          previewDisplayMenuHeight={PREVIEW_DISPLAY_MENU_HEIGHT}
          {openPreviewDisplayMenu}
          {previewDisplayMenuController}
          {showSubtitleTimeline}
          {togglePreviewDisplaySubtitleTimeline}
          {showWaveform}
          {togglePreviewDisplayWaveform}
          {showPbpOverlay}
          {togglePreviewDisplayPbp}
          {seekbarPbpVisible}
          {handlePbpOverlayMouseDown}
          {seekbarPbpViewBoxValue}
          {seekbarPbpCurveClipPathId}
          {seekbarPbpPlayedClipPathId}
          {seekbarPbpCurvePath}
          {seekbarPbpPlayedWidth}
          seekbarPbpViewboxWidth={SEEKBAR_PBP_VIEWBOX_WIDTH}
          seekbarPbpViewboxHeight={SEEKBAR_PBP_VIEWBOX_HEIGHT}
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
          timelineZoomNotchCount={TIMELINE_ZOOM_NOTCH_PERCENTS.length}
          {handleTimelineZoomControlWheel}
          {handleScaleChange}
          {handleScaleCommit}
          bind:playbackRateTriggerElement
          {isPlaybackRateMenuVisible}
          {canAdjustPlaybackRateValue}
          {playbackRateMenuController}
          {playbackRateDisplayLabelValue}
          playbackRateOptions={PLAYBACK_RATE_OPTIONS}
          {getSelectedPlaybackRate}
          {handlePlaybackRateSelect}
          bind:previewSettingsControlElement
          bind:previewSettingsButtonElement
          {handleSettingsMouseEnter}
          {handleSettingsMouseLeave}
          {isSettingsHover}
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
          onWideHoverChange={(value) => (isWideHover = value)}
          {isWebFullscreen}
          {toggleWebFullscreen}
          {isWebFullscreenHover}
          onWebFullscreenHoverChange={(value) =>
            (isWebFullscreenHover = value)}
          {isDocumentFullscreen}
          {toggleFullscreen}
          {isFullscreenHover}
          onFullscreenHoverChange={(value) => (isFullscreenHover = value)}
          bind:timelineContainer
          {handleWheel}
          {handleTimelineScroll}
          bind:waveformGestureElement
          {waveformScale}
          {showWaveformLayoutVisible}
          waveformPanelHeightPx={WAVEFORM_PANEL_HEIGHT_PX}
          {isWaveformLoading}
          bind:waveformContainer
          bind:timelineElement
          {timelineScale}
          {showSubtitleTimelineLayoutVisible}
          {scheduleTimelineRefresh}
          {isDraggingSeekbar}
          {handleTimelineClick}
          canClip={canBeClipped(video)}
          {clipSelections}
          {activeClipSelectionId}
          {hasPendingClipStartMarker}
          {pendingClipStartTime}
          videoDuration={videoElement?.duration ?? 0}
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
          seekbarThumbSize={SEEKBAR_THUMB_SIZE}
          {showSeekbarMoveIndicatorValue}
          {seekbarPointerXValue}
          {timeMarkers}
          {formatTimelineMarkerTime}
          {showSubtitleTimelineEffective}
          {subtitles}
          {getSubtitleStyle}
          {handleBlockMouseDown}
          {handleTimelineMouseDown}
          {showSeekbarPopupValue}
          {seekbarPopupViewportLeftValue}
          {seekbarPopupViewportTopValue}
          {seekbarPreviewImageSrc}
          transparentSeekbarPreviewImage={TRANSPARENT_SEEKBAR_PREVIEW_IMAGE}
          {seekbarPopupTimeValue}
        />
      </div>

      <VideoPreviewSidePanel
        bind:activeTab
        {subtitles}
        {currentSubtitleIndex}
        currentGenerateEventId={current_generate_event_id}
        {formatTime}
        onOpenStyleEditor={() => (showStyleEditor = true)}
        onClearSubtitles={clearSubtitles}
        onGenerateSubtitles={generateSubtitles}
        onAddSubtitle={addSubtitle}
        onSeekToTime={seekToTime}
        onAdjustTime={adjustTime}
        onRemoveSubtitle={removeSubtitle}
        {clipSelections}
        {activeClipSelectionId}
        bind:clipExportSelectionIds
        bind:mergeClipSelectionsOnExport
        onSetActiveClipSelection={setActiveClipSelection}
        {video}
        {profile}
        bind:uidSelected={uid_selected}
        {accounts}
        currentPostEventId={current_post_event_id}
        bind:showCoverEditor={show_cover_editor}
        {showDefaultCoverIcon}
        {handleCoverError}
        doPost={do_post}
        cancelPost={cancel_post}
        {danmuEnabled}
        {renderDanmuEmotes}
        {danmuPreventSubtitleOcclusionEnabled}
        {danmuSyncWithPlaybackRateEnabled}
        {danmakuStyle}
        bind:danmakuDisplayAreaIndex
        bind:danmakuSpeedPresetIndex
        bind:danmakuMaxOnScreenIndex
        {isDanmuFontMenuVisible}
        {isPbpMethodMenuVisible}
        {seekbarPbpGenerationMethod}
        danmakuDisplayAreaOptions={DANMAKU_DISPLAY_AREA_OPTIONS}
        danmakuSpeedPresetOptions={DANMAKU_SPEED_PRESET_OPTIONS}
        danmakuMaxOnScreenOptions={DANMAKU_MAX_ON_SCREEN_OPTIONS}
        danmakuFontOptions={DANMAKU_FONT_OPTIONS}
        {seekbarPbpMethodOptions}
        {danmuFontMenuController}
        {pbpMethodMenuController}
        onDanmuToggle={handleDanmuToggle}
        onRenderDanmuEmotesToggle={handleRenderDanmuEmotesToggle}
        onDanmuPreventSubtitleOcclusionToggle={handleDanmuPreventSubtitleOcclusionToggle}
        onDanmuSyncWithPlaybackRateToggle={handleDanmuSyncWithPlaybackRateToggle}
        onDanmuBoldToggle={handleDanmuBoldToggle}
        onToggleDanmuFontMenu={toggleDanmuFontMenu}
        onHandleDanmuFontSelect={handleDanmuFontSelect}
        onTogglePbpMethodMenu={togglePbpMethodMenu}
        onHandleSeekbarPbpMethodSelect={handleSeekbarPbpMethodSelect}
      />
    </div>
  </div>
{/if}

<SubtitleStyleEditor
  bind:show={showStyleEditor}
  {roomId}
  onClose={() => (showStyleEditor = false)}
/>

<CoverEditor
  bind:show={show_cover_editor}
  {video}
  on:coverUpdate={(event) => {
    video = {
      ...video,
      cover: event.detail.cover,
    };
  }}
/>

<!-- 键盘快捷键监听 -->
<svelte:window
  on:keydown={handleKeydown}
  on:keyup={handleKeyup}
  on:click={handleWindowClick}
  on:focus={handleWindowFocus}
  on:blur={handleWindowBlur}
/>

<style>
  .z-15 {
    z-index: 15;
  }

  :global(.lottie-icon) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  :global(.lottie-icon--hint) {
    width: 30px;
    height: 18px;
  }

  .three-playrate-hint {
    pointer-events: none;
    position: absolute;
    top: 18px;
    left: 50%;
    z-index: 77;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 130px;
    height: 34px;
    margin-left: -65px;
    border-radius: 4px;
    background-color: rgba(33, 33, 33, 0.9);
    color: #fff;
    line-height: 34px;
    user-select: none;
  }

  .three-playrate-hint-icon {
    display: inline-block;
    flex: 0 0 auto;
    width: 30px;
    height: 18px;
    margin-right: 8px;
  }

  .three-playrate-hint-text {
    white-space: nowrap;
    font-size: 12px;
    font-weight: 500;
  }

  .clip-preview-stable-overlay {
    contain: layout paint;
    isolation: isolate;
    transform: translateZ(0);
    backface-visibility: hidden;
  }
</style>
