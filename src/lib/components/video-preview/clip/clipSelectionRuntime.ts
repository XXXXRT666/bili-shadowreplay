import type { DanmuRenderOptions, VideoItem } from "../../../interface";

export interface ClipSelection {
  id: string;
  startTime: number;
  endTime: number;
  color: string;
}

export interface ClipSelectionState {
  clipSelections: ClipSelection[];
  activeClipSelectionId: string | null;
  clipStartTime: number;
  clipEndTime: number;
  clipTimesSet: boolean;
  hasPendingClipStartMarker: boolean;
  pendingClipStartTime: number;
  clipRegionColor: string;
}

export function createClipSelectionId(options: {
  prefix: string;
  counter: number;
}) {
  const nextCounter = options.counter + 1;
  return {
    nextCounter,
    id: `${options.prefix}${nextCounter}`,
  };
}

function getActiveClipSelection(options: {
  clipSelections: ClipSelection[];
  activeClipSelectionId: string | null;
}) {
  if (!options.activeClipSelectionId) {
    return null;
  }

  return (
    options.clipSelections.find(
      (selection) => selection.id === options.activeClipSelectionId
    ) ?? null
  );
}

function syncClipStateFromActiveSelection(
  options: ClipSelectionState
): ClipSelectionState {
  const activeSelection = getActiveClipSelection(options);

  if (!activeSelection) {
    return {
      ...options,
      clipStartTime: 0,
      clipEndTime: 0,
      clipTimesSet: false,
    };
  }

  return {
    ...options,
    clipStartTime: activeSelection.startTime,
    clipEndTime: activeSelection.endTime,
    clipTimesSet: activeSelection.endTime > activeSelection.startTime,
    clipRegionColor: activeSelection.color,
  };
}

export function applyClipRegionLabel(region: any, isActive = false) {
  const element = region?.element instanceof HTMLElement ? region.element : null;
  const label =
    region?.content instanceof HTMLElement ? (region.content as HTMLElement) : null;

  if (element) {
    element.style.boxShadow = isActive
      ? "inset 0 0 0 2px rgba(255, 255, 255, 0.6)"
      : "inset 0 0 0 1px rgba(255, 255, 255, 0.22)";
    element.style.zIndex = isActive ? "7" : "6";
  }

  if (!label) {
    return;
  }

  label.textContent = "切片";
  label.style.fontSize = "11px";
  label.style.fontWeight = "600";
  label.style.lineHeight = "1.2";
  label.style.color = "rgba(255, 255, 255, 0.95)";
  label.style.backgroundColor = isActive
    ? "rgba(10, 132, 255, 0.88)"
    : "rgba(10, 132, 255, 0.62)";
  label.style.borderRadius = "999px";
  label.style.padding = "2px 6px";
  label.style.margin = "4px";
  label.style.pointerEvents = "none";
  label.style.whiteSpace = "nowrap";
}

export function removeWaveformRegion(region: any) {
  if (region && typeof region.remove === "function") {
    region.remove();
  }
  return null;
}

export function updateClipSelectionFromRegion(options: {
  state: ClipSelectionState;
  region: any;
}) {
  if (!options.region?.id) {
    return options.state;
  }

  const selectionIndex = options.state.clipSelections.findIndex(
    (selection) => selection.id === options.region.id
  );

  if (selectionIndex < 0) {
    return options.state;
  }

  const nextStart = Math.max(0, Math.min(options.region.start, options.region.end));
  const nextEnd = Math.max(nextStart, Math.max(options.region.start, options.region.end));
  const nextClipSelections = options.state.clipSelections.map((selection, index) =>
    index === selectionIndex
      ? {
          ...selection,
          startTime: nextStart,
          endTime: nextEnd,
          color: options.region.color || selection.color,
        }
      : selection
  );

  if (options.state.hasPendingClipStartMarker) {
    return {
      ...options.state,
      clipSelections: nextClipSelections,
    };
  }

  return {
    ...options.state,
    clipSelections: nextClipSelections,
    activeClipSelectionId: options.region.id,
    clipStartTime: nextStart,
    clipEndTime: nextEnd,
    clipTimesSet: nextEnd > nextStart,
    clipRegionColor:
      options.region.color || nextClipSelections[selectionIndex]?.color || "",
  };
}

export function setActiveClipSelection(options: {
  state: ClipSelectionState;
  id: string | null;
}) {
  let nextId = options.id;
  if (
    nextId &&
    !options.state.clipSelections.some((selection) => selection.id === nextId)
  ) {
    nextId = null;
  }

  return syncClipStateFromActiveSelection({
    ...options.state,
    activeClipSelectionId: nextId,
  });
}

function addClipSelection(options: {
  state: ClipSelectionState;
  startTime: number;
  endTime: number;
  nextId: string;
  color: string;
}) {
  const nextStart = Math.min(options.startTime, options.endTime);
  const nextEnd = Math.max(options.startTime, options.endTime);

  if (nextEnd <= nextStart) {
    return {
      state: options.state,
      selection: null,
    };
  }

  const selection: ClipSelection = {
    id: options.nextId,
    startTime: nextStart,
    endTime: nextEnd,
    color: options.state.clipRegionColor || options.color,
  };

  const nextState = setActiveClipSelection({
    state: {
      ...options.state,
      clipSelections: [...options.state.clipSelections, selection],
    },
    id: selection.id,
  });

  return {
    state: nextState,
    selection,
  };
}

function removeClipSelectionById(options: {
  state: ClipSelectionState;
  id: string;
}) {
  const exists = options.state.clipSelections.some(
    (selection) => selection.id === options.id
  );
  if (!exists) {
    return options.state;
  }

  const nextClipSelections = options.state.clipSelections.filter(
    (selection) => selection.id !== options.id
  );
  const nextActiveId =
    options.state.activeClipSelectionId === options.id
      ? nextClipSelections[nextClipSelections.length - 1]?.id ?? null
      : options.state.activeClipSelectionId;

  return setActiveClipSelection({
    state: {
      ...options.state,
      clipSelections: nextClipSelections,
    },
    id: nextActiveId,
  });
}

export function clearAllClipSelections(options: ClipSelectionState) {
  return {
    ...options,
    clipSelections: [],
    activeClipSelectionId: null,
    clipStartTime: 0,
    clipEndTime: 0,
    clipTimesSet: false,
    hasPendingClipStartMarker: false,
    pendingClipStartTime: 0,
    clipRegionColor: "",
  } satisfies ClipSelectionState;
}

export function syncClipWaveformRegions(options: {
  waveformRegions: any;
  isWaveformLoaded: boolean;
  state: ClipSelectionState;
  clipStartMarkerRegion: any;
  clipSelectionRegions: Record<string, any>;
  clipStartMarkerRegionId: string;
}) {
  if (!options.waveformRegions || !options.isWaveformLoaded) {
    return {
      clipStartMarkerRegion: options.clipStartMarkerRegion,
      clipSelectionRegions: options.clipSelectionRegions,
    };
  }

  let nextClipStartMarkerRegion = options.clipStartMarkerRegion;
  const nextClipSelectionRegions = { ...options.clipSelectionRegions };

  if (options.state.hasPendingClipStartMarker) {
    if (nextClipStartMarkerRegion) {
      nextClipStartMarkerRegion.setOptions({
        start: options.state.pendingClipStartTime,
        end: options.state.pendingClipStartTime,
        color: options.state.clipRegionColor,
      });
    } else {
      nextClipStartMarkerRegion = options.waveformRegions.addRegion({
        id: options.clipStartMarkerRegionId,
        start: options.state.pendingClipStartTime,
        color: options.state.clipRegionColor,
        drag: false,
        resize: false,
      });
    }

    return {
      clipStartMarkerRegion: nextClipStartMarkerRegion,
      clipSelectionRegions: nextClipSelectionRegions,
    };
  }

  nextClipStartMarkerRegion = removeWaveformRegion(nextClipStartMarkerRegion);

  const nextSelectionIds = new Set(
    options.state.clipSelections.map((selection) => selection.id)
  );

  Object.keys(nextClipSelectionRegions).forEach((id) => {
    if (nextSelectionIds.has(id)) {
      return;
    }

    removeWaveformRegion(nextClipSelectionRegions[id]);
    delete nextClipSelectionRegions[id];
  });

  options.state.clipSelections.forEach((selection) => {
    const existingRegion = nextClipSelectionRegions[selection.id];

    if (existingRegion) {
      existingRegion.setOptions({
        start: selection.startTime,
        end: selection.endTime,
        color: selection.color,
        drag: true,
        resize: true,
      });
      applyClipRegionLabel(
        existingRegion,
        selection.id === options.state.activeClipSelectionId
      );
      return;
    }

    nextClipSelectionRegions[selection.id] = options.waveformRegions.addRegion({
      id: selection.id,
      start: selection.startTime,
      end: selection.endTime,
      color: selection.color,
      content: "切片",
      drag: true,
      resize: true,
    });

    applyClipRegionLabel(
      nextClipSelectionRegions[selection.id],
      selection.id === options.state.activeClipSelectionId
    );
  });

  options.state.clipSelections.forEach((selection) => {
    const region = nextClipSelectionRegions[selection.id];
    if (region) {
      applyClipRegionLabel(
        region,
        selection.id === options.state.activeClipSelectionId
      );
    }
  });

  return {
    clipStartMarkerRegion: nextClipStartMarkerRegion,
    clipSelectionRegions: nextClipSelectionRegions,
  };
}

export function setClipStartTime(options: {
  state: ClipSelectionState;
  currentTime: number;
  color: string;
}) {
  return {
    ...options.state,
    pendingClipStartTime: options.currentTime,
    clipStartTime: options.currentTime,
    clipEndTime: options.currentTime,
    clipTimesSet: false,
    hasPendingClipStartMarker: true,
    clipRegionColor: options.color,
  } satisfies ClipSelectionState;
}

export function setClipEndTime(options: {
  state: ClipSelectionState;
  currentTime: number;
  nextId: string;
  color: string;
}) {
  if (!options.state.hasPendingClipStartMarker) {
    return {
      state: options.state,
      selection: null,
    };
  }

  const anchorStart = options.state.pendingClipStartTime;
  const added = addClipSelection({
    state: options.state,
    startTime: anchorStart,
    endTime: options.currentTime,
    nextId: options.nextId,
    color: options.color,
  });

  if (!added.selection) {
    return {
      state: {
        ...options.state,
        hasPendingClipStartMarker: false,
        pendingClipStartTime: 0,
        clipRegionColor: "",
        clipStartTime: anchorStart,
        clipEndTime: anchorStart,
        clipTimesSet: false,
      } satisfies ClipSelectionState,
      selection: null,
    };
  }

  return {
    state: {
      ...added.state,
      hasPendingClipStartMarker: false,
      pendingClipStartTime: 0,
    } satisfies ClipSelectionState,
    selection: added.selection,
  };
}

export function clearClipSelectionState(options: { state: ClipSelectionState }) {
  if (options.state.hasPendingClipStartMarker) {
    return syncClipStateFromActiveSelection({
      ...options.state,
      hasPendingClipStartMarker: false,
      pendingClipStartTime: 0,
      clipRegionColor: "",
    });
  }

  if (options.state.activeClipSelectionId) {
    return removeClipSelectionById({
      state: options.state,
      id: options.state.activeClipSelectionId,
    });
  }

  return syncClipStateFromActiveSelection(options.state);
}

export function buildDefaultClipTitle(now = new Date()) {
  const pad = (value: number) => value.toString().padStart(2, "0");
  const timestamp = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(
    now.getDate()
  )}_${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `clip_${timestamp}`;
}

export function canBeClipped(video: VideoItem | null | undefined): boolean {
  if (!video) {
    return false;
  }

  return video.status !== -1;
}

export async function generateClip(options: {
  video: VideoItem | null | undefined;
  clipSelections: ClipSelection[];
  clipExportSelectionIds: string[];
  mergeClipSelectionsOnExport: boolean;
  clipTitle: string;
  includeSubtitle: boolean;
  includeDanmu: boolean;
  renderDanmuEmotes: boolean;
  danmuRenderOptions: DanmuRenderOptions;
  srtStyle: string;
  generateEventId: () => string;
  listen: (
    event: string,
    handler: (payload: any) => void
  ) => Promise<() => void>;
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
  onPrompt: (text: string) => void;
  onClipTitleResolved: (value: string) => void;
  onClippingChange: (value: boolean) => void;
  onCurrentEventIdChange: (value: string | null) => void;
  onSuccess: () => void;
  reportError: (message: string) => void;
}) {
  if (!options.video) {
    return;
  }

  const selectedIds = new Set(options.clipExportSelectionIds);
  const selectedSelections = options.clipSelections
    .filter((selection) => selectedIds.has(selection.id))
    .filter((selection) => selection.endTime > selection.startTime)
    .sort((a, b) => a.startTime - b.startTime);

  if (selectedSelections.length === 0) {
    options.reportError("请至少勾选一个有效选区");
    return;
  }

  let nextClipTitle = options.clipTitle.trim();
  if (!nextClipTitle) {
    nextClipTitle = buildDefaultClipTitle();
    options.onClipTitleResolved(nextClipTitle);
  }

  if (
    selectedSelections.some(
      (selection) => selection.endTime - selection.startTime < 1
    )
  ) {
    options.reportError("每个导出选区长度都不能少于1秒");
    return;
  }

  options.onClippingChange(true);
  const eventId = options.generateEventId();
  options.onCurrentEventIdChange(eventId);
  let taskSettled = false;
  let clearUpdateListener: (() => void) | undefined;
  let clearFinishedListener: (() => void) | undefined;

  function finishTask() {
    options.onPrompt("生成切片");
    options.onClippingChange(false);
    options.onCurrentEventIdChange(null);
    clearUpdateListener?.();
    clearFinishedListener?.();
  }

  try {
    clearUpdateListener = await options.listen(
      `progress-update:${eventId}`,
      (e) => {
        options.onPrompt(e.payload.content);
      }
    );
    clearFinishedListener = await options.listen(
      `progress-finished:${eventId}`,
      (e) => {
        if (taskSettled) {
          return;
        }
        taskSettled = true;
        if (e.payload.success) {
          options.onSuccess();
        } else {
          options.reportError(`切片生成失败: ${e.payload.message}`);
        }
        finishTask();
      }
    );

    await options.invoke("clip_video", {
      eventId,
      parentVideoId: options.video.id,
      ranges: selectedSelections.map((selection) => ({
        start: selection.startTime,
        end: selection.endTime,
      })),
      mergeRanges: options.mergeClipSelectionsOnExport,
      clipTitle: nextClipTitle,
      includeSubtitle: options.includeSubtitle,
      includeDanmu: options.includeDanmu,
      renderDanmuEmotes: options.renderDanmuEmotes,
      danmuRenderOptions: options.danmuRenderOptions,
      srtStyle: options.srtStyle,
    });
  } catch (error) {
    if (taskSettled) {
      return;
    }
    taskSettled = true;
    console.error("切片失败:", error);
    options.reportError(`切片失败: ${error}`);
    finishTask();
  }
}
