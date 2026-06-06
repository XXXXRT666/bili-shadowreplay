import { getCurrentWindow } from "@tauri-apps/api/window";

export interface ClipWindowLifecycleState {
  closeRequestUnlisten: (() => void) | null;
  windowMoveUnlisten: (() => void) | null;
  windowResizeUnlisten: (() => void) | null;
  isHandlingWindowClose: boolean;
}

export interface UploadAccountOption {
  value: number;
  name: string;
  platform: string;
}

export function createClipWindowLifecycleState(): ClipWindowLifecycleState {
  return {
    closeRequestUnlisten: null,
    windowMoveUnlisten: null,
    windowResizeUnlisten: null,
    isHandlingWindowClose: false,
  };
}

export async function loadUploadAccounts(options: {
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
}): Promise<UploadAccountOption[]> {
  const accountInfo = (await options.invoke("get_accounts")) as {
    accounts?: Array<{ uid: number; name: string; platform: string }>;
  };
  return (accountInfo.accounts ?? [])
    .filter((account) => account.platform === "bilibili")
    .map((account) => ({
      value: account.uid,
      name: account.name,
      platform: account.platform,
    }));
}

export async function setupClipWindowLifecycle(options: {
  state: ClipWindowLifecycleState;
  tauriEnv: boolean;
  loadDanmakuEmoteMap: () => Promise<Record<string, unknown>>;
  onDanmakuEmoteMapLoaded: (map: Record<string, unknown>) => void;
  logInfo: (message: string, payload?: Record<string, unknown>) => void;
  logWarn: (message: string, payload?: Record<string, unknown>) => void;
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
  onConfigLoaded: (config: unknown) => void;
  getIsVideoLoaded: () => boolean;
  prepareSeekbarThumbnails: () => void;
  detectMacOSNativePlayerSupport: () => Promise<boolean>;
  onMacOSNativePlayerSupportResolved: (available: boolean) => void;
  shouldMountMacOSNativePlayer: () => boolean;
  tick: () => Promise<void>;
  mountMacOSNativePlayer: () => Promise<void>;
  onCloseCleanup: () => Promise<void>;
  updateVideoDisplayMetrics: (reason: string) => void;
  shouldSuppressClipWindowResizeRefresh: () => boolean;
  getIsDocumentFullscreen: () => boolean;
  getIsWebFullscreen: () => boolean;
  requestFullscreenLayoutRefresh: () => Promise<void>;
  addWindowEventListener: (type: string, listener: EventListenerOrEventListenerObject) => void;
  addDocumentEventListener: (
    type: string,
    listener: EventListenerOrEventListenerObject
  ) => void;
  handleViewportResize: EventListenerOrEventListenerObject;
  handleWindowFocus: EventListenerOrEventListenerObject;
  handleDocumentFullscreenChange: EventListenerOrEventListenerObject;
  syncClipWindowHeightAfterLayout: (force?: boolean, targetHeightOverride?: number) => void;
  saveSubtitles: () => Promise<void>;
  setAccounts: (accounts: UploadAccountOption[]) => void;
}): Promise<void> {
  const danmakuEmoteMap = await options.loadDanmakuEmoteMap();
  options.onDanmakuEmoteMapLoaded(danmakuEmoteMap);
  options.logInfo("VideoPreview danmaku emote map loaded", {
    count: Object.keys(danmakuEmoteMap).length,
  });

  try {
    const config = await options.invoke("get_config");
    options.onConfigLoaded(config);
    options.logInfo("Loaded clip preview config", {
      use_native_clip_player:
        (config as { use_native_clip_player?: boolean } | null)?.use_native_clip_player ??
        null,
      use_seekbar_thumbnail_cache:
        (config as { use_seekbar_thumbnail_cache?: boolean } | null)
          ?.use_seekbar_thumbnail_cache ?? null,
    });
    if (options.getIsVideoLoaded()) {
      options.prepareSeekbarThumbnails();
    }
  } catch (error) {
    console.error("Failed to load clip preview config:", error);
    options.logWarn("Failed to load clip preview config", {
      error: String(error),
    });
  }

  if (options.tauriEnv) {
    try {
      const macOSNativePlayerAvailable = await options.detectMacOSNativePlayerSupport();
      options.onMacOSNativePlayerSupportResolved(macOSNativePlayerAvailable);
      if (macOSNativePlayerAvailable && options.shouldMountMacOSNativePlayer()) {
        await options.tick();
        await options.mountMacOSNativePlayer();
      }
    } catch (error) {
      options.onMacOSNativePlayerSupportResolved(false);
      options.logWarn("Failed to detect macOS native player support", {
        error: String(error),
      });
    }

    try {
      options.state.closeRequestUnlisten = await getCurrentWindow().onCloseRequested(
        async (event) => {
          if (options.state.isHandlingWindowClose) {
            return;
          }

          event.preventDefault();
          options.state.isHandlingWindowClose = true;
          const currentWindow = getCurrentWindow();

          try {
            await options.onCloseCleanup();
          } catch (error) {
            console.warn("Failed to clean up clip window before close:", error);
          }

          try {
            if (options.state.closeRequestUnlisten) {
              options.state.closeRequestUnlisten();
              options.state.closeRequestUnlisten = null;
            }
            await currentWindow.close();
          } catch (error) {
            options.state.isHandlingWindowClose = false;
            console.warn("Failed to close clip window:", error);
          }
        }
      );
    } catch (error) {
      console.warn("Failed to register clip window close handler:", error);
    }

    try {
      options.state.windowMoveUnlisten = await getCurrentWindow().onMoved(() => {
        options.updateVideoDisplayMetrics("window-moved");
      });
    } catch (error) {
      console.warn("Failed to register clip window move handler:", error);
    }

    try {
      options.state.windowResizeUnlisten = await getCurrentWindow().onResized(() => {
        if (options.shouldSuppressClipWindowResizeRefresh()) {
          return;
        }

        if (options.getIsDocumentFullscreen() || options.getIsWebFullscreen()) {
          void options.requestFullscreenLayoutRefresh();
          return;
        }

        options.updateVideoDisplayMetrics("window-resized");
        if (typeof window !== "undefined") {
          window.requestAnimationFrame(() => {
            options.updateVideoDisplayMetrics("window-resized-raf");
          });
        }
      });
    } catch (error) {
      console.warn("Failed to register clip window resize handler:", error);
    }
  }

  options.addWindowEventListener("resize", options.handleViewportResize);
  options.addWindowEventListener("focus", options.handleWindowFocus);
  options.addDocumentEventListener(
    "fullscreenchange",
    options.handleDocumentFullscreenChange
  );
  options.syncClipWindowHeightAfterLayout(true);

  if (!options.tauriEnv && typeof window !== "undefined") {
    window.addEventListener("beforeunload", () => {
      void options.saveSubtitles();
    });
  }

  try {
    options.setAccounts(await loadUploadAccounts({ invoke: options.invoke }));
  } catch (error) {
    console.error("Failed to initialize upload data:", error);
  }
}

export function disposeClipWindowLifecycle(state: ClipWindowLifecycleState): void {
  if (state.closeRequestUnlisten) {
    state.closeRequestUnlisten();
    state.closeRequestUnlisten = null;
  }
  if (state.windowMoveUnlisten) {
    state.windowMoveUnlisten();
    state.windowMoveUnlisten = null;
  }
  if (state.windowResizeUnlisten) {
    state.windowResizeUnlisten();
    state.windowResizeUnlisten = null;
  }
}
