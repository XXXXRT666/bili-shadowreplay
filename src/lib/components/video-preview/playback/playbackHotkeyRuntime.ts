interface PlaybackHotkeyRuntimeOptions {
  fastForwardHoldDelayMs: number;
  fastForwardPlaybackRate: number;
  getShow: () => boolean;
  getIsVideoLoaded: () => boolean;
  getVideoElement: () => HTMLVideoElement | undefined;
  getVideo: () => unknown;
  getIsPlaying: () => boolean;
  setIsPlaying: (value: boolean) => void;
  getCurrentPlaybackRate: () => number;
  setCurrentPlaybackRateValue: (value: number) => void;
  getSelectedPlaybackRate: () => number;
  getIsRightArrowFastForwardActive: () => boolean;
  setIsRightArrowFastForwardActive: (value: boolean) => void;
  getRightArrowFastForwardWasPlaying: () => boolean;
  setRightArrowFastForwardWasPlaying: (value: boolean) => void;
  getRightArrowPreviousPlaybackRate: () => number;
  setRightArrowPreviousPlaybackRate: (value: number) => void;
  getRightArrowHoldTimeout: () => ReturnType<typeof setTimeout> | null;
  setRightArrowHoldTimeout: (value: ReturnType<typeof setTimeout> | null) => void;
  usingMacOSNativePlayer: () => boolean;
  playPrimaryPlayback: () => void;
  pausePrimaryPlayback: () => void;
  getPlaybackCurrentTime: () => number;
  setPreviewTime: (time: number) => void;
  syncWaveformWithVideo: () => void;
  setCurrentPlaybackRate: (rate: number, refreshDanmu?: boolean) => void;
  canBeClipped: (video: unknown) => boolean;
  setClipStartTime: () => void;
  setClipEndTime: () => void;
  seekToClipStart: () => void;
  seekToClipEnd: () => void;
  generateClip: () => void | Promise<void>;
  clearClipSelection: () => void;
  getShowDetail: () => boolean;
  setShowDetail: (value: boolean) => void;
}

export function isBlockedHotkeyTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  const tagName = element?.tagName;

  return (
    (!!tagName && ["INPUT", "TEXTAREA", "SELECT"].includes(tagName)) ||
    !!element?.isContentEditable ||
    !!element?.closest(
      "input, textarea, select, [contenteditable='true'], [data-hotkey-block]",
    )
  );
}

export function isTextEntryHotkeyTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  if (!element) {
    return false;
  }

  if (element.tagName === "TEXTAREA" || element.isContentEditable) {
    return true;
  }

  if (element.tagName === "INPUT") {
    const input = element as HTMLInputElement;
    const inputType = (input.type || "text").toLowerCase();
    const nonTextInputTypes = new Set([
      "button",
      "checkbox",
      "color",
      "file",
      "hidden",
      "image",
      "radio",
      "range",
      "reset",
      "submit",
    ]);
    return !nonTextInputTypes.has(inputType);
  }

  return !!element.closest("textarea, [contenteditable='true']");
}

export function createPlaybackHotkeyRuntime(
  options: PlaybackHotkeyRuntimeOptions,
) {
  const startRateBasedFastForward = () => {
    if (options.getIsPlaying()) {
      if (!options.getRightArrowFastForwardWasPlaying()) {
        options.setRightArrowFastForwardWasPlaying(true);
      }
    }

    if (!options.getRightArrowFastForwardWasPlaying()) {
      options.playPrimaryPlayback();
      options.setIsPlaying(true);
    }

    options.setCurrentPlaybackRate(options.fastForwardPlaybackRate, false);
  };

  const startRightArrowFastForward = async () => {
    if (!options.getVideoElement() || options.getIsRightArrowFastForwardActive()) {
      return;
    }

    options.setRightArrowFastForwardWasPlaying(options.getIsPlaying());
    options.setRightArrowPreviousPlaybackRate(options.getSelectedPlaybackRate());
    options.setIsRightArrowFastForwardActive(true);
    startRateBasedFastForward();
  };

  const resetRightArrowFastForward = async (
    shouldSeekOnTap: boolean,
    reason = "reset",
  ) => {
    const holdTimeout = options.getRightArrowHoldTimeout();
    const hadPendingHold = holdTimeout !== null;

    if (holdTimeout !== null) {
      clearTimeout(holdTimeout);
      options.setRightArrowHoldTimeout(null);
    }

    const videoElement = options.getVideoElement();
    if (options.getIsRightArrowFastForwardActive() && videoElement) {
      if (
        options.usingMacOSNativePlayer() &&
        !options.getRightArrowFastForwardWasPlaying()
      ) {
        options.pausePrimaryPlayback();
        options.setCurrentPlaybackRateValue(
          options.getRightArrowPreviousPlaybackRate() ||
            options.getSelectedPlaybackRate(),
        );
        options.setIsPlaying(false);
      } else {
        options.setCurrentPlaybackRate(
          options.getRightArrowPreviousPlaybackRate() ||
            options.getSelectedPlaybackRate(),
          false,
        );

        if (!options.getRightArrowFastForwardWasPlaying()) {
          options.pausePrimaryPlayback();
          options.setIsPlaying(false);
        }
      }

      options.syncWaveformWithVideo();
    } else if (shouldSeekOnTap && hadPendingHold && videoElement) {
      options.setPreviewTime(
        Math.min(videoElement.duration, options.getPlaybackCurrentTime() + 5),
      );
    }

    options.setIsRightArrowFastForwardActive(false);
    options.setRightArrowFastForwardWasPlaying(false);
    options.setRightArrowPreviousPlaybackRate(1);
    void reason;
  };

  const togglePlay = () => {
    const nextIsPlaying = !options.getIsPlaying();
    if (options.usingMacOSNativePlayer()) {
      if (nextIsPlaying) {
        options.playPrimaryPlayback();
      } else {
        options.pausePrimaryPlayback();
      }
      options.setIsPlaying(nextIsPlaying);
      return;
    }

    const videoElement = options.getVideoElement();
    if (!videoElement) {
      return;
    }

    if (nextIsPlaying) {
      void videoElement.play().catch((error) => {
        console.warn("Failed to toggle playback:", error);
      });
    } else {
      videoElement.pause();
    }
    options.setIsPlaying(nextIsPlaying);
  };

  const handleKeydown = (event: KeyboardEvent) => {
    if (!options.getShow() || !options.getIsVideoLoaded()) {
      return;
    }

    const video = options.getVideo();
    const isInInput = isBlockedHotkeyTarget(event.target);

    switch (event.key) {
      case "【":
      case "[":
        if (!isInInput && options.canBeClipped(video)) {
          event.preventDefault();
          options.setClipStartTime();
        }
        break;
      case "】":
      case "]":
        if (!isInInput && options.canBeClipped(video)) {
          event.preventDefault();
          options.setClipEndTime();
        }
        break;
      case "q":
      case "Q":
        if (!isInInput && options.canBeClipped(video)) {
          event.preventDefault();
          options.seekToClipStart();
        }
        break;
      case "e":
      case "E":
        if (!isInInput && options.canBeClipped(video)) {
          event.preventDefault();
          options.seekToClipEnd();
        }
        break;
      case " ":
        if (!isInInput) {
          event.preventDefault();
          togglePlay();
        }
        break;
      case "ArrowLeft":
        if (!isInInput) {
          event.preventDefault();
          const videoElement = options.getVideoElement();
          if (videoElement) {
            options.setPreviewTime(
              Math.max(0, options.getPlaybackCurrentTime() - 5),
            );
          }
        }
        break;
      case "ArrowRight":
        if (!isInInput) {
          event.preventDefault();
          const videoElement = options.getVideoElement();
          if (!videoElement || event.repeat) {
            break;
          }
          void resetRightArrowFastForward(false, "hold-restart");
          options.setRightArrowHoldTimeout(
            setTimeout(() => {
              options.setRightArrowHoldTimeout(null);
              void startRightArrowFastForward();
            }, options.fastForwardHoldDelayMs),
          );
        }
        break;
      case "g":
      case "G":
        if (!isInInput && options.canBeClipped(video)) {
          event.preventDefault();
          void options.generateClip();
        }
        break;
      case "c":
      case "C":
        if (!isInInput && options.canBeClipped(video)) {
          event.preventDefault();
          options.clearClipSelection();
        }
        break;
      case "h":
      case "H":
        if (!isTextEntryHotkeyTarget(event.target) && options.canBeClipped(video)) {
          event.preventDefault();
          options.setShowDetail(!options.getShowDetail());
        }
        break;
    }
  };

  const handleKeyup = (event: KeyboardEvent) => {
    if (event.key !== "ArrowRight") {
      return;
    }

    if (!options.getShow() || !options.getIsVideoLoaded()) {
      void resetRightArrowFastForward(false, "keyup-hidden");
      return;
    }

    if (isBlockedHotkeyTarget(event.target)) {
      void resetRightArrowFastForward(false, "keyup-blocked");
      return;
    }

    event.preventDefault();
    void resetRightArrowFastForward(true, "key-release");
  };

  const handleVideoEnded = () => {
    void resetRightArrowFastForward(false, "video-ended");
    const videoElement = options.getVideoElement();
    if (videoElement) {
      videoElement.playbackRate = options.getSelectedPlaybackRate();
    }
    options.setCurrentPlaybackRateValue(options.getSelectedPlaybackRate());
    options.setIsPlaying(false);
  };

  const handleNativePlaybackEnded = () => {
    void resetRightArrowFastForward(false, "native-ended");
    options.setCurrentPlaybackRateValue(options.getSelectedPlaybackRate());
    options.setIsPlaying(false);
  };

  return {
    handleKeydown,
    handleKeyup,
    handleNativePlaybackEnded,
    handleVideoEnded,
    resetRightArrowFastForward,
    startRateBasedFastForward,
    startRightArrowFastForward,
    togglePlay,
  };
}
