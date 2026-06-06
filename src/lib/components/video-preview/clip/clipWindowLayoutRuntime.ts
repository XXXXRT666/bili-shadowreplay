import { getCurrentWindow, PhysicalPosition, PhysicalSize } from "@tauri-apps/api/window";

export interface PreviewDisplayMetrics {
  previewStageViewportTop: number;
  previewStageViewportLeft: number;
  previewStageViewportWidth: number;
  previewStageViewportHeight: number;
  previewVideoFrameTop: number;
  previewVideoFrameLeft: number;
  previewVideoFrameWidth: number;
  previewVideoFrameHeight: number;
  videoDisplayHeight: number;
}

export interface ClipWindowLayoutState {
  suppressWindowResizeRefreshUntilMs: number;
  previewLayoutSyncFrame: number | null;
  previewLayoutDoubleSyncFrame: number | null;
  clipWindowHeightSyncTimer: number | null;
}

export function createClipWindowLayoutState(): ClipWindowLayoutState {
  return {
    suppressWindowResizeRefreshUntilMs: 0,
    previewLayoutSyncFrame: null,
    previewLayoutDoubleSyncFrame: null,
    clipWindowHeightSyncTimer: null,
  };
}

export function resolvePreviewDisplayMetrics(options: {
  previewStageElement: HTMLElement | null;
  videoElement: HTMLVideoElement | null | undefined;
  videoWidth: number;
  videoHeight: number;
}): PreviewDisplayMetrics {
  const emptyMetrics: PreviewDisplayMetrics = {
    previewStageViewportTop: 0,
    previewStageViewportLeft: 0,
    previewStageViewportWidth: 0,
    previewStageViewportHeight: 0,
    previewVideoFrameTop: 0,
    previewVideoFrameLeft: 0,
    previewVideoFrameWidth: 0,
    previewVideoFrameHeight: 0,
    videoDisplayHeight: 0,
  };

  if (!options.previewStageElement && !options.videoElement) {
    return emptyMetrics;
  }

  if (options.previewStageElement) {
    const rect = options.previewStageElement.getBoundingClientRect();
    const nextMetrics: PreviewDisplayMetrics = {
      ...emptyMetrics,
      previewStageViewportTop: Math.max(0, rect.top),
      previewStageViewportLeft: Math.max(0, rect.left),
      previewStageViewportWidth: Math.max(0, rect.width),
      previewStageViewportHeight: Math.max(0, rect.height),
    };

    if (rect.width > 0 && rect.height > 0 && options.videoWidth > 0 && options.videoHeight > 0) {
      const videoAspect = options.videoWidth / options.videoHeight;
      const containerAspect = rect.width / rect.height;
      let rawFrameTop = 0;
      let rawFrameLeft = 0;
      let rawFrameWidth = rect.width;
      let rawFrameHeight = rect.height;

      if (containerAspect > videoAspect) {
        rawFrameHeight = rect.height;
        rawFrameWidth = rect.height * videoAspect;
        rawFrameTop = 0;
        rawFrameLeft = Math.max(0, (rect.width - rawFrameWidth) / 2);
      } else {
        rawFrameWidth = rect.width;
        rawFrameHeight = rect.width / videoAspect;
        rawFrameLeft = 0;
        rawFrameTop = Math.max(0, rect.height - rawFrameHeight);
      }

      const snappedLeft = Math.max(0, Math.ceil(rawFrameLeft));
      const snappedTop = Math.max(0, Math.ceil(rawFrameTop));
      const snappedRight = Math.min(rect.width, Math.floor(rawFrameLeft + rawFrameWidth));
      const snappedBottom = Math.min(
        rect.height,
        Math.floor(rawFrameTop + rawFrameHeight)
      );

      nextMetrics.previewVideoFrameLeft = snappedLeft;
      nextMetrics.previewVideoFrameTop = snappedTop;
      nextMetrics.previewVideoFrameWidth = Math.max(0, snappedRight - snappedLeft);
      nextMetrics.previewVideoFrameHeight = Math.max(0, snappedBottom - snappedTop);
      nextMetrics.videoDisplayHeight = nextMetrics.previewVideoFrameHeight || rawFrameHeight;
      return nextMetrics;
    }

    nextMetrics.videoDisplayHeight = rect.height;
    nextMetrics.previewVideoFrameWidth = rect.width;
    nextMetrics.previewVideoFrameHeight = rect.height;
    return nextMetrics;
  }

  const rect = options.videoElement!.getBoundingClientRect();
  return {
    ...emptyMetrics,
    previewVideoFrameWidth: rect.width,
    previewVideoFrameHeight: rect.height,
    videoDisplayHeight: rect.height,
  };
}

export function cancelPreviewLayoutSync(state: ClipWindowLayoutState): void {
  if (state.previewLayoutSyncFrame !== null) {
    cancelAnimationFrame(state.previewLayoutSyncFrame);
    state.previewLayoutSyncFrame = null;
  }
  if (state.previewLayoutDoubleSyncFrame !== null) {
    cancelAnimationFrame(state.previewLayoutDoubleSyncFrame);
    state.previewLayoutDoubleSyncFrame = null;
  }
}

export async function syncPreviewLayoutAfterUiChange(options: {
  state: ClipWindowLayoutState;
  tick: () => Promise<void>;
  updateVideoDisplayMetrics: (reason: string) => void;
}): Promise<void> {
  await options.tick();
  options.updateVideoDisplayMetrics("ui-change:tick");

  if (typeof window === "undefined") {
    return;
  }

  cancelPreviewLayoutSync(options.state);
  options.state.previewLayoutSyncFrame = window.requestAnimationFrame(() => {
    options.state.previewLayoutSyncFrame = null;
    options.updateVideoDisplayMetrics("ui-change:raf1");
    options.state.previewLayoutDoubleSyncFrame = window.requestAnimationFrame(() => {
      options.state.previewLayoutDoubleSyncFrame = null;
      options.updateVideoDisplayMetrics("ui-change:raf2");
    });
  });
}

export function getMonotonicNowMs(): number {
  if (typeof window !== "undefined" && typeof window.performance !== "undefined") {
    return window.performance.now();
  }
  return Date.now();
}

export function suppressClipWindowResizeRefresh(options: {
  tauriEnv: boolean;
  durationMs: number;
  state: ClipWindowLayoutState;
}): void {
  if (!options.tauriEnv) {
    return;
  }

  const now = getMonotonicNowMs();
  options.state.suppressWindowResizeRefreshUntilMs = Math.max(
    options.state.suppressWindowResizeRefreshUntilMs,
    now + Math.max(0, options.durationMs)
  );
}

export function shouldSuppressClipWindowResizeRefresh(
  state: ClipWindowLayoutState
): boolean {
  return getMonotonicNowMs() < state.suppressWindowResizeRefreshUntilMs;
}

export function getAdaptiveClipWindowHeightForState(options: {
  baseHeight: number;
  subtitleExtraHeight: number;
  waveformExtraHeight: number;
  subtitleTimelineVisible: boolean;
  waveformVisible: boolean;
}): number {
  const subtitleExtra = options.subtitleTimelineVisible
    ? options.subtitleExtraHeight
    : 0;
  const waveformExtra = options.waveformVisible ? options.waveformExtraHeight : 0;
  return Math.round(options.baseHeight + subtitleExtra + waveformExtra);
}

export function cancelClipWindowHeightSync(state: ClipWindowLayoutState): void {
  if (state.clipWindowHeightSyncTimer !== null && typeof window !== "undefined") {
    window.clearTimeout(state.clipWindowHeightSyncTimer);
    state.clipWindowHeightSyncTimer = null;
  }
}

export function scheduleClipWindowHeightSync(options: {
  tauriEnv: boolean;
  state: ClipWindowLayoutState;
  force?: boolean;
  delayMs: number;
  syncClipWindowHeight: (force?: boolean) => Promise<void>;
}): void {
  if (!options.tauriEnv || typeof window === "undefined") {
    return;
  }

  cancelClipWindowHeightSync(options.state);
  options.state.clipWindowHeightSyncTimer = window.setTimeout(() => {
    options.state.clipWindowHeightSyncTimer = null;
    void options.syncClipWindowHeight(options.force);
  }, options.delayMs);
}

export async function syncClipWindowHeight(options: {
  tauriEnv: boolean;
  show: boolean;
  isWindowWide: boolean;
  isWebFullscreen: boolean;
  isDocumentFullscreen: boolean;
  targetHeight: number;
  thresholdPx: number;
  resizeKeepTopLeft: (width: number, height: number) => Promise<void>;
}): Promise<void> {
  if (
    !options.tauriEnv ||
    !options.show ||
    options.isWindowWide ||
    options.isWebFullscreen ||
    options.isDocumentFullscreen
  ) {
    return;
  }

  const currentWindow = getCurrentWindow();
  try {
    const [isFullscreen, isMaximized, currentSize, currentPosition, scaleFactor] =
      await Promise.all([
        currentWindow.isFullscreen(),
        currentWindow.isMaximized(),
        currentWindow.innerSize(),
        currentWindow.innerPosition(),
        currentWindow.scaleFactor(),
      ]);
    if (isFullscreen || isMaximized) {
      return;
    }

    const safeScaleFactor = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
    const targetHeightPhysical = Math.max(
      1,
      Math.round(options.targetHeight * safeScaleFactor)
    );

    if (
      Math.abs(currentSize.height - targetHeightPhysical) <= options.thresholdPx
    ) {
      return;
    }

    try {
      await options.resizeKeepTopLeft(currentSize.width, targetHeightPhysical);
    } catch (error) {
      await currentWindow.setSize(
        new PhysicalSize(currentSize.width, targetHeightPhysical)
      );
      const resizedPosition = await currentWindow.innerPosition();
      const shouldRestorePosition =
        resizedPosition.x !== currentPosition.x ||
        resizedPosition.y !== currentPosition.y;
      if (shouldRestorePosition) {
        await currentWindow.setPosition(
          new PhysicalPosition(currentPosition.x, currentPosition.y)
        );
      }
    }
  } catch (error) {
    console.warn("Failed to sync clip window height:", error);
  }
}

function nextAnimationFrame(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof window === "undefined") {
      resolve();
      return;
    }
    window.requestAnimationFrame(() => resolve());
  });
}

export async function applyTimelinePanelLayoutChange(options: {
  reason: string;
  targetHeight: number;
  applyStateBeforeResize?: boolean;
  applyStateChange?: (() => void | Promise<void>) | null;
  prepareLayoutReserve?: (() => void | Promise<void>) | null;
  cleanupLayoutReserve?: (() => void | Promise<void>) | null;
  isPreviewDisplayMenuVisible: boolean;
  setHidePreviewDisplayMenuDuringPanelTransition: (value: boolean) => void;
  getHidePreviewDisplayMenuDuringPanelTransition: () => boolean;
  setPreviewDisplayMenuInteractionLocked: (locked: boolean) => void;
  setPreviewOverlayLayersSuppressed: (suppressed: boolean) => void;
  setPreviewDisplayMetricsFrozen: (frozen: boolean) => void;
  suppressClipWindowResizeRefresh: (durationMs: number) => void;
  resizeRefreshSuppressMs: number;
  lockPreviewStageHeightForPanelTransition: () => void;
  unlockPreviewStageHeightForPanelTransition: () => void;
  tick: () => Promise<void>;
  syncClipWindowHeight: (force?: boolean, targetHeightOverride?: number) => Promise<void>;
  updateVideoDisplayMetrics: (reason: string) => void;
  usingMacOSNativePlayer: boolean;
  syncMacOSNativePlayerBounds: (force: boolean, reason: string) => Promise<void>;
  tauriEnv: boolean;
  waitForWindowBoundsStable: (maxMs?: number, settleMs?: number) => Promise<unknown>;
  syncPreviewLayoutAfterUiChange: () => Promise<void>;
  resumeMacOSNativePlayerBoundsSync: (reason: string) => void;
}): Promise<void> {
  const applyStateBeforeResize = options.applyStateBeforeResize ?? true;
  const applyStateChange = options.applyStateChange ?? null;
  const prepareLayoutReserve = options.prepareLayoutReserve ?? null;
  const cleanupLayoutReserve = options.cleanupLayoutReserve ?? null;

  if (options.isPreviewDisplayMenuVisible) {
    options.setHidePreviewDisplayMenuDuringPanelTransition(true);
  }
  options.setPreviewDisplayMenuInteractionLocked(true);
  options.setPreviewOverlayLayersSuppressed(true);
  options.setPreviewDisplayMetricsFrozen(true);
  options.suppressClipWindowResizeRefresh(options.resizeRefreshSuppressMs);
  options.lockPreviewStageHeightForPanelTransition();
  await options.tick();
  try {
    if (prepareLayoutReserve) {
      await prepareLayoutReserve();
      await options.tick();
    }
    if (applyStateBeforeResize && applyStateChange) {
      await applyStateChange();
      await options.tick();
    }
    await options.syncClipWindowHeight(true, options.targetHeight);
    options.setPreviewDisplayMetricsFrozen(false);
    options.updateVideoDisplayMetrics(`${options.reason}:panel-layout-post-resize`);
    await nextAnimationFrame();
    if (options.usingMacOSNativePlayer) {
      await options.syncMacOSNativePlayerBounds(
        true,
        `${options.reason}:panel-layout-post-resize`
      );
    }
    if (options.tauriEnv) {
      await options.waitForWindowBoundsStable(320, 90);
    }
    if (!applyStateBeforeResize && applyStateChange) {
      await applyStateChange();
      await options.tick();
    }
    await options.syncPreviewLayoutAfterUiChange();
    options.updateVideoDisplayMetrics(`${options.reason}:panel-layout-final-pre-resume`);
  } finally {
    options.setPreviewDisplayMetricsFrozen(false);
    options.unlockPreviewStageHeightForPanelTransition();
    await options.syncPreviewLayoutAfterUiChange();
    options.suppressClipWindowResizeRefresh(options.resizeRefreshSuppressMs);
    if (options.usingMacOSNativePlayer) {
      await options.syncMacOSNativePlayerBounds(true, `${options.reason}:panel-layout-finally`);
    }
    options.resumeMacOSNativePlayerBoundsSync(options.reason);
    await nextAnimationFrame();
    if (cleanupLayoutReserve) {
      await cleanupLayoutReserve();
      await options.tick();
    }
    options.setPreviewDisplayMenuInteractionLocked(false);
    options.setPreviewOverlayLayersSuppressed(false);
    if (options.getHidePreviewDisplayMenuDuringPanelTransition()) {
      options.setHidePreviewDisplayMenuDuringPanelTransition(false);
      await options.tick();
    }
  }
}
