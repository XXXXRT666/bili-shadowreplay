import { invoke } from "../../../invoker";

const SEEKBAR_THUMBNAIL_DEFAULT_STEP_SECONDS = 5;
const SEEKBAR_THUMBNAIL_MIN_BUCKET_COUNT = 240;
const SEEKBAR_THUMBNAIL_CACHE_LIMIT = 180;
const SEEKBAR_THUMBNAIL_CACHE_POLL_INTERVAL_MS = 2000;

interface SeekbarThumbnailRuntimeOptions {
  getShouldGenerate: () => boolean;
  getVideoId: () => number | null;
  getDuration: () => number;
  getShowSeekbarPopup: () => boolean;
  getSeekbarPopupTime: () => number;
  setPreviewImageSrc: (value: string) => void;
}

export function createSeekbarThumbnailRuntime(
  options: SeekbarThumbnailRuntimeOptions
) {
  let seekbarThumbnailCache = new Map<number, string>();
  let seekbarThumbnailPendingMap = new Map<number, Promise<string | null>>();
  let seekbarThumbnailRequestTime: number | null = null;
  let seekbarThumbnailRequestVersion = 0;
  let seekbarThumbnailWorkerRunning = false;
  let seekbarThumbnailSessionToken = 0;
  let seekbarThumbnailCacheReady = false;
  let seekbarThumbnailCacheReadyVideoId: number | null = null;
  let seekbarThumbnailCacheTaskPending = false;
  let seekbarThumbnailCachePollTimer: number | null = null;
  let seekbarThumbnailCachePollVideoId: number | null = null;

  function clearPreviewImage() {
    options.setPreviewImageSrc("");
  }

  function clearCache() {
    seekbarThumbnailCache.clear();
    seekbarThumbnailPendingMap.clear();
  }

  function stopCacheReadyPolling() {
    if (
      seekbarThumbnailCachePollTimer !== null &&
      typeof window !== "undefined"
    ) {
      window.clearInterval(seekbarThumbnailCachePollTimer);
    }
    seekbarThumbnailCachePollTimer = null;
    seekbarThumbnailCachePollVideoId = null;
  }

  function getSeekbarThumbnailStepSeconds(duration = options.getDuration()) {
    if (!Number.isFinite(duration) || duration <= 0) {
      return SEEKBAR_THUMBNAIL_DEFAULT_STEP_SECONDS;
    }
    const stepForMinBuckets = duration / SEEKBAR_THUMBNAIL_MIN_BUCKET_COUNT;
    if (!Number.isFinite(stepForMinBuckets) || stepForMinBuckets <= 0) {
      return SEEKBAR_THUMBNAIL_DEFAULT_STEP_SECONDS;
    }
    return Math.min(SEEKBAR_THUMBNAIL_DEFAULT_STEP_SECONDS, stepForMinBuckets);
  }

  function normalizeSeekbarPreviewTime(
    time: number,
    durationOverride = options.getDuration()
  ) {
    const duration = Math.max(0, durationOverride);
    if (!Number.isFinite(duration) || duration <= 0) {
      return 0;
    }
    if (!Number.isFinite(time)) {
      return 0;
    }
    return Math.max(0, Math.min(duration, time));
  }

  function getSeekbarThumbnailBucketKey(
    time: number,
    duration = options.getDuration()
  ) {
    const step = getSeekbarThumbnailStepSeconds(duration);
    const normalizedTime = normalizeSeekbarPreviewTime(time, duration);
    return Math.round(normalizedTime / step);
  }

  function requestRefreshAfterCacheReady() {
    if (!options.getShowSeekbarPopup()) {
      return;
    }
    queueForTime(options.getSeekbarPopupTime());
  }

  async function enqueueCacheTask(videoId: number) {
    if (!options.getShouldGenerate() || seekbarThumbnailCacheTaskPending) {
      return;
    }

    seekbarThumbnailCacheTaskPending = true;
    try {
      await invoke("enqueue_seekbar_thumbnail_cache_task", { videoId });
    } catch (error) {
      console.warn("Failed to enqueue seekbar thumbnail cache task:", error);
    } finally {
      seekbarThumbnailCacheTaskPending = false;
    }
  }

  async function checkCacheReady(
    videoId: number,
    triggerTaskIfMissing: boolean
  ) {
    if (seekbarThumbnailCacheReady && seekbarThumbnailCacheReadyVideoId === videoId) {
      return true;
    }

    try {
      const hasCache = await invoke<boolean>("has_seekbar_thumbnail_cache", {
        videoId,
      });
      if (hasCache) {
        seekbarThumbnailCacheReady = true;
        seekbarThumbnailCacheReadyVideoId = videoId;
        stopCacheReadyPolling();
        requestRefreshAfterCacheReady();
        return true;
      }
    } catch (error) {
      console.warn("Failed to check seekbar thumbnail cache:", error);
    }

    seekbarThumbnailCacheReady = false;
    seekbarThumbnailCacheReadyVideoId = null;
    if (triggerTaskIfMissing && options.getShouldGenerate()) {
      void enqueueCacheTask(videoId);
      startCacheReadyPolling(videoId);
    }
    return false;
  }

  function startCacheReadyPolling(videoId: number) {
    if (typeof window === "undefined") {
      return;
    }
    if (seekbarThumbnailCachePollTimer !== null && seekbarThumbnailCachePollVideoId === videoId) {
      return;
    }

    stopCacheReadyPolling();
    seekbarThumbnailCachePollVideoId = videoId;
    const sessionToken = seekbarThumbnailSessionToken;
    seekbarThumbnailCachePollTimer = window.setInterval(() => {
      if (
        sessionToken !== seekbarThumbnailSessionToken ||
        seekbarThumbnailCachePollVideoId !== videoId ||
        options.getVideoId() !== videoId ||
        !options.getShouldGenerate()
      ) {
        stopCacheReadyPolling();
        return;
      }

      void checkCacheReady(videoId, false);
    }, SEEKBAR_THUMBNAIL_CACHE_POLL_INTERVAL_MS);
  }

  async function generateThumbnailImageFromBackend(
    videoId: number,
    time: number,
    duration: number,
    sessionToken: number
  ) {
    if (sessionToken !== seekbarThumbnailSessionToken) {
      return null;
    }

    const targetTime = normalizeSeekbarPreviewTime(time, duration);
    try {
      const dataUrl = await invoke<string>("generate_seekbar_thumbnail", {
        videoId,
        timestamp: targetTime,
      });
      if (sessionToken !== seekbarThumbnailSessionToken) {
        return null;
      }
      return typeof dataUrl === "string" && dataUrl.length > 0 ? dataUrl : null;
    } catch (error) {
      const message = String(error ?? "");
      if (!message.includes("Seekbar thumbnail cache is not ready")) {
        console.warn("Failed to generate seekbar thumbnail via backend:", error);
      }
      seekbarThumbnailCacheReady = false;
      seekbarThumbnailCacheReadyVideoId = null;
      return null;
    }
  }

  async function getThumbnailAtTime(time: number) {
    const duration = options.getDuration();
    if (!Number.isFinite(duration) || duration <= 0) {
      return null;
    }

    const videoId = options.getVideoId();
    if (videoId === null) {
      return null;
    }

    const bucketKey = getSeekbarThumbnailBucketKey(time, duration);
    const cached = seekbarThumbnailCache.get(bucketKey);
    if (cached) {
      seekbarThumbnailCache.delete(bucketKey);
      seekbarThumbnailCache.set(bucketKey, cached);
      return cached;
    }

    const pending = seekbarThumbnailPendingMap.get(bucketKey);
    if (pending) {
      return pending;
    }

    const stepSeconds = getSeekbarThumbnailStepSeconds(duration);
    const bucketTime = normalizeSeekbarPreviewTime(bucketKey * stepSeconds, duration);
    const sessionToken = seekbarThumbnailSessionToken;

    const promise = (async () => {
      const cacheReady = await checkCacheReady(videoId, true);
      if (!cacheReady || sessionToken !== seekbarThumbnailSessionToken) {
        return null;
      }

      const generated = await generateThumbnailImageFromBackend(
        videoId,
        bucketTime,
        duration,
        sessionToken
      );
      if (!generated || sessionToken !== seekbarThumbnailSessionToken) {
        void enqueueCacheTask(videoId);
        startCacheReadyPolling(videoId);
        return null;
      }

      if (seekbarThumbnailCache.has(bucketKey)) {
        seekbarThumbnailCache.delete(bucketKey);
      }
      seekbarThumbnailCache.set(bucketKey, generated);
      if (seekbarThumbnailCache.size > SEEKBAR_THUMBNAIL_CACHE_LIMIT) {
        const firstKey = seekbarThumbnailCache.keys().next().value;
        if (firstKey !== undefined) {
          seekbarThumbnailCache.delete(firstKey);
        }
      }
      return generated;
    })().finally(() => {
      seekbarThumbnailPendingMap.delete(bucketKey);
    });

    seekbarThumbnailPendingMap.set(bucketKey, promise);
    return promise;
  }

  async function processQueue() {
    if (seekbarThumbnailWorkerRunning) {
      return;
    }
    seekbarThumbnailWorkerRunning = true;

    while (seekbarThumbnailRequestTime !== null) {
      const targetTime = seekbarThumbnailRequestTime;
      const requestVersion = seekbarThumbnailRequestVersion;
      seekbarThumbnailRequestTime = null;
      const imageSrc = await getThumbnailAtTime(targetTime);
      if (requestVersion !== seekbarThumbnailRequestVersion) {
        continue;
      }
      options.setPreviewImageSrc(imageSrc ?? "");
    }

    seekbarThumbnailWorkerRunning = false;
  }

  function queueForTime(time: number) {
    if (!Number.isFinite(time) || time < 0) {
      clearPreviewImage();
      return;
    }
    seekbarThumbnailRequestTime = time;
    seekbarThumbnailRequestVersion += 1;
    if (!seekbarThumbnailWorkerRunning) {
      void processQueue();
    }
  }

  function reset(resetCache = false) {
    clearPreviewImage();
    seekbarThumbnailRequestTime = null;
    seekbarThumbnailRequestVersion += 1;
    seekbarThumbnailWorkerRunning = false;
    seekbarThumbnailSessionToken += 1;
    if (resetCache) {
      stopCacheReadyPolling();
      seekbarThumbnailCacheReady = false;
      seekbarThumbnailCacheReadyVideoId = null;
      seekbarThumbnailCacheTaskPending = false;
      clearCache();
    }
  }

  async function prepare() {
    const videoId = options.getVideoId();
    if (videoId === null) {
      stopCacheReadyPolling();
      seekbarThumbnailCacheReady = false;
      seekbarThumbnailCacheReadyVideoId = null;
      return;
    }
    await checkCacheReady(videoId, options.getShouldGenerate());
  }

  function dispose() {
    stopCacheReadyPolling();
    reset(true);
  }

  return {
    dispose,
    prepare,
    queueForTime,
    reset,
  };
}
