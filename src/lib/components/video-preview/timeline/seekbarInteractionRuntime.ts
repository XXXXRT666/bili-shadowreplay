interface SeekbarEventPoint {
  clientX: number;
  clientY: number;
}

interface SeekbarInteractionRuntimeOptions {
  hoverHitTopExtend: number;
  hoverHitBottomExtend: number;
  getSeekbarElement: () => HTMLElement | null;
  getSeekbarProgressElement: () => HTMLElement | null;
  getTimelineWidth: () => number;
  getWindowObject: () => Window | null;
  getIsDraggingSeekbar: () => boolean;
  setIsDraggingSeekbar: (value: boolean) => void;
  getIsPlaying: () => boolean;
  setIsPlaying: (value: boolean) => void;
  getWasPlayingBeforeDrag: () => boolean;
  setWasPlayingBeforeDrag: (value: boolean) => void;
  getPreviewTimeValue: () => number;
  setPreviewTimeValue: (value: number) => void;
  setIsSeekbarHovering: (value: boolean) => void;
  setIsSeekbarTrackHovering: (value: boolean) => void;
  setSeekbarHoverTime: (value: number) => void;
  setSeekbarMetricsWidth: (value: number) => void;
  setSeekbarViewportLeft: (value: number) => void;
  setSeekbarViewportTop: (value: number) => void;
  setSeekbarViewportHeight: (value: number) => void;
  getSeekbarPointerClientX: () => number | null;
  getSeekbarPointerClientY: () => number | null;
  setSeekbarPointerClientPoint: (clientX: number, clientY: number) => void;
  getSeekbarHoverSyncFrame: () => number | null;
  setSeekbarHoverSyncFrame: (value: number | null) => void;
  getSeekbarDuration: () => number;
  getPlaybackCurrentTime: () => number;
  normalizeSeekbarPreviewTime: (time: number, durationOverride?: number) => number;
  resetThumbnails: (resetCache: boolean) => void;
  pausePlayback: () => void;
  playPlayback: () => void;
  commitPreviewTime: (time: number) => void;
  resetFastForward: (reason: string) => Promise<void>;
}

export function getEventClientPoint(event: Event): SeekbarEventPoint | null {
  const maybePointerEvent = event as Event & {
    clientX?: number;
    clientY?: number;
    pageX?: number;
    pageY?: number;
  };
  if (
    Number.isFinite(maybePointerEvent.clientX) &&
    Number.isFinite(maybePointerEvent.clientY)
  ) {
    return {
      clientX: Number(maybePointerEvent.clientX),
      clientY: Number(maybePointerEvent.clientY),
    };
  }
  if (
    typeof window !== "undefined" &&
    Number.isFinite(maybePointerEvent.pageX) &&
    Number.isFinite(maybePointerEvent.pageY)
  ) {
    return {
      clientX: Number(maybePointerEvent.pageX) - window.scrollX,
      clientY: Number(maybePointerEvent.pageY) - window.scrollY,
    };
  }
  return null;
}

export function createSeekbarInteractionRuntime(
  options: SeekbarInteractionRuntimeOptions
) {
  const refreshSeekbarViewportAnchor = () => {
    const anchorRect = options.getSeekbarElement()?.getBoundingClientRect();
    if (!anchorRect) {
      return;
    }
    options.setSeekbarViewportLeft(anchorRect.left);
    options.setSeekbarViewportTop(anchorRect.top);
    options.setSeekbarViewportHeight(Math.max(1, anchorRect.height));
  };

  const refreshSeekbarMetrics = () => {
    const seekbarElement = options.getSeekbarElement();
    if (!seekbarElement) {
      options.setSeekbarMetricsWidth(Math.max(0, options.getTimelineWidth()));
      return;
    }
    const rect = seekbarElement.getBoundingClientRect();
    options.setSeekbarMetricsWidth(Math.max(0, rect.width));
    refreshSeekbarViewportAnchor();
  };

  const isPointerInsideSeekbar = (clientX: number, clientY: number) => {
    const seekbarElement = options.getSeekbarElement();
    if (!seekbarElement) {
      return false;
    }
    const rect = seekbarElement.getBoundingClientRect();
    return (
      clientX >= rect.left &&
      clientX <= rect.right &&
      clientY >= rect.top - options.hoverHitTopExtend &&
      clientY <= rect.bottom + options.hoverHitBottomExtend
    );
  };

  const isPointerInsideSeekbarTrack = (clientX: number, clientY: number) => {
    const seekbarProgressElement = options.getSeekbarProgressElement();
    if (!seekbarProgressElement) {
      return false;
    }
    const rect = seekbarProgressElement.getBoundingClientRect();
    return (
      clientX >= rect.left &&
      clientX <= rect.right &&
      clientY >= rect.top &&
      clientY <= rect.bottom
    );
  };

  const updateSeekbarTimeFromClientX = (
    clientX: number,
    applyToDragPreview = false
  ) => {
    const seekbarElement = options.getSeekbarElement();
    if (!seekbarElement) {
      return;
    }
    const rect = seekbarElement.getBoundingClientRect();
    if (rect.width <= 0) {
      return;
    }
    const clampedX = Math.max(0, Math.min(clientX - rect.left, rect.width));
    const duration = options.getSeekbarDuration();
    const mappedTime =
      duration > 0
        ? options.normalizeSeekbarPreviewTime(
            (clampedX / rect.width) * duration,
            duration
          )
        : 0;
    options.setSeekbarHoverTime(mappedTime);
    if (applyToDragPreview || options.getIsDraggingSeekbar()) {
      options.setPreviewTimeValue(mappedTime);
    }
    options.setSeekbarMetricsWidth(rect.width);
    refreshSeekbarViewportAnchor();
  };

  const syncSeekbarHoverFromClientPoint = (clientX: number, clientY: number) => {
    if (!options.getSeekbarElement()) {
      return;
    }
    options.setSeekbarPointerClientPoint(clientX, clientY);
    const hovering = isPointerInsideSeekbar(clientX, clientY);
    const trackHovering = isPointerInsideSeekbarTrack(clientX, clientY);
    if (!options.getIsDraggingSeekbar()) {
      options.setIsSeekbarHovering(hovering);
      options.setIsSeekbarTrackHovering(trackHovering);
    }
    if (hovering || options.getIsDraggingSeekbar()) {
      updateSeekbarTimeFromClientX(clientX, options.getIsDraggingSeekbar());
    }
  };

  const clearSeekbarHoverSyncFrame = () => {
    const frame = options.getSeekbarHoverSyncFrame();
    const windowObject = options.getWindowObject();
    if (frame === null || !windowObject) {
      return;
    }
    windowObject.cancelAnimationFrame(frame);
    options.setSeekbarHoverSyncFrame(null);
  };

  const scheduleSeekbarHoverSync = (clientX: number, clientY: number) => {
    const windowObject = options.getWindowObject();
    if (!windowObject) {
      return;
    }
    options.setSeekbarPointerClientPoint(clientX, clientY);
    clearSeekbarHoverSyncFrame();
    const frame = windowObject.requestAnimationFrame(() => {
      options.setSeekbarHoverSyncFrame(null);
      syncSeekbarHoverFromClientPoint(clientX, clientY);
    });
    options.setSeekbarHoverSyncFrame(frame);
  };

  const scheduleSeekbarHoverSyncFromLastPoint = () => {
    const clientX = options.getSeekbarPointerClientX();
    const clientY = options.getSeekbarPointerClientY();
    if (clientX === null || clientY === null) {
      return;
    }
    scheduleSeekbarHoverSync(clientX, clientY);
  };

  const handleSeekbarMouseMove = (event: MouseEvent) => {
    if (!options.getIsDraggingSeekbar()) {
      return;
    }
    const seekbarElement = options.getSeekbarElement();
    if (!seekbarElement) {
      return;
    }
    options.setSeekbarPointerClientPoint(event.clientX, event.clientY);
    options.setIsSeekbarHovering(isPointerInsideSeekbar(event.clientX, event.clientY));
    options.setIsSeekbarTrackHovering(
      isPointerInsideSeekbarTrack(event.clientX, event.clientY)
    );
    updateSeekbarTimeFromClientX(event.clientX, true);
  };

  const handleSeekbarMouseUp = (event: MouseEvent) => {
    if (!options.getIsDraggingSeekbar()) {
      return;
    }
    options.setSeekbarPointerClientPoint(event.clientX, event.clientY);

    const duration = options.getSeekbarDuration();
    if (duration > 0) {
      options.commitPreviewTime(
        options.normalizeSeekbarPreviewTime(
          options.getPreviewTimeValue(),
          duration
        )
      );
    }

    options.setIsDraggingSeekbar(false);
    const hovering = isPointerInsideSeekbar(event.clientX, event.clientY);
    options.setIsSeekbarHovering(hovering);
    options.setIsSeekbarTrackHovering(
      isPointerInsideSeekbarTrack(event.clientX, event.clientY)
    );
    if (hovering) {
      updateSeekbarTimeFromClientX(event.clientX, false);
    }

    document.removeEventListener("mousemove", handleSeekbarMouseMove);
    document.removeEventListener("mouseup", handleSeekbarMouseUp);

    if (options.getWasPlayingBeforeDrag()) {
      options.playPlayback();
      options.setIsPlaying(true);
    }
  };

  const handleSeekbarMouseDown = async (event: MouseEvent) => {
    await options.resetFastForward("seekbar-drag");
    event.preventDefault();
    event.stopPropagation();

    const seekbarElement = options.getSeekbarElement();
    if (!seekbarElement || options.getSeekbarDuration() < 0) {
      return;
    }

    options.setSeekbarPointerClientPoint(event.clientX, event.clientY);
    options.setIsDraggingSeekbar(true);
    options.setIsSeekbarHovering(true);
    options.setIsSeekbarTrackHovering(
      isPointerInsideSeekbarTrack(event.clientX, event.clientY)
    );
    options.setWasPlayingBeforeDrag(options.getIsPlaying());
    options.setPreviewTimeValue(options.getPlaybackCurrentTime());
    updateSeekbarTimeFromClientX(event.clientX, true);

    if (options.getIsPlaying()) {
      options.pausePlayback();
      options.setIsPlaying(false);
    }

    document.addEventListener("mousemove", handleSeekbarMouseMove);
    document.addEventListener("mouseup", handleSeekbarMouseUp);
  };

  const handleSeekbarMouseEnter = (event: MouseEvent) => {
    if (options.getIsDraggingSeekbar()) {
      return;
    }
    options.setSeekbarPointerClientPoint(event.clientX, event.clientY);
    options.setIsSeekbarHovering(isPointerInsideSeekbar(event.clientX, event.clientY));
    options.setIsSeekbarTrackHovering(
      isPointerInsideSeekbarTrack(event.clientX, event.clientY)
    );
    updateSeekbarTimeFromClientX(event.clientX, false);
  };

  const handleSeekbarMouseHoverMove = (event: MouseEvent) => {
    if (options.getIsDraggingSeekbar()) {
      return;
    }
    options.setSeekbarPointerClientPoint(event.clientX, event.clientY);
    options.setIsSeekbarHovering(isPointerInsideSeekbar(event.clientX, event.clientY));
    options.setIsSeekbarTrackHovering(
      isPointerInsideSeekbarTrack(event.clientX, event.clientY)
    );
    updateSeekbarTimeFromClientX(event.clientX, false);
  };

  const handleSeekbarMouseLeave = () => {
    if (options.getIsDraggingSeekbar()) {
      return;
    }
    options.setIsSeekbarHovering(false);
    options.setIsSeekbarTrackHovering(false);
  };

  const resetSeekbarPreviewState = (resetCache = false) => {
    options.setIsSeekbarHovering(false);
    options.setIsSeekbarTrackHovering(false);
    options.setSeekbarHoverTime(options.getPlaybackCurrentTime());
    options.resetThumbnails(resetCache);
  };

  const cleanup = () => {
    clearSeekbarHoverSyncFrame();
    document.removeEventListener("mousemove", handleSeekbarMouseMove);
    document.removeEventListener("mouseup", handleSeekbarMouseUp);
  };

  return {
    cleanup,
    clearSeekbarHoverSyncFrame,
    getEventClientPoint,
    handleSeekbarMouseUp,
    handleSeekbarMouseDown,
    handleSeekbarMouseEnter,
    handleSeekbarMouseHoverMove,
    handleSeekbarMouseLeave,
    refreshSeekbarMetrics,
    refreshSeekbarViewportAnchor,
    rememberSeekbarPointerClientPoint: options.setSeekbarPointerClientPoint,
    resetSeekbarPreviewState,
    scheduleSeekbarHoverSync,
    scheduleSeekbarHoverSyncFromLastPoint,
    syncSeekbarHoverFromClientPoint,
  };
}
