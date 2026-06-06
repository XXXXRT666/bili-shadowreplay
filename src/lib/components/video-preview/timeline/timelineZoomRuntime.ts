import { getEventClientPoint } from "./seekbarInteractionRuntime";

type GestureTargetKind = "timeline" | "zoomControl" | "waveform";

interface ClientPoint {
  clientX: number;
  clientY: number;
}

interface TimelineZoomRuntimeOptions {
  gestureScaleDeltaPerStep: number;
  gestureWheelDeltaPerStep: number;
  gestureCommitDelayMs: number;
  seekbarPbpViewBoxWidth: number;
  getWindowObject: () => Window | null;
  getTimelineElement: () => HTMLElement | null;
  getTimelineContainer: () => HTMLElement | null;
  getTimelineSliderValue: () => number;
  setTimelineSliderValue: (value: number) => void;
  getTimelineScale: () => number;
  setTimelineScale: (value: number) => void;
  getTimelineWidth: () => number;
  setTimelineWidth: (value: number) => void;
  getTimelineZoomSteps: () => number;
  getSeekbarPbpViewBoxX: () => number;
  setSeekbarPbpViewBoxX: (value: number) => void;
  setSeekbarPbpViewBoxWidth: (value: number) => void;
  incrementSeekbarPbpZoomVersion: () => void;
  getWaveformZoomInputThreshold: () => number;
  snapTimelineSliderValue: (value: number) => number;
  getTimelineScaleForSliderValue: (value: number) => number;
  getNearestTimelineZoomNotchIndex: (value: number) => number;
  getTimelineZoomNotchValue: (index: number) => number;
  refreshSeekbarMetrics: () => void;
  updateTimeMarkers: () => void;
  scheduleSeekbarHoverSync: (clientX: number, clientY: number) => void;
  scheduleSeekbarHoverSyncFromLastPoint: () => void;
  syncSeekbarHoverFromClientPoint: (clientX: number, clientY: number) => void;
  rememberSeekbarPointerClientPoint: (clientX: number, clientY: number) => void;
  commitWaveformScale: () => Promise<void>;
}

export function createTimelineZoomRuntime(options: TimelineZoomRuntimeOptions) {
  let timelineMeasureFrame: number | null = null;
  let timelineGestureCommitTimer: number | null = null;
  let timelineGestureActive = false;
  let timelineGestureStartScale = 1;
  let timelineGestureStartNotchIndex = 0;
  let timelineGestureWheelAccumulatedDelta = 0;
  let waveformZoomAccumulatedDelta = 0;
  let lastWaveformZoomSliderValue: number | null = null;
  const gestureTargets: Record<GestureTargetKind, HTMLElement | null> = {
    timeline: null,
    zoomControl: null,
    waveform: null,
  };

  const syncTimelineScrollMetrics = () => {
    const timelineContainer = options.getTimelineContainer();
    if (!timelineContainer) {
      options.setSeekbarPbpViewBoxX(0);
      options.setSeekbarPbpViewBoxWidth(options.seekbarPbpViewBoxWidth);
      return;
    }

    const clientWidth = Math.max(0, timelineContainer.clientWidth);
    const scrollWidth = Math.max(clientWidth, timelineContainer.scrollWidth);
    if (clientWidth <= 0 || scrollWidth <= 0) {
      options.setSeekbarPbpViewBoxX(0);
      options.setSeekbarPbpViewBoxWidth(options.seekbarPbpViewBoxWidth);
      return;
    }

    const maxScrollLeft = Math.max(0, scrollWidth - clientWidth);
    const scrollLeft = Math.max(
      0,
      Math.min(maxScrollLeft, timelineContainer.scrollLeft),
    );
    const visibleRatio = Math.max(
      0.0001,
      Math.min(1, clientWidth / scrollWidth),
    );
    const startRatio = Math.max(
      0,
      Math.min(1 - visibleRatio, scrollLeft / scrollWidth),
    );

    options.setSeekbarPbpViewBoxX(options.seekbarPbpViewBoxWidth * startRatio);
    options.setSeekbarPbpViewBoxWidth(
      Math.max(1, options.seekbarPbpViewBoxWidth * visibleRatio),
    );
  };

  const handleTimelineScroll = () => {
    syncTimelineScrollMetrics();
    options.scheduleSeekbarHoverSyncFromLastPoint();
  };

  const refreshTimelineMeasurements = () => {
    const timelineElement = options.getTimelineElement();
    if (!timelineElement) {
      return;
    }
    const rect = timelineElement.getBoundingClientRect();
    options.setTimelineWidth(rect.width);
    options.refreshSeekbarMetrics();
    syncTimelineScrollMetrics();
    options.updateTimeMarkers();
  };

  const scheduleTimelineRefresh = () => {
    const windowObject = options.getWindowObject();
    if (!windowObject) {
      return;
    }
    if (timelineMeasureFrame !== null) {
      windowObject.cancelAnimationFrame(timelineMeasureFrame);
    }
    timelineMeasureFrame = windowObject.requestAnimationFrame(() => {
      timelineMeasureFrame = null;
      refreshTimelineMeasurements();
    });
  };

  const resetWaveformZoomInputTracking = (sliderValue: number | null = null) => {
    waveformZoomAccumulatedDelta = 0;
    lastWaveformZoomSliderValue = sliderValue;
  };

  const clearTimelineGestureCommitTimer = () => {
    const windowObject = options.getWindowObject();
    if (timelineGestureCommitTimer === null || !windowObject) {
      return;
    }
    windowObject.clearTimeout(timelineGestureCommitTimer);
    timelineGestureCommitTimer = null;
  };

  const handleScaleCommit = () => {
    const sliderValue = options.snapTimelineSliderValue(
      options.getTimelineSliderValue(),
    );
    options.setTimelineSliderValue(sliderValue);
    options.setTimelineScale(options.getTimelineScaleForSliderValue(sliderValue));
    options.incrementSeekbarPbpZoomVersion();
    scheduleTimelineRefresh();
    options.updateTimeMarkers();
    resetWaveformZoomInputTracking(sliderValue);
    void options.commitWaveformScale();
  };

  const scheduleTimelineGestureCommit = () => {
    const windowObject = options.getWindowObject();
    if (!windowObject) {
      return;
    }
    clearTimelineGestureCommitTimer();
    timelineGestureCommitTimer = windowObject.setTimeout(() => {
      timelineGestureCommitTimer = null;
      handleScaleCommit();
    }, options.gestureCommitDelayMs);
  };

  const handleScaleChange = () => {
    const sliderValue = options.snapTimelineSliderValue(
      options.getTimelineSliderValue(),
    );
    options.setTimelineSliderValue(sliderValue);
    options.setTimelineScale(options.getTimelineScaleForSliderValue(sliderValue));
    options.incrementSeekbarPbpZoomVersion();
    options.updateTimeMarkers();
    scheduleTimelineRefresh();

    if (lastWaveformZoomSliderValue === null) {
      lastWaveformZoomSliderValue = sliderValue;
      return;
    }

    waveformZoomAccumulatedDelta += sliderValue - lastWaveformZoomSliderValue;
    lastWaveformZoomSliderValue = sliderValue;

    if (
      Math.abs(waveformZoomAccumulatedDelta) >=
      options.getWaveformZoomInputThreshold()
    ) {
      waveformZoomAccumulatedDelta = 0;
      void options.commitWaveformScale();
    }
  };

  const applyTimelineZoomGestureTargetIndex = (
    targetIndex: number,
    clientPoint: ClientPoint | null = null,
  ) => {
    const currentIndex = options.getNearestTimelineZoomNotchIndex(
      options.getTimelineSliderValue(),
    );
    const nextValue = options.getTimelineZoomNotchValue(targetIndex);
    const nextIndex = options.getNearestTimelineZoomNotchIndex(nextValue);
    if (nextIndex === currentIndex) {
      return;
    }
    options.setTimelineSliderValue(nextValue);
    handleScaleChange();
    if (clientPoint) {
      options.scheduleSeekbarHoverSync(clientPoint.clientX, clientPoint.clientY);
    } else {
      options.scheduleSeekbarHoverSyncFromLastPoint();
    }
    scheduleTimelineGestureCommit();
  };

  const getGestureScale = (event: Event) => {
    const value = (event as Event & { scale?: number }).scale;
    return typeof value === "number" && Number.isFinite(value) && value > 0
      ? value
      : 1;
  };

  const handleTimelineGestureStart = (event: Event) => {
    event.preventDefault();
    const clientPoint = getEventClientPoint(event);
    if (clientPoint) {
      options.rememberSeekbarPointerClientPoint(
        clientPoint.clientX,
        clientPoint.clientY,
      );
    }
    timelineGestureActive = true;
    timelineGestureStartScale = getGestureScale(event);
    timelineGestureStartNotchIndex = options.getNearestTimelineZoomNotchIndex(
      options.getTimelineSliderValue(),
    );
    clearTimelineGestureCommitTimer();
  };

  const handleTimelineGestureChange = (event: Event) => {
    event.preventDefault();
    if (!timelineGestureActive) {
      const fallbackPoint = getEventClientPoint(event);
      if (fallbackPoint) {
        options.rememberSeekbarPointerClientPoint(
          fallbackPoint.clientX,
          fallbackPoint.clientY,
        );
      }
      timelineGestureActive = true;
      timelineGestureStartScale = getGestureScale(event);
      timelineGestureStartNotchIndex = options.getNearestTimelineZoomNotchIndex(
        options.getTimelineSliderValue(),
      );
      return;
    }
    const clientPoint = getEventClientPoint(event);
    if (clientPoint) {
      options.rememberSeekbarPointerClientPoint(
        clientPoint.clientX,
        clientPoint.clientY,
      );
    }
    const currentGestureScale = getGestureScale(event);
    const scaleDelta = currentGestureScale - timelineGestureStartScale;
    if (!Number.isFinite(scaleDelta) || Math.abs(scaleDelta) < 0.0001) {
      return;
    }
    const stepOffset = Math.round(
      scaleDelta / options.gestureScaleDeltaPerStep,
    );
    applyTimelineZoomGestureTargetIndex(
      timelineGestureStartNotchIndex + stepOffset,
      clientPoint,
    );
  };

  const handleTimelineGestureEnd = (event: Event) => {
    event.preventDefault();
    const clientPoint = getEventClientPoint(event);
    if (clientPoint) {
      options.scheduleSeekbarHoverSync(clientPoint.clientX, clientPoint.clientY);
    } else {
      options.scheduleSeekbarHoverSyncFromLastPoint();
    }
    timelineGestureActive = false;
    timelineGestureStartScale = 1;
    timelineGestureStartNotchIndex = options.getNearestTimelineZoomNotchIndex(
      options.getTimelineSliderValue(),
    );
    clearTimelineGestureCommitTimer();
    handleScaleCommit();
  };

  const addTimelineGestureListeners = (target: HTMLElement) => {
    target.addEventListener(
      "gesturestart",
      handleTimelineGestureStart as EventListener,
      { passive: false },
    );
    target.addEventListener(
      "gesturechange",
      handleTimelineGestureChange as EventListener,
      { passive: false },
    );
    target.addEventListener("gestureend", handleTimelineGestureEnd as EventListener, {
      passive: false,
    });
  };

  const removeTimelineGestureListeners = (target: HTMLElement) => {
    target.removeEventListener(
      "gesturestart",
      handleTimelineGestureStart as EventListener,
    );
    target.removeEventListener(
      "gesturechange",
      handleTimelineGestureChange as EventListener,
    );
    target.removeEventListener(
      "gestureend",
      handleTimelineGestureEnd as EventListener,
    );
  };

  const syncGestureTarget = (
    kind: GestureTargetKind,
    nextTarget: HTMLElement | null,
  ) => {
    const currentTarget = gestureTargets[kind];
    if (currentTarget === nextTarget) {
      return;
    }
    if (currentTarget) {
      removeTimelineGestureListeners(currentTarget);
    }
    gestureTargets[kind] = nextTarget;
    if (nextTarget) {
      addTimelineGestureListeners(nextTarget);
    }
  };

  const applyTimelineZoomFromWheelDelta = (
    delta: number,
    clientX: number,
    clientY: number,
  ) => {
    if (!Number.isFinite(delta) || Math.abs(delta) < 0.01) {
      return;
    }
    options.rememberSeekbarPointerClientPoint(clientX, clientY);
    const deltaSign = Math.sign(delta);
    const accumulatorSign = Math.sign(timelineGestureWheelAccumulatedDelta);
    if (
      deltaSign !== 0 &&
      accumulatorSign !== 0 &&
      deltaSign !== accumulatorSign
    ) {
      timelineGestureWheelAccumulatedDelta = 0;
    }
    timelineGestureWheelAccumulatedDelta += delta;

    const stepOffset = -Math.trunc(
      timelineGestureWheelAccumulatedDelta / options.gestureWheelDeltaPerStep,
    );
    if (stepOffset === 0) {
      return;
    }

    timelineGestureWheelAccumulatedDelta +=
      stepOffset * options.gestureWheelDeltaPerStep;
    const currentIndex = options.getNearestTimelineZoomNotchIndex(
      options.getTimelineSliderValue(),
    );
    applyTimelineZoomGestureTargetIndex(currentIndex + stepOffset, {
      clientX,
      clientY,
    });
  };

  const handleTimelineZoomControlWheel = (event: WheelEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const dominantDelta =
      Math.abs(event.deltaY) >= Math.abs(event.deltaX)
        ? event.deltaY
        : event.deltaX;
    applyTimelineZoomFromWheelDelta(dominantDelta, event.clientX, event.clientY);
  };

  const handleWheel = (event: WheelEvent) => {
    event.preventDefault();
    const timelineContainer = options.getTimelineContainer();
    if (!timelineContainer) {
      return;
    }
    if (event.ctrlKey) {
      applyTimelineZoomFromWheelDelta(event.deltaY, event.clientX, event.clientY);
      return;
    }
    timelineGestureWheelAccumulatedDelta = 0;

    let didScroll = false;
    const horizontalDelta = event.deltaX;
    if (Math.abs(horizontalDelta) > 0.01) {
      timelineContainer.scrollLeft += horizontalDelta;
      didScroll = true;
    }

    const absDeltaY = Math.abs(event.deltaY);
    const wheelDeltaY = Math.abs(
      Number((event as WheelEvent & { wheelDeltaY?: number }).wheelDeltaY ?? 0),
    );
    const wheelDelta = Math.abs(
      Number((event as WheelEvent & { wheelDelta?: number }).wheelDelta ?? 0),
    );
    const sourceCapabilities = (
      event as WheelEvent & {
        sourceCapabilities?: {
          firesTouchEvents?: boolean;
        };
      }
    ).sourceCapabilities;
    const isSourceMouse = sourceCapabilities?.firesTouchEvents === false;
    const isSourceTouchLike = sourceCapabilities?.firesTouchEvents === true;
    const isDiscreteDeltaY = Number.isInteger(event.deltaY) && absDeltaY >= 1;
    const isMouseWheelEvent =
      isSourceMouse ||
      (!isSourceTouchLike &&
        (event.deltaMode !== WheelEvent.DOM_DELTA_PIXEL ||
          wheelDeltaY >= 1 ||
          wheelDelta >= 1 ||
          isDiscreteDeltaY ||
          absDeltaY >= 24));
    if (isMouseWheelEvent && absDeltaY > 0.01) {
      timelineContainer.scrollLeft += event.deltaY;
      didScroll = true;
    }
    if (didScroll) {
      syncTimelineScrollMetrics();
      options.syncSeekbarHoverFromClientPoint(event.clientX, event.clientY);
    }
  };

  const cleanup = () => {
    const windowObject = options.getWindowObject();
    if (timelineMeasureFrame !== null && windowObject) {
      windowObject.cancelAnimationFrame(timelineMeasureFrame);
      timelineMeasureFrame = null;
    }
    clearTimelineGestureCommitTimer();
    for (const kind of Object.keys(gestureTargets) as GestureTargetKind[]) {
      const target = gestureTargets[kind];
      if (target) {
        removeTimelineGestureListeners(target);
        gestureTargets[kind] = null;
      }
    }
  };

  return {
    applyTimelineZoomFromWheelDelta,
    cleanup,
    handleScaleChange,
    handleScaleCommit,
    handleTimelineScroll,
    handleTimelineZoomControlWheel,
    handleWheel,
    refreshTimelineMeasurements,
    resetWaveformZoomInputTracking,
    scheduleTimelineRefresh,
    syncGestureTarget,
    syncTimelineScrollMetrics,
  };
}
