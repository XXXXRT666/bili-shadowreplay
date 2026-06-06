import { invoke, log } from "../../../invoker";

export const MACOS_NATIVE_PLAYER_ID = "clip-preview";
export const MACOS_NATIVE_PLAYER_POLL_INTERVAL_MS = 50;
export const MACOS_NATIVE_PLAYER_POPUP_FALLBACK_ENABLED = false;
export const DEFAULT_MACOS_NATIVE_PLAYER_WINDOWED_PREVIEW_COMPENSATION = 28;
export const MACOS_NATIVE_PLAYER_POLL_MISSING_GRACE_MS = 1500;
export const MACOS_NATIVE_PLAYER_MAX_CONSECUTIVE_POLL_FAILURES = 3;

export interface MacOSNativePlayerState {
  currentTime: number;
  duration: number;
  rate: number;
  playing: boolean;
}

export interface MacOSNativePlayerTargetRect {
  x: number;
  y: number;
  width: number;
  height: number;
  windowedYOffset: number;
}

export interface MacOSNativePlayerBackdropSnapshot {
  htmlBackground: string;
  bodyBackground: string;
  appBackground: string;
  htmlBackgroundColor: string;
  bodyBackgroundColor: string;
  appBackgroundColor: string;
}

interface MacOSNativePlayerRuntimeOptions {
  playerId?: string;
  pollIntervalMs?: number;
  tauriEnv: boolean;
  getVideoSource: () => string | null | undefined;
  getSlotElement: () => HTMLDivElement | null;
  getTargetRect: () => MacOSNativePlayerTargetRect | null;
  getViewportSignatureParts: () => string[];
  getIsDocumentFullscreen: () => boolean;
  getShouldUseMacOSNativePlayer: () => boolean;
  getUsingMacOSNativePlayer: () => boolean;
  getIsMacOSNativePlayerActive: () => boolean;
  getMacOSNativePlayerAvailable: () => boolean;
  setMacOSNativePlayerMountFailed: (value: boolean) => void;
  setIsMacOSNativePlayerActive: (value: boolean) => void;
  setForceDomPreviewHidden: (value: boolean) => void;
  getVolume: () => number;
  getVideoElement: () => HTMLVideoElement | null;
  shouldHideDomPreview: () => boolean;
  onPlaybackState: (state: MacOSNativePlayerState) => void;
  onPlaybackEnded: () => void;
}

export function computeMacOSNativePlayerWindowedYOffset(options: {
  tauriEnv: boolean;
  isDocumentFullscreen: boolean;
  compensation: number;
}) {
  if (!options.tauriEnv || options.isDocumentFullscreen) {
    return 0;
  }

  return options.compensation;
}

export function computeMacOSNativePlayerTargetRect(options: {
  previewStageElement: HTMLDivElement | null;
  previewVideoFrameLeft: number;
  previewVideoFrameTop: number;
  previewVideoFrameWidth: number;
  previewVideoFrameHeight: number;
  shouldUseMacOSNativePlayer: boolean;
  windowedYOffset: number;
}) {
  const { previewStageElement } = options;
  if (!previewStageElement) {
    return null;
  }

  const stageRect = previewStageElement.getBoundingClientRect();
  const innerLeft =
    options.shouldUseMacOSNativePlayer && options.previewVideoFrameWidth > 0
      ? options.previewVideoFrameLeft
      : 0;
  const innerTop =
    options.shouldUseMacOSNativePlayer && options.previewVideoFrameHeight > 0
      ? options.previewVideoFrameTop
      : 0;
  const innerWidth =
    options.shouldUseMacOSNativePlayer && options.previewVideoFrameWidth > 0
      ? options.previewVideoFrameWidth
      : stageRect.width;
  const innerHeight =
    options.shouldUseMacOSNativePlayer && options.previewVideoFrameHeight > 0
      ? options.previewVideoFrameHeight
      : stageRect.height;

  if (innerWidth <= 0 || innerHeight <= 0) {
    return null;
  }

  return {
    x: stageRect.left + innerLeft,
    y: stageRect.top + innerTop + options.windowedYOffset,
    width: innerWidth,
    height: innerHeight,
    windowedYOffset: options.windowedYOffset,
  } satisfies MacOSNativePlayerTargetRect;
}

export function getMacOSNativePlayerViewportSignatureParts() {
  if (typeof window === "undefined") {
    return ["0", "0", "0", "0"];
  }

  return [
    String(Math.round(window.screenX ?? 0)),
    String(Math.round(window.screenY ?? 0)),
    String(Math.round(window.innerWidth ?? 0)),
    String(Math.round(window.innerHeight ?? 0)),
  ];
}

export function shouldHideMacOSNativeDomPreview(options: {
  forceDomPreviewHidden: boolean;
  usingMacOSNativePlayer: boolean;
  shouldPreferMacOSNativePlayerPresentation: boolean;
}) {
  return (
    options.forceDomPreviewHidden ||
    options.usingMacOSNativePlayer ||
    options.shouldPreferMacOSNativePlayerPresentation
  );
}

export function syncMacOSNativeDomPreviewVisibility(options: {
  videoElement: HTMLVideoElement | null;
  shouldHideDomPreview: boolean;
}) {
  const applyVisibility = (element: HTMLVideoElement | null) => {
    if (!element) {
      return;
    }

    if (options.shouldHideDomPreview) {
      element.style.setProperty("opacity", "0", "important");
      element.style.setProperty("visibility", "hidden", "important");
      element.style.setProperty("pointer-events", "none", "important");
      return;
    }

    element.style.removeProperty("opacity");
    element.style.removeProperty("visibility");
    element.style.removeProperty("pointer-events");
  };

  applyVisibility(options.videoElement);
}

export function applyMacOSNativePlayerBackdrop(
  snapshot: MacOSNativePlayerBackdropSnapshot | null
) {
  if (typeof document === "undefined" || snapshot) {
    return snapshot;
  }

  const appRoot = document.getElementById("app");
  const nextSnapshot: MacOSNativePlayerBackdropSnapshot = {
    htmlBackground: document.documentElement.style.background,
    bodyBackground: document.body.style.background,
    appBackground: appRoot?.style.background ?? "",
    htmlBackgroundColor: document.documentElement.style.backgroundColor,
    bodyBackgroundColor: document.body.style.backgroundColor,
    appBackgroundColor: appRoot?.style.backgroundColor ?? "",
  };

  document.documentElement.style.background = "transparent";
  document.documentElement.style.backgroundColor = "transparent";
  document.body.style.background = "transparent";
  document.body.style.backgroundColor = "transparent";
  if (appRoot) {
    appRoot.style.background = "transparent";
    appRoot.style.backgroundColor = "transparent";
  }

  return nextSnapshot;
}

export function restoreMacOSNativePlayerBackdrop(
  snapshot: MacOSNativePlayerBackdropSnapshot | null
) {
  if (typeof document === "undefined" || !snapshot) {
    return null;
  }

  const appRoot = document.getElementById("app");

  document.documentElement.style.background = snapshot.htmlBackground;
  document.documentElement.style.backgroundColor = snapshot.htmlBackgroundColor;
  document.body.style.background = snapshot.bodyBackground;
  document.body.style.backgroundColor = snapshot.bodyBackgroundColor;
  if (appRoot) {
    appRoot.style.background = snapshot.appBackground;
    appRoot.style.backgroundColor = snapshot.appBackgroundColor;
  }

  return null;
}

export async function detectMacOSNativePlayerSupport() {
  return Boolean(await invoke("macos_native_player_supported"));
}

export async function focusMacOSNativePlayerHost() {
  await invoke("macos_native_player_focus_host");
}

export async function setMacOSNativePlayerRate(rate: number, playerId = MACOS_NATIVE_PLAYER_ID) {
  await invoke("macos_native_player_set_rate", { playerId, rate });
}

export async function pauseMacOSNativePlayer(playerId = MACOS_NATIVE_PLAYER_ID) {
  await invoke("macos_native_player_pause", { playerId });
}

export async function seekMacOSNativePlayer(seconds: number, playerId = MACOS_NATIVE_PLAYER_ID) {
  await invoke("macos_native_player_seek", { playerId, seconds });
}

export function createMacOSNativeClipPlayerRuntime(options: MacOSNativePlayerRuntimeOptions) {
  const playerId = options.playerId ?? MACOS_NATIVE_PLAYER_ID;
  const pollIntervalMs = options.pollIntervalMs ?? MACOS_NATIVE_PLAYER_POLL_INTERVAL_MS;
  let macOSNativePlayerPollTimer: number | null = null;
  let macOSNativePlayerBoundsFollowTimer: number | null = null;
  let macOSNativePlayerStatePollPending = false;
  let macOSNativePlayerPollMissingGraceUntilMs = 0;
  let macOSNativePlayerConsecutivePollFailures = 0;
  let macOSNativePlayerBoundsSyncSuspendCount = 0;
  let lastMacOSNativePlayerBoundsSignature = "";
  let macOSNativeBackdropSnapshot: MacOSNativePlayerBackdropSnapshot | null = null;
  let popupFallbackVisible = false;

  function getNowMs() {
    if (typeof performance !== "undefined" && typeof performance.now === "function") {
      return performance.now();
    }

    return Date.now();
  }

  function syncPreviewDomVisibility() {
    syncMacOSNativeDomPreviewVisibility({
      videoElement: options.getVideoElement(),
      shouldHideDomPreview: options.shouldHideDomPreview(),
    });
  }

  function stopBoundsFollowLoop() {
    if (macOSNativePlayerBoundsFollowTimer !== null && typeof window !== "undefined") {
      window.clearInterval(macOSNativePlayerBoundsFollowTimer);
      macOSNativePlayerBoundsFollowTimer = null;
    }
    lastMacOSNativePlayerBoundsSignature = "";
  }

  function stopPolling() {
    if (macOSNativePlayerPollTimer !== null && typeof window !== "undefined") {
      window.clearInterval(macOSNativePlayerPollTimer);
      macOSNativePlayerPollTimer = null;
    }
    macOSNativePlayerStatePollPending = false;
    macOSNativePlayerConsecutivePollFailures = 0;
  }

  function handleMissing(reason: string) {
    if (!options.getIsMacOSNativePlayerActive() && !options.getUsingMacOSNativePlayer()) {
      return;
    }

    void log.warn("macOS native player missing, fallback to DOM preview", {
      reason,
      playerId,
    });
    stopBoundsFollowLoop();
    stopPolling();
    options.setIsMacOSNativePlayerActive(false);
    options.setForceDomPreviewHidden(false);
    popupFallbackVisible = false;
    syncPreviewDomVisibility();
    macOSNativeBackdropSnapshot = restoreMacOSNativePlayerBackdrop(macOSNativeBackdropSnapshot);
  }

  async function syncBounds(force = false, reason = "unknown") {
    if (!options.getUsingMacOSNativePlayer() || !options.getSlotElement()) {
      return;
    }

    if (macOSNativePlayerBoundsSyncSuspendCount > 0 && !force) {
      return;
    }

    const targetRect = options.getTargetRect();
    if (!targetRect) {
      return;
    }

    const rectSignature = [
      Math.round(targetRect.x),
      Math.round(targetRect.y),
      Math.round(targetRect.width),
      Math.round(targetRect.height),
      options.getIsDocumentFullscreen() ? 1 : 0,
      ...options.getViewportSignatureParts(),
    ].join(":");

    if (rectSignature === lastMacOSNativePlayerBoundsSignature) {
      return;
    }

    lastMacOSNativePlayerBoundsSignature = rectSignature;

    try {
      const updated = (await invoke("macos_native_player_update_bounds", {
        playerId,
        x: targetRect.x,
        y: targetRect.y,
        width: targetRect.width,
        height: targetRect.height,
        windowedYOffset: targetRect.windowedYOffset,
        isFullscreen: options.getIsDocumentFullscreen(),
        reason,
      })) as boolean;
      if (!updated) {
        handleMissing("bounds-missing");
      }
    } catch (error) {
      console.warn("Failed to update macOS native player bounds:", error);
      handleMissing("bounds-error");
    }
  }

  async function syncVolume() {
    if (!options.getUsingMacOSNativePlayer()) {
      return;
    }

    try {
      await invoke("macos_native_player_set_volume", {
        playerId,
        volume: options.getVolume(),
      });
    } catch (error) {
      console.warn("Failed to sync macOS native volume:", error);
    }
  }

  async function pollState() {
    if (!options.getUsingMacOSNativePlayer() || macOSNativePlayerStatePollPending) {
      return;
    }

    macOSNativePlayerStatePollPending = true;

    try {
      const state = (await invoke("macos_native_player_get_state", {
        playerId,
      })) as MacOSNativePlayerState;

      if (!options.getUsingMacOSNativePlayer()) {
        return;
      }

      macOSNativePlayerConsecutivePollFailures = 0;
      options.onPlaybackState(state);
      if (
        state.duration > 0 &&
        state.currentTime >= Math.max(0, state.duration - 0.05) &&
        !state.playing
      ) {
        options.onPlaybackEnded();
      }
    } catch (error) {
      if (options.getUsingMacOSNativePlayer()) {
        console.warn("Failed to poll macOS native player state:", error);
        macOSNativePlayerConsecutivePollFailures += 1;
        const withinGraceWindow = getNowMs() < macOSNativePlayerPollMissingGraceUntilMs;
        if (
          withinGraceWindow ||
          macOSNativePlayerConsecutivePollFailures <
            MACOS_NATIVE_PLAYER_MAX_CONSECUTIVE_POLL_FAILURES
        ) {
          return;
        }
        handleMissing("state-poll-error");
      }
    } finally {
      macOSNativePlayerStatePollPending = false;
    }
  }

  function startBoundsFollowLoop() {
    if (macOSNativePlayerBoundsFollowTimer !== null || typeof window === "undefined") {
      return;
    }

    macOSNativePlayerBoundsFollowTimer = window.setInterval(() => {
      void syncBounds(false, "follow-loop");
    }, 16);
  }

  function startPolling() {
    if (macOSNativePlayerPollTimer !== null || typeof window === "undefined") {
      return;
    }

    void pollState();
    macOSNativePlayerPollTimer = window.setInterval(() => {
      void pollState();
    }, pollIntervalMs);
  }

  async function mount() {
    if (!options.getShouldUseMacOSNativePlayer() || !options.getSlotElement()) {
      return false;
    }

    const source = options.getVideoSource();
    if (!source) {
      return false;
    }

    const targetRect = options.getTargetRect();
    if (!targetRect) {
      return false;
    }

    options.setMacOSNativePlayerMountFailed(false);
    options.setForceDomPreviewHidden(true);
    syncPreviewDomVisibility();

    try {
      await invoke("macos_native_player_mount", {
        playerId,
        source,
        x: targetRect.x,
        y: targetRect.y,
        width: targetRect.width,
        height: targetRect.height,
        windowedYOffset: targetRect.windowedYOffset,
        isFullscreen: options.getIsDocumentFullscreen(),
      });
      options.setMacOSNativePlayerMountFailed(false);
      options.setIsMacOSNativePlayerActive(true);
      macOSNativePlayerConsecutivePollFailures = 0;
      macOSNativePlayerPollMissingGraceUntilMs =
        getNowMs() + MACOS_NATIVE_PLAYER_POLL_MISSING_GRACE_MS;
      options.setForceDomPreviewHidden(false);
      syncPreviewDomVisibility();
      macOSNativeBackdropSnapshot = applyMacOSNativePlayerBackdrop(macOSNativeBackdropSnapshot);
      await syncBounds(true, "mount-post");
      await syncVolume();
      startBoundsFollowLoop();
      startPolling();
      return true;
    } catch (error) {
      options.setMacOSNativePlayerMountFailed(true);
      options.setIsMacOSNativePlayerActive(false);
      options.setForceDomPreviewHidden(false);
      syncPreviewDomVisibility();
      macOSNativeBackdropSnapshot = restoreMacOSNativePlayerBackdrop(
        macOSNativeBackdropSnapshot
      );
      stopBoundsFollowLoop();
      stopPolling();
      console.warn("Failed to mount macOS native player:", error);
      return false;
    }
  }

  async function syncPresentationMode(shouldUseFallback: boolean) {
    if (
      !options.tauriEnv ||
      !options.getMacOSNativePlayerAvailable() ||
      !options.getIsMacOSNativePlayerActive()
    ) {
      popupFallbackVisible = false;
      return;
    }

    if (popupFallbackVisible === shouldUseFallback) {
      return;
    }

    popupFallbackVisible = shouldUseFallback;

    try {
      await invoke("macos_native_player_set_presentation_mode", {
        playerId,
        useFallback: shouldUseFallback,
      });
    } catch (error) {
      popupFallbackVisible = !shouldUseFallback;
      console.warn("Failed to sync macOS native player popup presentation mode:", error);
    }
  }

  async function teardown() {
    const shouldUnmount =
      options.tauriEnv &&
      options.getMacOSNativePlayerAvailable() &&
      options.getIsMacOSNativePlayerActive();

    stopBoundsFollowLoop();
    stopPolling();
    options.setIsMacOSNativePlayerActive(false);
    options.setForceDomPreviewHidden(false);
    popupFallbackVisible = false;
    syncPreviewDomVisibility();
    macOSNativeBackdropSnapshot = restoreMacOSNativePlayerBackdrop(
      macOSNativeBackdropSnapshot
    );

    if (!shouldUnmount) {
      return;
    }

    try {
      await invoke("macos_native_player_unmount", { playerId });
    } catch (error) {
      console.warn("Failed to unmount macOS native player:", error);
    }
  }

  return {
    mount,
    syncBounds,
    syncVolume,
    syncPresentationMode,
    startBoundsFollowLoop,
    stopBoundsFollowLoop,
    startPolling,
    stopPolling,
    pollState,
    handleMissing,
    teardown,
    suspendBoundsSync(reason: string) {
      void reason;
      macOSNativePlayerBoundsSyncSuspendCount += 1;
    },
    resumeBoundsSync(reason: string) {
      void reason;
      macOSNativePlayerBoundsSyncSuspendCount = Math.max(
        0,
        macOSNativePlayerBoundsSyncSuspendCount - 1
      );
    },
    syncDomPreviewVisibility: syncPreviewDomVisibility,
    restoreBackdrop() {
      macOSNativeBackdropSnapshot = restoreMacOSNativePlayerBackdrop(
        macOSNativeBackdropSnapshot
      );
    },
  };
}
