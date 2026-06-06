import {
  DANMAKU_EMOTE_SCALE,
  DANMAKU_LINE_HEIGHT,
  getDanmakuEmoteRenderKind,
  parseDanmakuContent,
  type DanmakuSegment,
} from "../../../danmaku-emotes";
import type { DanmuEntry, DanmakuEmoteMap } from "../../../interface";

const SCROLL_SPEED_REFERENCE_WIDTH_PX = 512;
const PLAY_RES_X = 1280;
const SCROLL_BASE_DURATION_MS = 4500;
const DANMU_TRACK_GAP_PX = 1;
const DANMU_MAX_DELAY_MS = 500;
const DANMU_DELAY_STEP_MS = 100;
const PREVENT_SUBTITLE_VISIBLE_RATIO = 0.85;
const SPEED_PRESET_RATES = [0.6, 0.8, 1, 1.2, 1.4] as const;

export interface ActiveDanmu {
  id: number;
  content: string;
  emoteRenderKind: "mixed" | "emote-only" | "none";
  segments: DanmakuSegment[];
  elapsedMs: number;
  top: number;
  topPx: number;
  bottomPx: number;
  widthPx: number;
  heightPx: number;
  durationMs: number;
  scheduledStartMs: number;
  endMs: number;
  middleMs: number;
  speedPxPerMs: number;
}

export interface DanmuPlaybackState {
  activeDanmus: ActiveDanmu[];
  recentDanmuPositions: number[];
  nextDanmuRenderId: number;
  lastDanmuTimeMs: number;
  nextDanmuIndex: number;
}

export interface DanmuLayoutConfig {
  containerWidthPx: number;
  containerHeightPx: number;
  fontSizePx: number;
  lineHeight: number;
  displayArea: number;
  speedPreset: number;
  maxOnScreen: number;
  preventSubtitleOcclusion: boolean;
}

interface DanmuMeasurement {
  widthPx: number;
  heightPx: number;
}

interface DanmuPlacement {
  topPx: number;
  scheduledStartMs: number;
  endMs: number;
  middleMs: number;
  durationMs: number;
  speedPxPerMs: number;
}

export function loadDanmuRecords(options: {
  invoke: <T>(command: string, args: Record<string, unknown>) => Promise<T>;
  videoId: number | null | undefined;
}): Promise<DanmuEntry[]> {
  if (!options.videoId) {
    return Promise.resolve([]);
  }

  return options
    .invoke<DanmuEntry[]>("get_video_danmu", {
      id: options.videoId,
    })
    .then((records) => [...records].sort((a, b) => a.ts - b.ts));
}

export function clearActiveDanmusState(
  state: DanmuPlaybackState
): DanmuPlaybackState {
  return {
    ...state,
    activeDanmus: [],
    recentDanmuPositions: [],
  };
}

export function removeActiveDanmuById(
  activeDanmus: ActiveDanmu[],
  id: number
): ActiveDanmu[] {
  return activeDanmus.filter((item) => item.id !== id);
}

export function getDanmuAnimationDelayMs(options: {
  elapsedMs: number;
  syncWithPlaybackRateEnabled: boolean;
  currentPlaybackRate: number;
}): number {
  return (
    options.elapsedMs /
    Math.max(
      options.syncWithPlaybackRateEnabled ? options.currentPlaybackRate : 1,
      0.1
    )
  );
}

export function findDanmuIndexAfter(
  danmuRecords: DanmuEntry[],
  ts: number
): number {
  let left = 0;
  let right = danmuRecords.length;

  while (left < right) {
    const middle = Math.floor((left + right) / 2);
    if (danmuRecords[middle].ts <= ts) {
      left = middle + 1;
    } else {
      right = middle;
    }
  }

  return left;
}

export function getDanmuMaxActiveDurationMs(options: {
  layout: DanmuLayoutConfig;
}): number {
  const containerWidthPx = getPositiveNumber(options.layout.containerWidthPx, 1280);
  const fontSizePx = getPositiveNumber(options.layout.fontSizePx, 25);
  const speedRate = getSpeedPresetRate(options.layout.speedPreset);
  const minSpeedPxPerMs =
    ((getScaledScrollSpeedReferenceWidthPx(options.layout) + fontSizePx) /
      SCROLL_BASE_DURATION_MS) *
    speedRate;

  return Math.ceil(
    (containerWidthPx + containerWidthPx * 1.5) /
      Math.max(minSpeedPxPerMs, 0.001) +
      DANMU_MAX_DELAY_MS
  );
}

function getPositiveNumber(value: number, fallback: number): number {
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

function getSpeedPresetRate(speedPreset: number): number {
  const index = Math.max(
    0,
    Math.min(SPEED_PRESET_RATES.length - 1, Math.round(speedPreset))
  );
  return SPEED_PRESET_RATES[index];
}

function getLayoutScaleX(layout: DanmuLayoutConfig): number {
  return getPositiveNumber(layout.containerWidthPx, PLAY_RES_X) / PLAY_RES_X;
}

function getScaledScrollSpeedReferenceWidthPx(
  layout: DanmuLayoutConfig
): number {
  return SCROLL_SPEED_REFERENCE_WIDTH_PX * getLayoutScaleX(layout);
}

function getAvailableHeightPx(layout: DanmuLayoutConfig): number {
  const containerHeightPx = getPositiveNumber(layout.containerHeightPx, 720);
  const displayAreaRatio =
    Number.isFinite(layout.displayArea) && layout.displayArea > 0
      ? Math.min(layout.displayArea, 100) / 100
      : 1;
  const subtitleRatio = layout.preventSubtitleOcclusion
    ? PREVENT_SUBTITLE_VISIBLE_RATIO
    : 1;

  return Math.max(0, containerHeightPx * Math.min(displayAreaRatio, subtitleRatio));
}

function getDanmuDistancePx(layout: DanmuLayoutConfig): number {
  return getPositiveNumber(layout.containerWidthPx, 1280) + 1;
}

function measureTextWidth(text: string, fontSizePx: number): number {
  let width = 0;

  for (const char of text) {
    if (char === "\n" || char === "\r") {
      continue;
    }

    if (/\s/.test(char)) {
      width += fontSizePx * 0.33;
    } else if (/[\u1100-\u11ff\u2e80-\u9fff\uf900-\ufaff\uff00-\uffef]/.test(char)) {
      width += fontSizePx;
    } else if (/[A-Z0-9]/.test(char)) {
      width += fontSizePx * 0.62;
    } else {
      width += fontSizePx * 0.55;
    }
  }

  return width;
}

function hasVisibleText(segment: DanmakuSegment | null): boolean {
  return !!segment && segment.kind === "text" && segment.text.trim().length > 0;
}

function getSegmentGapPx(
  previousSegment: DanmakuSegment | null,
  currentSegment: DanmakuSegment,
  fontSizePx: number
): number {
  return hasVisibleText(previousSegment) && currentSegment.kind === "emote"
    ? fontSizePx * 0.1
    : 0;
}

function measureDanmuSegments(
  segments: DanmakuSegment[],
  layout: DanmuLayoutConfig
): DanmuMeasurement {
  const fontSizePx = getPositiveNumber(layout.fontSizePx, 25);
  const lineHeight = getPositiveNumber(layout.lineHeight, DANMAKU_LINE_HEIGHT);
  let widthPx = 0;
  let lineWidthPx = 0;
  let lineCount = 1;
  let previousSegment: DanmakuSegment | null = null;

  for (const segment of segments) {
    if (segment.kind === "emote") {
      lineWidthPx +=
        getSegmentGapPx(previousSegment, segment, fontSizePx) +
        fontSizePx * DANMAKU_EMOTE_SCALE;
      previousSegment = segment;
      continue;
    }

    const lines = segment.text.split(/\r?\n/);
    lines.forEach((line, index) => {
      if (index > 0) {
        widthPx = Math.max(widthPx, lineWidthPx);
        lineWidthPx = 0;
        lineCount += 1;
      }
      lineWidthPx +=
        getSegmentGapPx(previousSegment, segment, fontSizePx) +
        measureTextWidth(line, fontSizePx);
      previousSegment = segment;
    });
  }

  widthPx = Math.max(widthPx, lineWidthPx, fontSizePx);

  return {
    widthPx,
    heightPx: Math.max(fontSizePx * lineHeight * lineCount, fontSizePx),
  };
}

function createPlacementBase(options: {
  layout: DanmuLayoutConfig;
  measurement: DanmuMeasurement;
  scheduledStartMs: number;
}): Omit<DanmuPlacement, "topPx"> {
  const distancePx = getDanmuDistancePx(options.layout);
  const speedPxPerMs =
    ((getScaledScrollSpeedReferenceWidthPx(options.layout) +
      options.measurement.widthPx) /
      SCROLL_BASE_DURATION_MS) *
    getSpeedPresetRate(options.layout.speedPreset);
  const durationMs =
    (distancePx + options.measurement.widthPx) /
    Math.max(speedPxPerMs, 0.001);
  const endMs = options.scheduledStartMs + durationMs;
  const middleMs =
    options.scheduledStartMs + distancePx / Math.max(speedPxPerMs, 0.001);

  return {
    scheduledStartMs: options.scheduledStartMs,
    endMs,
    middleMs,
    durationMs,
    speedPxPerMs,
  };
}

function getDanmuRightAt(
  danmu: ActiveDanmu,
  timeMs: number,
  layout: DanmuLayoutConfig
): number {
  const elapsedMs = Math.max(0, timeMs - danmu.scheduledStartMs);
  const x = getDanmuDistancePx(layout) - elapsedMs * danmu.speedPxPerMs;
  return x + danmu.widthPx;
}

function overlapsVertically(
  topPx: number,
  bottomPx: number,
  danmu: ActiveDanmu
): boolean {
  return topPx <= danmu.bottomPx && bottomPx >= danmu.topPx;
}

function canPlaceAt(options: {
  activeDanmus: ActiveDanmu[];
  layout: DanmuLayoutConfig;
  topPx: number;
  measurement: DanmuMeasurement;
  placement: Omit<DanmuPlacement, "topPx">;
}): boolean {
  const bottomPx = options.topPx + options.measurement.heightPx;
  const distancePx = getDanmuDistancePx(options.layout);

  return options.activeDanmus.every((danmu) => {
    if (!overlapsVertically(options.topPx, bottomPx, danmu)) {
      return true;
    }

    if (danmu.endMs < options.placement.middleMs) {
      return (
        getDanmuRightAt(
          danmu,
          options.placement.scheduledStartMs,
          options.layout
        ) < distancePx
      );
    }

    return false;
  });
}

function findPlacementAtStart(options: {
  activeDanmus: ActiveDanmu[];
  layout: DanmuLayoutConfig;
  measurement: DanmuMeasurement;
  scheduledStartMs: number;
}): DanmuPlacement | null {
  const availableHeightPx = getAvailableHeightPx(options.layout);
  if (options.measurement.heightPx >= availableHeightPx) {
    return null;
  }

  const placementBase = createPlacementBase({
    layout: options.layout,
    measurement: options.measurement,
    scheduledStartMs: options.scheduledStartMs,
  });
  const maxTopPx = availableHeightPx - options.measurement.heightPx;
  const sortedActiveDanmus = [...options.activeDanmus].sort(
    (a, b) => a.topPx - b.topPx || a.bottomPx - b.bottomPx
  );
  const candidateTopPositions = [0];

  for (const danmu of sortedActiveDanmus) {
    const nextTop = danmu.bottomPx + DANMU_TRACK_GAP_PX;
    if (nextTop <= maxTopPx) {
      candidateTopPositions.push(nextTop);
    }
  }

  for (const topPx of candidateTopPositions) {
    if (
      canPlaceAt({
        activeDanmus: options.activeDanmus,
        layout: options.layout,
        topPx,
        measurement: options.measurement,
        placement: placementBase,
      })
    ) {
      return {
        ...placementBase,
        topPx,
      };
    }
  }

  return null;
}

function findPlacement(options: {
  activeDanmus: ActiveDanmu[];
  layout: DanmuLayoutConfig;
  measurement: DanmuMeasurement;
  startMs: number;
}): DanmuPlacement | null {
  for (
    let delayMs = 0;
    delayMs <= DANMU_MAX_DELAY_MS;
    delayMs += DANMU_DELAY_STEP_MS
  ) {
    const placement = findPlacementAtStart({
      activeDanmus: options.activeDanmus,
      layout: options.layout,
      measurement: options.measurement,
      scheduledStartMs: options.startMs + delayMs,
    });
    if (placement) {
      return placement;
    }
  }

  return null;
}

export function spawnDanmuEntry(options: {
  state: DanmuPlaybackState;
  entry: DanmuEntry;
  elapsedMs?: number;
  currentTimeMs?: number;
  renderDanmuEmotes: boolean;
  danmakuEmoteMap: DanmakuEmoteMap;
  layout: DanmuLayoutConfig;
}): DanmuPlaybackState {
  if (
    options.layout.maxOnScreen > 0 &&
    options.state.activeDanmus.length >= options.layout.maxOnScreen
  ) {
    return options.state;
  }

  const nextRenderId = options.state.nextDanmuRenderId;
  const shouldRenderEmotes =
    options.renderDanmuEmotes && options.entry.renderEmotes !== false;
  const segments = parseDanmakuContent(
    options.entry.content,
    shouldRenderEmotes ? options.danmakuEmoteMap : {}
  );
  const emoteRenderKind = getDanmakuEmoteRenderKind(segments);
  const measurement = measureDanmuSegments(segments, options.layout);
  const currentTimeMs =
    options.currentTimeMs ?? options.entry.ts + (options.elapsedMs ?? 0);
  const placement = findPlacement({
    activeDanmus: options.state.activeDanmus,
    layout: options.layout,
    measurement,
    startMs: options.entry.ts,
  });

  if (!placement) {
    return options.state;
  }

  return {
    ...options.state,
    activeDanmus: [
      ...options.state.activeDanmus,
      {
        id: nextRenderId,
        content: options.entry.content,
        emoteRenderKind,
        segments,
        elapsedMs: currentTimeMs - placement.scheduledStartMs,
        top:
          (placement.topPx /
            getPositiveNumber(options.layout.containerHeightPx, 720)) *
          100,
        topPx: placement.topPx,
        bottomPx: placement.topPx + measurement.heightPx,
        widthPx: measurement.widthPx,
        heightPx: measurement.heightPx,
        durationMs: placement.durationMs,
        scheduledStartMs: placement.scheduledStartMs,
        endMs: placement.endMs,
        middleMs: placement.middleMs,
        speedPxPerMs: placement.speedPxPerMs,
      },
    ],
    recentDanmuPositions: [],
    nextDanmuRenderId: nextRenderId + 1,
  };
}

export function resetDanmuPlaybackState(options: {
  danmuRecords: DanmuEntry[];
  positionMs?: number;
  state: DanmuPlaybackState;
}): DanmuPlaybackState {
  const nextPositionMs = Math.max(0, options.positionMs ?? 0);
  const clearedState = clearActiveDanmusState(options.state);

  return {
    ...clearedState,
    lastDanmuTimeMs: nextPositionMs,
    nextDanmuIndex: findDanmuIndexAfter(options.danmuRecords, nextPositionMs),
  };
}

export function rebuildDanmuPlaybackState(options: {
  danmuRecords: DanmuEntry[];
  positionMs?: number;
  danmuActiveDurationMs: number;
  renderDanmuEmotes: boolean;
  danmakuEmoteMap: DanmakuEmoteMap;
  layout: DanmuLayoutConfig;
  state: DanmuPlaybackState;
}): DanmuPlaybackState {
  const targetPositionMs = Math.max(0, options.positionMs ?? 0);
  let nextState = clearActiveDanmusState(options.state);

  if (options.danmuRecords.length === 0) {
    return {
      ...nextState,
      lastDanmuTimeMs: targetPositionMs,
      nextDanmuIndex: 0,
    };
  }

  const visibleWindowStart = targetPositionMs - options.danmuActiveDurationMs;
  let index = findDanmuIndexAfter(options.danmuRecords, visibleWindowStart);

  while (
    index < options.danmuRecords.length &&
    options.danmuRecords[index].ts <= targetPositionMs
  ) {
    const danmu = options.danmuRecords[index];
    const elapsedMs = targetPositionMs - danmu.ts;
    if (elapsedMs < options.danmuActiveDurationMs) {
      nextState = {
        ...nextState,
        activeDanmus: nextState.activeDanmus.filter(
          (activeDanmu) => activeDanmu.endMs > danmu.ts
        ),
      };
      nextState = spawnDanmuEntry({
        state: nextState,
        entry: danmu,
        elapsedMs,
        currentTimeMs: targetPositionMs,
        renderDanmuEmotes: options.renderDanmuEmotes,
        danmakuEmoteMap: options.danmakuEmoteMap,
        layout: options.layout,
      });
    }
    index += 1;
  }

  return {
    ...nextState,
    activeDanmus: nextState.activeDanmus.filter(
      (danmu) => danmu.endMs > targetPositionMs
    ),
    lastDanmuTimeMs: targetPositionMs,
    nextDanmuIndex: index,
  };
}

export function syncDanmuPlaybackState(options: {
  danmuRecords: DanmuEntry[];
  currentTimeMs: number;
  danmuLookbackMs: number;
  danmuActiveDurationMs: number;
  renderDanmuEmotes: boolean;
  danmakuEmoteMap: DanmakuEmoteMap;
  layout: DanmuLayoutConfig;
  state: DanmuPlaybackState;
  preserveActiveDanmus?: boolean;
}): DanmuPlaybackState {
  if (options.danmuRecords.length === 0) {
    return {
      ...options.state,
      lastDanmuTimeMs: options.currentTimeMs,
      nextDanmuIndex: 0,
    };
  }

  const windowStart = options.state.lastDanmuTimeMs;
  if (
    options.currentTimeMs < options.state.lastDanmuTimeMs ||
    options.currentTimeMs - options.state.lastDanmuTimeMs >
      options.danmuLookbackMs * 2
  ) {
    const shouldKeepExistingDomDanmus =
      options.preserveActiveDanmus &&
      options.currentTimeMs >= options.state.lastDanmuTimeMs;
    if (!shouldKeepExistingDomDanmus) {
      return rebuildDanmuPlaybackState({
        danmuRecords: options.danmuRecords,
        positionMs: options.currentTimeMs,
        danmuActiveDurationMs: options.danmuActiveDurationMs,
        renderDanmuEmotes: options.renderDanmuEmotes,
        danmakuEmoteMap: options.danmakuEmoteMap,
        layout: options.layout,
        state: options.state,
      });
    }
  }

  let nextState = {
    ...options.state,
  };
  let nextDanmuIndex =
    options.state.nextDanmuIndex > options.danmuRecords.length
      ? options.danmuRecords.length
      : options.state.nextDanmuIndex;

  while (
    nextDanmuIndex < options.danmuRecords.length &&
    options.danmuRecords[nextDanmuIndex].ts <= options.currentTimeMs
  ) {
    const danmu = options.danmuRecords[nextDanmuIndex];
    if (danmu.ts > windowStart) {
      nextState = spawnDanmuEntry({
        state: nextState,
        entry: danmu,
        renderDanmuEmotes: options.renderDanmuEmotes,
        danmakuEmoteMap: options.danmakuEmoteMap,
        currentTimeMs: options.currentTimeMs,
        layout: options.layout,
      });
    }
    nextDanmuIndex += 1;
  }

  return {
    ...nextState,
    lastDanmuTimeMs: options.currentTimeMs,
    nextDanmuIndex,
  };
}
