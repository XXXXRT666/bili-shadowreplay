import type { DanmuEntry, VideoPbpData } from "../../../interface";

export type SeekbarPbpGenerationMethod =
  | "bilibili_pbp"
  | "conv_curve"
  | "ts_quadratic";

export const SEEKBAR_PBP_METHOD_OPTIONS: Array<{
  value: SeekbarPbpGenerationMethod;
  label: string;
}> = [
  { value: "conv_curve", label: "高斯卷积" },
  { value: "ts_quadratic", label: "二次曲线" },
];

export const BILIBILI_PBP_METHOD_OPTION: {
  value: SeekbarPbpGenerationMethod;
  label: string;
} = { value: "bilibili_pbp", label: "B站高能" };

export const SEEKBAR_PBP_VIEWBOX_WIDTH = 1000;
export const SEEKBAR_PBP_VIEWBOX_HEIGHT = 100;

const TIMELINE_MARKER_END_GAP_RATIO = 0.35;
const TIMELINE_MARKER_END_GAP_MIN_SECONDS = 8;
const SEEKBAR_PBP_BASELINE_Y = 72;
const SEEKBAR_PBP_MIN_Y = 4;
const SEEKBAR_PBP_BASE_SAMPLE_COUNT = 240;
const SEEKBAR_PBP_MIN_SAMPLE_COUNT = 160;
const SEEKBAR_PBP_MAX_SAMPLE_COUNT = 4200;
const SEEKBAR_PBP_HISTOGRAM_MIN_SMOOTH_RADIUS = 2;
const SEEKBAR_PBP_HISTOGRAM_MAX_SMOOTH_RADIUS = 10;
const SEEKBAR_PBP_HISTOGRAM_SMOOTH_PASSES = 2;
const SEEKBAR_PBP_REFERENCE_BUCKET_MS = 1000;
const SEEKBAR_PBP_REFERENCE_SMOOTH_RADIUS = 2;
const SEEKBAR_PBP_VALUE_EASE_EXPONENT = 0.5;
const SEEKBAR_PBP_WAVE_GAIN = 1.75;
const SEEKBAR_PBP_CURVE_TENSION = 0.82;
const SEEKBAR_PBP_EDGE_TRANSITION_RATIO = 0.085;

export function resolveTimelineMarkers(duration: number, scale: number): number[] {
  if (!Number.isFinite(duration) || duration <= 0) {
    return [];
  }

  const minTotalMarkers = 10;
  const visibleSeconds = duration / Math.max(1, Number.isFinite(scale) ? scale : 1);
  const maxIntervalForMinMarkers = visibleSeconds / (minTotalMarkers - 1);
  const minuteIntervalCandidates = [1800, 900, 600, 300, 120, 60];
  const shortVideoSecondCandidates = [30, 15, 10, 5];
  const primaryCandidates =
    visibleSeconds <= 60 ? shortVideoSecondCandidates : minuteIntervalCandidates;

  let selectedInterval: number | null = null;
  for (const interval of primaryCandidates) {
    if (interval <= maxIntervalForMinMarkers) {
      selectedInterval = interval;
      break;
    }
  }

  if (selectedInterval === null) {
    const fallbackSteps = [60, 30, 15, 10, 5, 2, 1];
    selectedInterval =
      fallbackSteps.find((step) => step <= maxIntervalForMinMarkers) ??
      Math.max(0.5, maxIntervalForMinMarkers);
  }

  const internalMarkers: number[] = [];
  for (
    let markerTime = selectedInterval;
    markerTime < duration - 0.0001;
    markerTime += selectedInterval
  ) {
    internalMarkers.push(markerTime);
  }

  const minEndGapSeconds = Math.max(
    TIMELINE_MARKER_END_GAP_MIN_SECONDS,
    selectedInterval * TIMELINE_MARKER_END_GAP_RATIO
  );
  while (internalMarkers.length > 0) {
    const lastMarker = internalMarkers[internalMarkers.length - 1];
    if (duration - lastMarker >= minEndGapSeconds) {
      break;
    }
    internalMarkers.pop();
  }

  return internalMarkers;
}

function buildSeekbarPbpFlatPath() {
  return (
    `M 0 ${SEEKBAR_PBP_VIEWBOX_HEIGHT} ` +
    `L 0 ${SEEKBAR_PBP_BASELINE_Y} ` +
    `L ${SEEKBAR_PBP_VIEWBOX_WIDTH} ${SEEKBAR_PBP_BASELINE_Y} ` +
    `L ${SEEKBAR_PBP_VIEWBOX_WIDTH} ${SEEKBAR_PBP_VIEWBOX_HEIGHT} Z`
  );
}

function smoothSeekbarPbpHistogramOnce(values: Float64Array, radius: number) {
  const length = values.length;
  if (length === 0 || radius <= 0) {
    return values.slice();
  }

  const clampedRadius = Math.max(1, Math.floor(radius));
  const kernelSize = clampedRadius * 2 + 1;
  const kernel = new Float64Array(kernelSize);
  const sigma = Math.max(0.8, clampedRadius / 1.8);
  const sigma2 = sigma * sigma * 2;
  let kernelSum = 0;
  for (let offset = -clampedRadius; offset <= clampedRadius; offset += 1) {
    const weight = Math.exp(-(offset * offset) / sigma2);
    const kernelIndex = offset + clampedRadius;
    kernel[kernelIndex] = weight;
    kernelSum += weight;
  }
  if (kernelSum <= 0) {
    return values.slice();
  }
  for (let index = 0; index < kernelSize; index += 1) {
    kernel[index] /= kernelSum;
  }

  const smoothed = new Float64Array(length);
  for (let index = 0; index < length; index += 1) {
    let convolvedValue = 0;
    for (let offset = -clampedRadius; offset <= clampedRadius; offset += 1) {
      const sampleIndex = Math.max(0, Math.min(length - 1, index + offset));
      const kernelIndex = offset + clampedRadius;
      convolvedValue += values[sampleIndex] * kernel[kernelIndex];
    }
    smoothed[index] = convolvedValue;
  }
  return smoothed;
}

function smoothSeekbarPbpHistogram(values: Float64Array, radius: number, passes = 1) {
  if (passes <= 1) {
    return smoothSeekbarPbpHistogramOnce(values, radius);
  }

  let smoothed = values.slice();
  for (let pass = 0; pass < passes; pass += 1) {
    smoothed = smoothSeekbarPbpHistogramOnce(smoothed, radius);
  }
  return smoothed;
}

export function resolveSeekbarPbpSampleCount(duration: number, scale: number) {
  if (!Number.isFinite(duration) || duration <= 0) {
    return SEEKBAR_PBP_BASE_SAMPLE_COUNT;
  }

  const zoomFactor = Math.max(1, Number.isFinite(scale) ? scale : 1);
  const sampleCount = Math.round(SEEKBAR_PBP_BASE_SAMPLE_COUNT * zoomFactor);
  return Math.max(
    SEEKBAR_PBP_MIN_SAMPLE_COUNT,
    Math.min(SEEKBAR_PBP_MAX_SAMPLE_COUNT, sampleCount)
  );
}

function resolveSeekbarPbpSmoothRadius(sampleCount: number) {
  const normalizedCount = Math.max(
    SEEKBAR_PBP_MIN_SAMPLE_COUNT,
    Math.min(SEEKBAR_PBP_MAX_SAMPLE_COUNT, sampleCount)
  );
  const ratio =
    (normalizedCount - SEEKBAR_PBP_MIN_SAMPLE_COUNT) /
    Math.max(1, SEEKBAR_PBP_MAX_SAMPLE_COUNT - SEEKBAR_PBP_MIN_SAMPLE_COUNT);
  const radius =
    SEEKBAR_PBP_HISTOGRAM_MIN_SMOOTH_RADIUS +
    Math.round(
      ratio *
        (SEEKBAR_PBP_HISTOGRAM_MAX_SMOOTH_RADIUS -
          SEEKBAR_PBP_HISTOGRAM_MIN_SMOOTH_RADIUS)
    );
  return Math.max(
    SEEKBAR_PBP_HISTOGRAM_MIN_SMOOTH_RADIUS,
    Math.min(SEEKBAR_PBP_HISTOGRAM_MAX_SMOOTH_RADIUS, radius)
  );
}

function buildSeekbarPbpHistogram(
  records: DanmuEntry[],
  durationMs: number,
  bucketCount: number,
  bucketSpanMs: number
) {
  const histogram = new Float64Array(bucketCount);
  for (const record of records) {
    const timestamp = Number(record.ts);
    if (!Number.isFinite(timestamp) || timestamp < 0 || timestamp > durationMs) {
      continue;
    }
    const rawIndex = Math.floor(timestamp / bucketSpanMs);
    const clampedIndex = Math.max(0, Math.min(bucketCount - 1, rawIndex));
    histogram[clampedIndex] += 1;
  }
  return histogram;
}

export function resolveSeekbarPbpGlobalMaxDensity(
  records: DanmuEntry[],
  duration: number,
  method: SeekbarPbpGenerationMethod
) {
  if (!Number.isFinite(duration) || duration <= 0 || records.length === 0) {
    return 0;
  }

  const durationMs = duration * 1000;
  const bucketCount = Math.max(1, Math.ceil(durationMs / SEEKBAR_PBP_REFERENCE_BUCKET_MS));
  const histogram = buildSeekbarPbpHistogram(
    records,
    durationMs,
    bucketCount,
    SEEKBAR_PBP_REFERENCE_BUCKET_MS
  );

  const radius =
    method === "ts_quadratic"
      ? Math.max(1, Math.round(SEEKBAR_PBP_REFERENCE_SMOOTH_RADIUS * 0.8))
      : SEEKBAR_PBP_REFERENCE_SMOOTH_RADIUS;
  const passes = method === "ts_quadratic" ? 1 : SEEKBAR_PBP_HISTOGRAM_SMOOTH_PASSES;
  const smoothedHistogram = smoothSeekbarPbpHistogram(histogram, radius, passes);
  const densityFactor = 1000 / SEEKBAR_PBP_REFERENCE_BUCKET_MS;
  let maxDensity = 0;
  for (const value of smoothedHistogram) {
    const density = value * densityFactor;
    if (density > maxDensity) {
      maxDensity = density;
    }
  }
  return maxDensity;
}

function mapSeekbarPbpDensityToY(value: number, maxDensity: number) {
  if (!Number.isFinite(value) || value <= 0 || maxDensity <= 0) {
    return SEEKBAR_PBP_BASELINE_Y;
  }

  const normalized = Math.max(0, Math.min(1, value / maxDensity));
  const eased = Math.pow(normalized, SEEKBAR_PBP_VALUE_EASE_EXPONENT);
  const amplitude = SEEKBAR_PBP_BASELINE_Y - SEEKBAR_PBP_MIN_Y;
  const boosted = Math.max(0, Math.min(1, eased * SEEKBAR_PBP_WAVE_GAIN));
  return SEEKBAR_PBP_BASELINE_Y - amplitude * boosted;
}

function applySeekbarPbpEdgeTransition(values: Float64Array) {
  if (values.length <= 2) {
    return;
  }

  const edgePointCount = Math.max(
    2,
    Math.floor(values.length * SEEKBAR_PBP_EDGE_TRANSITION_RATIO)
  );
  for (let index = 0; index < values.length; index += 1) {
    const distanceToNearestEdge = Math.min(index, values.length - 1 - index);
    if (distanceToNearestEdge >= edgePointCount) {
      continue;
    }
    const t = distanceToNearestEdge / edgePointCount;
    const eased = Math.sin((Math.PI * t) / 2);
    const amplitude = SEEKBAR_PBP_BASELINE_Y - values[index];
    values[index] = SEEKBAR_PBP_BASELINE_Y - amplitude * eased;
  }
}

export function buildSeekbarPbpCurvePath(
  records: DanmuEntry[],
  duration: number,
  scale: number,
  globalMaxDensity: number,
  method: SeekbarPbpGenerationMethod
) {
  if (!Number.isFinite(duration) || duration <= 0 || records.length === 0) {
    return buildSeekbarPbpFlatPath();
  }

  const sampleCount = resolveSeekbarPbpSampleCount(duration, scale);
  const durationMs = duration * 1000;
  const bucketSpanMs = durationMs / sampleCount;
  if (!Number.isFinite(bucketSpanMs) || bucketSpanMs <= 0) {
    return buildSeekbarPbpFlatPath();
  }

  const histogram = buildSeekbarPbpHistogram(records, durationMs, sampleCount, bucketSpanMs);
  const seriesValues = new Float64Array(sampleCount);
  let scaleMaxDensity = Math.max(0, globalMaxDensity);

  const baseSmoothRadius = resolveSeekbarPbpSmoothRadius(sampleCount);
  const smoothRadius =
    method === "ts_quadratic"
      ? Math.max(1, Math.round(baseSmoothRadius * 0.6))
      : baseSmoothRadius;
  const smoothPasses = method === "ts_quadratic" ? 1 : SEEKBAR_PBP_HISTOGRAM_SMOOTH_PASSES;
  const smoothedHistogram = smoothSeekbarPbpHistogram(
    histogram,
    smoothRadius,
    smoothPasses
  );
  const densityFactor = 1000 / bucketSpanMs;
  for (let index = 0; index < sampleCount; index += 1) {
    seriesValues[index] = smoothedHistogram[index] * densityFactor;
  }
  if (scaleMaxDensity <= 0) {
    for (const density of seriesValues) {
      if (density > scaleMaxDensity) {
        scaleMaxDensity = density;
      }
    }
  }

  if (scaleMaxDensity <= 0) {
    return buildSeekbarPbpFlatPath();
  }

  const values = new Float64Array(sampleCount);
  for (let index = 0; index < sampleCount; index += 1) {
    values[index] = mapSeekbarPbpDensityToY(seriesValues[index], scaleMaxDensity);
  }
  applySeekbarPbpEdgeTransition(values);

  if (values.length === 0) {
    return buildSeekbarPbpFlatPath();
  }

  const format = (value: number) => value.toFixed(1);
  const barWidth = SEEKBAR_PBP_VIEWBOX_WIDTH / sampleCount;
  const points: Array<{ x: number; y: number }> = [{ x: 0, y: values[0] }];
  for (let index = 0; index < sampleCount; index += 1) {
    const centerX = Math.min(SEEKBAR_PBP_VIEWBOX_WIDTH, (index + 0.5) * barWidth);
    points.push({ x: centerX, y: values[index] });
  }
  points.push({
    x: SEEKBAR_PBP_VIEWBOX_WIDTH,
    y: values[sampleCount - 1],
  });

  let path =
    `M 0 ${format(SEEKBAR_PBP_VIEWBOX_HEIGHT)} ` +
    `L ${format(points[0].x)} ${format(points[0].y)}`;
  if (method === "ts_quadratic" && points.length > 2) {
    for (let index = 1; index < points.length - 2; index += 1) {
      const xc = (points[index].x + points[index + 1].x) / 2;
      const yc = (points[index].y + points[index + 1].y) / 2;
      path +=
        ` Q ${format(points[index].x)} ${format(points[index].y)}` +
        ` ${format(xc)} ${format(yc)}`;
    }
    const penultimate = points[points.length - 2];
    const tail = points[points.length - 1];
    path +=
      ` Q ${format(penultimate.x)} ${format(penultimate.y)}` +
      ` ${format(tail.x)} ${format(tail.y)}`;
  } else {
    const curveFactor = SEEKBAR_PBP_CURVE_TENSION / 6;
    const clampY = (value: number) =>
      Math.max(SEEKBAR_PBP_MIN_Y, Math.min(SEEKBAR_PBP_BASELINE_Y, value));
    for (let index = 0; index < points.length - 1; index += 1) {
      const prev = points[Math.max(0, index - 1)];
      const current = points[index];
      const next = points[index + 1];
      const afterNext = points[Math.min(points.length - 1, index + 2)];

      const cp1x = current.x + (next.x - prev.x) * curveFactor;
      const cp1y = clampY(current.y + (next.y - prev.y) * curveFactor);
      const cp2x = next.x - (afterNext.x - current.x) * curveFactor;
      const cp2y = clampY(next.y - (afterNext.y - current.y) * curveFactor);

      path +=
        ` C ${format(cp1x)} ${format(cp1y)}` +
        ` ${format(cp2x)} ${format(cp2y)}` +
        ` ${format(next.x)} ${format(next.y)}`;
    }
  }

  path += ` L ${format(SEEKBAR_PBP_VIEWBOX_WIDTH)} ${format(SEEKBAR_PBP_VIEWBOX_HEIGHT)} Z`;
  return path;
}

export function buildBilibiliSeekbarPbpCurvePath(
  data: VideoPbpData | null | undefined,
  duration: number
) {
  if (
    !data ||
    !Number.isFinite(duration) ||
    duration <= 0 ||
    !Number.isFinite(data.stepSec) ||
    data.stepSec <= 0 ||
    !Array.isArray(data.values) ||
    data.values.length === 0
  ) {
    return buildSeekbarPbpFlatPath();
  }

  const values = data.values
    .map((value) => Number(value))
    .filter((value) => Number.isFinite(value) && value >= 0);
  let maxValue = 0;
  for (const value of values) {
    if (value > maxValue) {
      maxValue = value;
    }
  }
  if (values.length === 0 || maxValue <= 0) {
    return buildSeekbarPbpFlatPath();
  }

  const format = (value: number) => value.toFixed(1);
  const points = values.map((value, index) => {
    const x = Math.max(
      0,
      Math.min(
        SEEKBAR_PBP_VIEWBOX_WIDTH,
        ((index * data.stepSec) / duration) * SEEKBAR_PBP_VIEWBOX_WIDTH
      )
    );
    return { x, y: mapSeekbarPbpDensityToY(value, maxValue) };
  });

  if (points[0]?.x !== 0) {
    points.unshift({ x: 0, y: points[0]?.y ?? SEEKBAR_PBP_BASELINE_Y });
  }

  const lastPoint = points[points.length - 1];
  if (lastPoint && lastPoint.x < SEEKBAR_PBP_VIEWBOX_WIDTH) {
    points.push({
      x: SEEKBAR_PBP_VIEWBOX_WIDTH,
      y: lastPoint.y,
    });
  }

  let path =
    `M 0 ${format(SEEKBAR_PBP_VIEWBOX_HEIGHT)} ` +
    `L ${format(points[0].x)} ${format(points[0].y)}`;

  const curveFactor = SEEKBAR_PBP_CURVE_TENSION / 6;
  const clampY = (value: number) =>
    Math.max(SEEKBAR_PBP_MIN_Y, Math.min(SEEKBAR_PBP_BASELINE_Y, value));
  for (let index = 0; index < points.length - 1; index += 1) {
    const prev = points[Math.max(0, index - 1)];
    const current = points[index];
    const next = points[index + 1];
    const afterNext = points[Math.min(points.length - 1, index + 2)];

    const cp1x = current.x + (next.x - prev.x) * curveFactor;
    const cp1y = clampY(current.y + (next.y - prev.y) * curveFactor);
    const cp2x = next.x - (afterNext.x - current.x) * curveFactor;
    const cp2y = clampY(next.y - (afterNext.y - current.y) * curveFactor);

    path +=
      ` C ${format(cp1x)} ${format(cp1y)}` +
      ` ${format(cp2x)} ${format(cp2y)}` +
      ` ${format(next.x)} ${format(next.y)}`;
  }

  path += ` L ${format(SEEKBAR_PBP_VIEWBOX_WIDTH)} ${format(SEEKBAR_PBP_VIEWBOX_HEIGHT)} Z`;
  return path;
}

export function formatTime(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `${minutes}:${remainingSeconds.toFixed(1).padStart(4, "0")}`;
}

export function formatTimelineMarkerTime(seconds: number): string {
  const clamped = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(clamped / 3600);
  const minutes = Math.floor((clamped % 3600) / 60);
  const remainingSeconds = clamped % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(
      remainingSeconds
    ).padStart(2, "0")}`;
  }
  return `${minutes}:${String(remainingSeconds).padStart(2, "0")}`;
}

export function formatTimeForSeekInput(seconds: number): string {
  const clamped = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(clamped / 3600);
  const minutes = Math.floor((clamped % 3600) / 60);
  const remainingSeconds = clamped % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(
      remainingSeconds
    ).padStart(2, "0")}`;
  }
  return `${minutes}:${String(remainingSeconds).padStart(2, "0")}`;
}

export function parseTimeInput(value: string): number | null {
  const cleaned = value.trim();
  if (!cleaned || !/^[0-9:.]+$/.test(cleaned)) {
    return null;
  }

  const segments = cleaned.split(":");
  if (segments.length > 3 || segments.some((segment) => segment.length === 0)) {
    return null;
  }

  const secondsPart = Number.parseFloat(segments[segments.length - 1]);
  if (!Number.isFinite(secondsPart)) {
    return null;
  }

  let minutesPart = 0;
  let hoursPart = 0;
  if (segments.length >= 2) {
    minutesPart = Number.parseInt(segments[segments.length - 2], 10);
    if (!Number.isFinite(minutesPart)) {
      return null;
    }
  }
  if (segments.length === 3) {
    hoursPart = Number.parseInt(segments[0], 10);
    if (!Number.isFinite(hoursPart)) {
      return null;
    }
  }

  const totalSeconds = hoursPart * 3600 + minutesPart * 60 + secondsPart;
  if (!Number.isFinite(totalSeconds)) {
    return null;
  }
  return Math.max(0, totalSeconds);
}
