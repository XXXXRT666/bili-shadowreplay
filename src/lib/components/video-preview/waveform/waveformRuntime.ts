import WaveSurfer from "wavesurfer.js";
import HoverPlugin from "wavesurfer.js/dist/plugins/hover.esm.js";
import RegionsPlugin from "wavesurfer.js/dist/plugins/regions.esm.js";

export interface AudioWaveformData {
  peaks: number[];
  duration: number;
}

export interface EnsureWaveformDataState {
  waveformDataPromise: Promise<AudioWaveformData> | null;
  waveformDataVideoId: number | null;
}

export interface WaveformRuntimeState {
  wavesurfer: any;
  waveformRegions: any;
  isWaveformLoaded: boolean;
  isWaveformLoading: boolean;
  waveformRenderFrame: number | null;
}

export interface CreateWaveSurferRuntimeOptions {
  container: HTMLElement | null;
  videoFile: string | null | undefined;
  showWaveform: boolean;
  panelHeightPx: number;
  barHeightRatio: number;
  formatTimelineMarkerTime: (seconds: number) => string;
  ensureWaveformData: () => Promise<AudioWaveformData>;
  setPreviewTime: (time: number) => void;
  getVideoDuration: () => number;
  hasManagedClipSelection: (id: string) => boolean;
  hasPendingClipStartMarker: () => boolean;
  setActiveClipSelection: (id: string | null) => void;
  updateClipSelectionFromRegion: (region: any) => void;
  syncClipWaveformRegionAppearance: () => void;
  syncClipWaveformRegions: () => void;
  onLoadingStateChange: (loading: boolean) => void;
  onReadyStateChange: (loaded: boolean) => void;
}

export interface CreateWaveSurferRuntimeResult {
  wavesurfer: any;
  waveformRegions: any;
}

export interface RedrawWaveformOptions {
  showWaveform: boolean;
  wavesurfer: any;
  isWaveformLoaded: boolean;
  waveformRenderFrame: number | null;
  videoDuration: number;
  renderWidth: number;
  currentTime: number;
  syncWaveformWithVideo: () => void;
  onFrameChange: (frame: number | null) => void;
}

export async function ensureWaveformData(options: {
  videoId: number | null | undefined;
  state: EnsureWaveformDataState;
  invoke: <T>(command: string, args: Record<string, unknown>) => Promise<T>;
}): Promise<AudioWaveformData> {
  if (!options.videoId) {
    return Promise.reject(new Error("Missing video id"));
  }

  if (
    options.state.waveformDataPromise &&
    options.state.waveformDataVideoId === options.videoId
  ) {
    return options.state.waveformDataPromise;
  }

  const currentVideoId = options.videoId;
  options.state.waveformDataVideoId = currentVideoId;
  options.state.waveformDataPromise = options
    .invoke<AudioWaveformData>("generate_audio_waveform", {
      videoId: currentVideoId,
    })
    .catch((error) => {
      if (options.state.waveformDataVideoId === currentVideoId) {
        options.state.waveformDataPromise = null;
      }
      throw error;
    });

  return options.state.waveformDataPromise;
}

export async function createWaveSurferRuntime(
  options: CreateWaveSurferRuntimeOptions
): Promise<CreateWaveSurferRuntimeResult | null> {
  if (!options.container || !options.videoFile) {
    console.log("Missing container or video file:", {
      container: options.container,
      videoFile: options.videoFile,
    });
    return null;
  }

  options.container.style.width = "100%";
  options.container.style.height = options.showWaveform
    ? `${options.panelHeightPx}px`
    : "0";
  options.container.style.minHeight = options.showWaveform
    ? `${options.panelHeightPx}px`
    : "0";
  options.container.style.overflow = "hidden";

  console.log("Creating WaveSurfer with:", {
    container: options.container,
    file: options.videoFile,
    containerDimensions: {
      width: options.container.offsetWidth,
      height: options.container.offsetHeight,
    },
  });

  const waveformHover = HoverPlugin.create({
    lineColor: "rgba(255, 255, 255, 0.7)",
    lineWidth: 1,
    labelColor: "#ffffff",
    labelSize: 11,
    labelBackground: "rgba(10, 132, 255, 0.88)",
    labelPreferLeft: true,
    formatTimeCallback: options.formatTimelineMarkerTime,
  });
  const waveformRegions = RegionsPlugin.create();
  const wavesurfer = WaveSurfer.create({
    container: options.container,
    waveColor: "#4a5568",
    progressColor: "#0A84FF",
    cursorColor: "#0A84FF",
    barWidth: 2,
    barRadius: 1,
    barGap: 1,
    barHeight: options.barHeightRatio,
    backend: "WebAudio",
    height: options.panelHeightPx,
    normalize: true,
    interact: true,
    dragToSeek: true,
    plugins: [waveformHover, waveformRegions],
  });

  let isDragging = false;

  wavesurfer.on("ready", () => {
    options.onReadyStateChange(true);
    options.onLoadingStateChange(false);
    options.syncClipWaveformRegions();
    console.log("Waveform loaded successfully");
    console.log("WaveSurfer instance:", wavesurfer);
  });

  wavesurfer.on("dragstart", () => {
    isDragging = true;
  });

  wavesurfer.on("dragend", () => {
    isDragging = false;
  });

  wavesurfer.on("interaction", (newTime: number) => {
    if (isDragging) {
      return;
    }
    if (options.getVideoDuration() > 0) {
      options.setPreviewTime(newTime);
    }
  });

  wavesurfer.on("drag", (newTime: number) => {
    const duration = options.getVideoDuration();
    if (duration > 0) {
      options.setPreviewTime(newTime * duration);
    }
  });

  wavesurfer.on("error", (error: unknown) => {
    console.error("WaveSurfer error:", error);
    options.onLoadingStateChange(false);
  });

  wavesurfer.on("loading", (percent: number) => {
    console.log("WaveSurfer loading:", `${percent}%`);
  });

  waveformRegions.on("region-update", (region: any) => {
    if (!region?.id || !options.hasManagedClipSelection(region.id)) {
      return;
    }

    options.updateClipSelectionFromRegion(region);
    options.syncClipWaveformRegionAppearance();
  });

  waveformRegions.on("region-updated", (region: any) => {
    if (!region?.id || !options.hasManagedClipSelection(region.id)) {
      return;
    }

    options.updateClipSelectionFromRegion(region);
    options.syncClipWaveformRegionAppearance();
  });

  waveformRegions.on("region-clicked", (region: any, event: MouseEvent) => {
    const duration = options.getVideoDuration();
    const wrapper =
      typeof wavesurfer.getWrapper === "function"
        ? (wavesurfer.getWrapper() as HTMLElement)
        : null;

    if (wrapper && duration > 0) {
      const rect = wrapper.getBoundingClientRect();

      if (rect.width > 0) {
        const relativeX = Math.max(0, Math.min(rect.width, event.clientX - rect.left));
        const clickedTime = (relativeX / rect.width) * duration;
        options.setPreviewTime(clickedTime);
      }
    }

    if (
      !options.hasPendingClipStartMarker() &&
      region?.id &&
      options.hasManagedClipSelection(region.id)
    ) {
      options.setActiveClipSelection(region.id);
    }

    event.stopPropagation();
  });

  const waveformData = await options.ensureWaveformData();
  const opusFile = options.videoFile.includes(".")
    ? options.videoFile.replace(/\.[^.]+$/, ".opus")
    : `${options.videoFile}.opus`;

  console.log("WaveSurfer created, loading precomputed peaks:", {
    opusFile,
    peakCount: waveformData.peaks.length,
    duration: waveformData.duration,
  });
  await wavesurfer.load(opusFile, [waveformData.peaks], waveformData.duration);

  return {
    wavesurfer,
    waveformRegions,
  };
}

export function syncWaveformWithVideo(options: {
  wavesurfer: any;
  videoDuration: number;
  isWaveformLoaded: boolean;
  currentTime: number;
}): void {
  if (!options.wavesurfer || !options.isWaveformLoaded || !options.videoDuration) {
    return;
  }

  try {
    const progress = options.currentTime / options.videoDuration;
    options.wavesurfer.seekTo(progress);
  } catch (error) {
    console.warn("Failed to sync waveform:", error);
  }
}

export function destroyWaveSurferRuntime(state: WaveformRuntimeState): void {
  if (state.waveformRenderFrame !== null && typeof window !== "undefined") {
    cancelAnimationFrame(state.waveformRenderFrame);
    state.waveformRenderFrame = null;
  }

  if (state.wavesurfer) {
    state.wavesurfer.destroy();
    state.wavesurfer = null;
    state.isWaveformLoaded = false;
    state.isWaveformLoading = false;
  }

  state.waveformRegions = null;
}

export async function redrawWaveformAtCurrentWidth(
  options: RedrawWaveformOptions
): Promise<void> {
  if (!options.showWaveform || !options.wavesurfer || !options.isWaveformLoaded) {
    return;
  }

  if (options.waveformRenderFrame !== null) {
    cancelAnimationFrame(options.waveformRenderFrame);
  }

  const nextFrame = requestAnimationFrame(() => {
    options.onFrameChange(null);

    if (!options.showWaveform || !options.wavesurfer || !options.isWaveformLoaded) {
      return;
    }

    try {
      if (options.videoDuration <= 0 || options.renderWidth <= 0) {
        options.syncWaveformWithVideo();
        return;
      }

      const wrapper =
        typeof options.wavesurfer.getWrapper === "function"
          ? (options.wavesurfer.getWrapper() as HTMLElement)
          : null;
      const currentMinPxPerSec =
        typeof options.wavesurfer.options?.minPxPerSec === "number" &&
        options.wavesurfer.options.minPxPerSec > 0
          ? options.wavesurfer.options.minPxPerSec
          : (wrapper?.scrollWidth ?? options.renderWidth) / options.videoDuration;
      const targetMinPxPerSec = options.renderWidth / options.videoDuration;

      if (Math.abs(currentMinPxPerSec - targetMinPxPerSec) >= 0.01) {
        const anchorTime = Math.max(0, Math.min(options.videoDuration, options.currentTime));
        const viewportWidth =
          typeof options.wavesurfer.getWidth === "function"
            ? options.wavesurfer.getWidth()
            : options.renderWidth;

        options.wavesurfer.zoom(targetMinPxPerSec);

        if (
          wrapper &&
          typeof options.wavesurfer.setScroll === "function" &&
          viewportWidth > 0
        ) {
          const maxScroll = Math.max(0, wrapper.scrollWidth - viewportWidth);
          const targetScroll = Math.max(
            0,
            Math.min(maxScroll, anchorTime * targetMinPxPerSec - viewportWidth / 2)
          );
          options.wavesurfer.setScroll(targetScroll);
        }
      }

      options.syncWaveformWithVideo();
    } catch (error) {
      console.warn("Failed to refresh waveform after scale commit:", error);
    }
  });

  options.onFrameChange(nextFrame);
}
