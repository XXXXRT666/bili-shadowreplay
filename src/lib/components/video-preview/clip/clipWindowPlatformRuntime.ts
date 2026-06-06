import { currentMonitor, getCurrentWindow, PhysicalPosition, PhysicalSize } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { focusMacOSNativePlayerHost } from "../macos-native/macosNativeClipPlayer";

export interface WindowBounds {
  position: PhysicalPosition;
  size: PhysicalSize;
}

export interface ClipWindowPlatformState {
  windowWideBeforeFullscreen: boolean | null;
  windowWideRestoreBounds: WindowBounds | null;
  fullscreenRestoreBounds: WindowBounds | null;
  windowWideTransitionId: number;
  fullscreenExitFocusTimers: number[];
  fullscreenStateSyncPending: boolean;
}

export function createClipWindowPlatformState(): ClipWindowPlatformState {
  return {
    windowWideBeforeFullscreen: null,
    windowWideRestoreBounds: null,
    fullscreenRestoreBounds: null,
    windowWideTransitionId: 0,
    fullscreenExitFocusTimers: [],
    fullscreenStateSyncPending: false,
  };
}

interface ClipWindowPlatformRuntimeOptions {
  tauriEnv: boolean;
  state: ClipWindowPlatformState;
  getShow: () => boolean;
  getIsWindowWide: () => boolean;
  setIsWindowWide: (value: boolean) => void;
  getIsDocumentFullscreen: () => boolean;
  setIsDocumentFullscreen: (value: boolean) => void;
  focusPreviewStage: () => void;
  scheduleTimelineRefresh: () => void;
  syncPreviewLayoutAfterUiChange: () => Promise<void>;
  updateVideoDisplayMetrics: (reason: string) => void;
}

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}

function lerpNumber(from: number, to: number, t: number): number {
  return from + (to - from) * t;
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

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    if (typeof window === "undefined") {
      resolve();
      return;
    }
    window.setTimeout(resolve, ms);
  });
}

function boundsMatch(a: WindowBounds, b: WindowBounds): boolean {
  return (
    a.position.x === b.position.x &&
    a.position.y === b.position.y &&
    a.size.width === b.size.width &&
    a.size.height === b.size.height
  );
}

async function getWindowInnerBounds(): Promise<WindowBounds> {
  const currentWindow = getCurrentWindow();
  const position = await currentWindow.innerPosition();
  const size = await currentWindow.innerSize();
  return { position, size };
}

async function getWindowWorkAreaBounds(): Promise<WindowBounds | null> {
  const monitor = await currentMonitor();
  if (!monitor) {
    return null;
  }
  return { position: monitor.workArea.position, size: monitor.workArea.size };
}

async function waitForWindowBoundsStable(maxMs = 320, settleMs = 80): Promise<WindowBounds | null> {
  if (typeof window === "undefined") {
    return null;
  }
  const now = () => window.performance?.now?.() ?? Date.now();
  const start = now();
  let lastBounds = await getWindowInnerBounds();
  let lastChange = start;

  while (now() - start < maxMs) {
    await sleep(30);
    const nextBounds = await getWindowInnerBounds();
    if (!boundsMatch(lastBounds, nextBounds)) {
      lastBounds = nextBounds;
      lastChange = now();
      continue;
    }
    if (now() - lastChange >= settleMs) {
      break;
    }
  }

  return lastBounds;
}

async function waitForWindowFullscreenExit(timeoutMs = 800): Promise<boolean> {
  const currentWindow = getCurrentWindow();
  const now = () => window.performance?.now?.() ?? Date.now();
  const start = now();
  while (now() - start < timeoutMs) {
    if (!(await currentWindow.isFullscreen())) {
      return true;
    }
    await sleep(50);
  }
  return !(await currentWindow.isFullscreen());
}

async function animateWindowTo(
  state: ClipWindowPlatformState,
  targetBounds: WindowBounds,
  updateVideoDisplayMetrics: (reason: string) => void,
  durationMs = 180
): Promise<void> {
  if (typeof window === "undefined") {
    return;
  }

  const currentWindow = getCurrentWindow();
  const transitionId = (state.windowWideTransitionId += 1);
  const startBounds = await getWindowInnerBounds();
  const startTime = window.performance?.now?.() ?? Date.now();
  const duration = Math.max(0, durationMs);

  if (duration === 0) {
    await currentWindow.setPosition(targetBounds.position);
    await currentWindow.setSize(targetBounds.size);
    return;
  }

  while (transitionId === state.windowWideTransitionId) {
    const now = window.performance?.now?.() ?? Date.now();
    const t = clamp01((now - startTime) / duration);
    const eased = 1 - Math.pow(1 - t, 2);
    const nextPosition = new PhysicalPosition(
      Math.round(lerpNumber(startBounds.position.x, targetBounds.position.x, eased)),
      Math.round(lerpNumber(startBounds.position.y, targetBounds.position.y, eased))
    );
    const nextSize = new PhysicalSize(
      Math.round(lerpNumber(startBounds.size.width, targetBounds.size.width, eased)),
      Math.round(lerpNumber(startBounds.size.height, targetBounds.size.height, eased))
    );

    await currentWindow.setPosition(nextPosition);
    await currentWindow.setSize(nextSize);
    updateVideoDisplayMetrics("animate-window");

    if (t >= 1) {
      break;
    }

    await nextAnimationFrame();
  }
}

export function createClipWindowPlatformRuntime(
  options: ClipWindowPlatformRuntimeOptions
) {
  async function applyPreviewHotkeyFocus(): Promise<void> {
    if (typeof window === "undefined" || typeof document === "undefined") {
      return;
    }

    if (options.getIsDocumentFullscreen() || !options.getShow()) {
      return;
    }

    if (options.tauriEnv) {
      try {
        await focusMacOSNativePlayerHost();
      } catch {}
      try {
        await getCurrentWindow().setFocus();
      } catch {}
      try {
        await getCurrentWebview().setFocus();
      } catch {}
    } else if (typeof window.focus === "function") {
      window.focus();
    }

    options.focusPreviewStage();
  }

  function cancelFullscreenExitFocusRestore(): void {
    if (typeof window === "undefined") {
      options.state.fullscreenExitFocusTimers = [];
      return;
    }

    if (options.state.fullscreenExitFocusTimers.length > 0) {
      options.state.fullscreenExitFocusTimers.forEach((timerId) => {
        window.clearTimeout(timerId);
      });
      options.state.fullscreenExitFocusTimers = [];
    }
  }

  function restorePreviewHotkeyFocusAfterFullscreenExit(): void {
    if (typeof window === "undefined" || typeof document === "undefined") {
      return;
    }

    const tryFocus = () => {
      void applyPreviewHotkeyFocus();
    };

    cancelFullscreenExitFocusRestore();
    tryFocus();
    window.requestAnimationFrame(() => {
      tryFocus();
    });
    options.state.fullscreenExitFocusTimers = [80, 180, 320, 520, 800, 1200].map(
      (delay) =>
        window.setTimeout(() => {
          tryFocus();
        }, delay)
    );
  }

  async function restoreWindowWideAfterFullscreen(): Promise<void> {
    if (options.state.windowWideBeforeFullscreen === null) {
      return;
    }

    const shouldBeWide = options.state.windowWideBeforeFullscreen;
    options.state.windowWideBeforeFullscreen = null;

    if (!options.tauriEnv) {
      options.setIsWindowWide(shouldBeWide);
      options.state.fullscreenRestoreBounds = null;
      return;
    }

    const currentWindow = getCurrentWindow();
    try {
      if (shouldBeWide) {
        if (!options.getIsWindowWide()) {
          await waitForWindowBoundsStable();
          const targetBounds = await getWindowWorkAreaBounds();
          if (targetBounds) {
            await animateWindowTo(
              options.state,
              targetBounds,
              options.updateVideoDisplayMetrics
            );
            options.setIsWindowWide(true);
          } else {
            await currentWindow.maximize();
            options.setIsWindowWide(await currentWindow.isMaximized());
          }
        }
        return;
      }

      if (options.getIsWindowWide()) {
        await waitForWindowBoundsStable();
        const restoreBounds = options.state.windowWideRestoreBounds;
        if (restoreBounds) {
          await currentWindow.unmaximize();
          await animateWindowTo(
            options.state,
            restoreBounds,
            options.updateVideoDisplayMetrics
          );
        } else {
          await currentWindow.unmaximize();
        }
        options.state.windowWideRestoreBounds = null;
      }
      options.setIsWindowWide(false);
    } catch (error) {
      console.warn("Failed to restore window wide state:", error);
    }
    options.state.fullscreenRestoreBounds = null;
  }

  async function syncFullscreenState(): Promise<void> {
    if (options.state.fullscreenStateSyncPending) {
      return;
    }

    const previousFullscreen = options.getIsDocumentFullscreen();
    options.state.fullscreenStateSyncPending = true;

    try {
      if (options.tauriEnv) {
        options.setIsDocumentFullscreen(await getCurrentWindow().isFullscreen());
      } else if (typeof document !== "undefined") {
        options.setIsDocumentFullscreen(Boolean(document.fullscreenElement));
      } else {
        options.setIsDocumentFullscreen(false);
      }
    } catch {
      if (!options.tauriEnv && typeof document !== "undefined") {
        options.setIsDocumentFullscreen(Boolean(document.fullscreenElement));
      }
    } finally {
      options.state.fullscreenStateSyncPending = false;
    }

    if (previousFullscreen && !options.getIsDocumentFullscreen()) {
      await restoreWindowWideAfterFullscreen();
      restorePreviewHotkeyFocusAfterFullscreenExit();
    } else if (
      !previousFullscreen &&
      options.getIsDocumentFullscreen() &&
      options.state.windowWideBeforeFullscreen === null
    ) {
      options.state.windowWideBeforeFullscreen =
        options.getIsWindowWide() || options.state.windowWideRestoreBounds !== null;
    }
  }

  async function requestFullscreenLayoutRefresh(): Promise<void> {
    await syncFullscreenState();
    options.updateVideoDisplayMetrics("fullscreen-refresh:sync");

    if (typeof window === "undefined") {
      return;
    }

    window.requestAnimationFrame(() => {
      options.updateVideoDisplayMetrics("fullscreen-refresh:raf1");
      window.requestAnimationFrame(() => {
        options.updateVideoDisplayMetrics("fullscreen-refresh:raf2");
      });
    });

    window.setTimeout(() => {
      options.updateVideoDisplayMetrics("fullscreen-refresh:timeout");
    }, 60);
  }

  async function toggleFullscreen(): Promise<void> {
    try {
      if (options.tauriEnv) {
        const currentWindow = getCurrentWindow();
        const isCurrentlyFullscreen = await currentWindow.isFullscreen();
        if (!isCurrentlyFullscreen && options.state.windowWideBeforeFullscreen === null) {
          options.state.windowWideBeforeFullscreen =
            options.getIsWindowWide() ||
            options.state.windowWideRestoreBounds !== null;
          options.state.fullscreenRestoreBounds = await getWindowInnerBounds();
        }
        const nextFullscreen = !isCurrentlyFullscreen;
        await currentWindow.setFullscreen(nextFullscreen);
        options.setIsDocumentFullscreen(nextFullscreen);
        if (!nextFullscreen) {
          await restoreWindowWideAfterFullscreen();
          restorePreviewHotkeyFocusAfterFullscreenExit();
        }
        await requestFullscreenLayoutRefresh();
        return;
      }

      if (typeof document === "undefined") {
        return;
      }

      if (document.fullscreenElement) {
        if (typeof document.exitFullscreen === "function") {
          await document.exitFullscreen();
        }
        return;
      }

      if (typeof document.documentElement.requestFullscreen === "function") {
        if (options.state.windowWideBeforeFullscreen === null) {
          options.state.windowWideBeforeFullscreen =
            options.getIsWindowWide() ||
            options.state.windowWideRestoreBounds !== null;
        }
        await document.documentElement.requestFullscreen();
      }
    } catch (error) {
      console.warn("Failed to toggle clip fullscreen:", error);
    }
  }

  async function toggleWindowWide(): Promise<void> {
    if (!options.tauriEnv) {
      if (typeof document !== "undefined" && document.fullscreenElement) {
        if (typeof document.exitFullscreen === "function") {
          await document.exitFullscreen();
        }
        options.state.windowWideBeforeFullscreen = false;
        await restoreWindowWideAfterFullscreen();
        options.scheduleTimelineRefresh();
        await options.syncPreviewLayoutAfterUiChange();
        return;
      }
      options.setIsWindowWide(!options.getIsWindowWide());
      options.scheduleTimelineRefresh();
      await options.syncPreviewLayoutAfterUiChange();
      return;
    }

    const currentWindow = getCurrentWindow();

    try {
      const isFullscreen = await currentWindow.isFullscreen();
      if (isFullscreen) {
        const inferredWasWide =
          options.getIsWindowWide() ||
          options.state.windowWideRestoreBounds !== null;
        const wasWide =
          options.state.windowWideBeforeFullscreen ?? inferredWasWide;
        if (
          !wasWide &&
          options.state.fullscreenRestoreBounds &&
          !options.state.windowWideRestoreBounds
        ) {
          options.state.windowWideRestoreBounds =
            options.state.fullscreenRestoreBounds;
        }
        options.state.windowWideBeforeFullscreen = null;
        await currentWindow.setFullscreen(false);
        options.setIsDocumentFullscreen(false);
        await waitForWindowFullscreenExit();
        await waitForWindowBoundsStable();
        await nextAnimationFrame();
        if (wasWide) {
          const restoreBounds = options.state.windowWideRestoreBounds;
          if (restoreBounds) {
            await currentWindow.unmaximize();
            await animateWindowTo(
              options.state,
              restoreBounds,
              options.updateVideoDisplayMetrics,
              0
            );
          } else {
            await currentWindow.unmaximize();
          }
          options.setIsWindowWide(false);
          options.state.windowWideRestoreBounds = null;
        } else {
          const targetBounds = await getWindowWorkAreaBounds();
          if (targetBounds) {
            await animateWindowTo(
              options.state,
              targetBounds,
              options.updateVideoDisplayMetrics
            );
            options.setIsWindowWide(true);
          } else {
            await currentWindow.maximize();
            options.setIsWindowWide(await currentWindow.isMaximized());
          }
        }
        options.scheduleTimelineRefresh();
        await requestFullscreenLayoutRefresh();
        options.state.fullscreenRestoreBounds = null;
        return;
      }

      if (!options.getIsWindowWide()) {
        if (!options.state.windowWideRestoreBounds) {
          options.state.windowWideRestoreBounds = await getWindowInnerBounds();
        }
        const targetBounds = await getWindowWorkAreaBounds();
        if (targetBounds) {
          await animateWindowTo(
            options.state,
            targetBounds,
            options.updateVideoDisplayMetrics
          );
          options.setIsWindowWide(true);
        } else {
          await currentWindow.maximize();
          options.setIsWindowWide(await currentWindow.isMaximized());
        }
      } else {
        const restoreBounds = options.state.windowWideRestoreBounds;
        if (restoreBounds) {
          await currentWindow.unmaximize();
          await animateWindowTo(
            options.state,
            restoreBounds,
            options.updateVideoDisplayMetrics
          );
        } else {
          await currentWindow.unmaximize();
        }
        options.setIsWindowWide(false);
        options.state.windowWideRestoreBounds = null;
      }
    } catch (error) {
      console.warn("Failed to toggle window wide mode:", error);
    }

    options.scheduleTimelineRefresh();
    await requestFullscreenLayoutRefresh();
    if (typeof window !== "undefined") {
      window.setTimeout(() => {
        options.updateVideoDisplayMetrics("toggle-window-wide-timeout");
      }, 180);
    }
  }

  return {
    cancelFullscreenExitFocusRestore,
    restorePreviewHotkeyFocusAfterFullscreenExit,
    syncFullscreenState,
    requestFullscreenLayoutRefresh,
    waitForWindowBoundsStable,
    toggleFullscreen,
    toggleWindowWide,
  };
}
