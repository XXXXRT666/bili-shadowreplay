<script lang="ts">
  import type { DanmakuSegment } from "../../danmaku-emotes";
  import type { ActiveDanmu } from "./danmu/danmuRuntime";

  export let activeDanmus: ActiveDanmu[] = [];
  export let isPlaying = false;
  export let animationRate = 1;
  export let playbackTimeMs = 0;
  export let videoDanmuFontSize = "";
  export let videoDanmuFontFamily = "";
  export let videoDanmuFontWeight = "";
  export let videoDanmuLineHeight = "";
  export let videoDanmuEmoteScale = "";
  export let videoDanmuEmoteOffset = "";
  export let videoDanmuOpacity = "";
  export let videoDanmuTextShadow = "";
  export let getVideoDanmuEmoteStyle:
    | ((segments: DanmakuSegment[], index: number) => string)
    | undefined = undefined;
  export let onRemoveActiveDanmu: ((id: number) => void) | undefined = undefined;

  function getDanmuTravelDistancePx(danmu: ActiveDanmu): number {
    const distance = danmu.durationMs * danmu.speedPxPerMs;
    return Number.isFinite(distance) && distance > 0
      ? distance
      : danmu.widthPx;
  }

  function clampNumber(value: number, min: number, max: number): number {
    return Math.max(min, Math.min(max, value));
  }

  function getComputedTranslateXPx(node: HTMLElement): number {
    const transform = getComputedStyle(node).transform;
    if (!transform || transform === "none") {
      return 0;
    }

    try {
      return new DOMMatrixReadOnly(transform).m41;
    } catch {
      const matrixMatch = transform.match(/matrix\(([^)]+)\)/);
      if (!matrixMatch) {
        return 0;
      }
      const values = matrixMatch[1]
        .split(",")
        .map((part) => Number.parseFloat(part.trim()));
      return Number.isFinite(values[4]) ? values[4] : 0;
    }
  }

  function getRemainingDurationMs(
    danmu: ActiveDanmu,
    translateXPx: number,
    rate: number,
  ): number {
    const distancePx = getDanmuTravelDistancePx(danmu);
    const remainingPx = clampNumber(distancePx + translateXPx, 0, distancePx);
    return remainingPx / Math.max(danmu.speedPxPerMs * Math.max(rate, 0.1), 0.001);
  }

  type DanmuAnimationParams = {
    danmu: ActiveDanmu;
    isPlaying: boolean;
    animationRate: number;
    playbackTimeMs: number;
  };

  function danmuAnimation(node: HTMLElement, params: DanmuAnimationParams) {
    let current = params;
    let isRunning = false;
    let lastRate = Math.max(params.animationRate, 0.1);

    function applyStaticVariables(danmu: ActiveDanmu) {
      node.style.setProperty(
        "--video-danmu-distance",
        `${getDanmuTravelDistancePx(danmu)}px`,
      );
    }

    function setStillAt(translateXPx: number) {
      node.style.animation = "none";
      node.style.transform = `translateX(${translateXPx}px)`;
      isRunning = false;
    }

    function startFrom(translateXPx: number, rate: number) {
      const durationMs = getRemainingDurationMs(current.danmu, translateXPx, rate);
      if (durationMs <= 0) {
        setStillAt(-getDanmuTravelDistancePx(current.danmu));
        return;
      }

      node.style.setProperty("--video-danmu-from", `${translateXPx}px`);
      node.style.animation = "none";
      node.style.transform = `translateX(${translateXPx}px)`;
      node.getBoundingClientRect();
      node.style.animation = `video-danmu-fly ${durationMs}ms linear forwards`;
      isRunning = true;
    }

    function getInitialTranslateXPx(danmu: ActiveDanmu, timeMs: number) {
      const elapsedMs = clampNumber(
        Math.max(danmu.elapsedMs, timeMs - danmu.scheduledStartMs),
        0,
        danmu.durationMs,
      );
      return -elapsedMs * danmu.speedPxPerMs;
    }

    applyStaticVariables(current.danmu);
    if (current.isPlaying) {
      startFrom(
        getInitialTranslateXPx(current.danmu, current.playbackTimeMs),
        lastRate,
      );
    } else {
      setStillAt(getInitialTranslateXPx(current.danmu, current.playbackTimeMs));
    }

    return {
      update(next: DanmuAnimationParams) {
        const previous = current;
        current = next;
        const nextRate = Math.max(next.animationRate, 0.1);
        applyStaticVariables(next.danmu);

        if (next.danmu.id !== previous.danmu.id) {
          lastRate = nextRate;
          if (next.isPlaying) {
            startFrom(
              getInitialTranslateXPx(next.danmu, next.playbackTimeMs),
              nextRate,
            );
          } else {
            setStillAt(getInitialTranslateXPx(next.danmu, next.playbackTimeMs));
          }
          return;
        }

        if (!previous.isPlaying && next.isPlaying) {
          startFrom(getComputedTranslateXPx(node), lastRate);
        } else if (previous.isPlaying && !next.isPlaying) {
          setStillAt(getComputedTranslateXPx(node));
        }
      },
      destroy() {
        node.style.animation = "none";
      },
    };
  }
</script>

{#each activeDanmus as danmu (danmu.id)}
  <p
    class="video-danmu-item"
    data-video-danmu-id={danmu.id}
    use:danmuAnimation={{ danmu, isPlaying, animationRate, playbackTimeMs }}
    style={`--video-danmu-font-size: ${videoDanmuFontSize}; --video-danmu-font-family: ${videoDanmuFontFamily}; --video-danmu-font-weight: ${videoDanmuFontWeight}; --video-danmu-line-height: ${videoDanmuLineHeight}; --video-danmu-emote-scale: ${videoDanmuEmoteScale}; --video-danmu-emote-offset: ${videoDanmuEmoteOffset}; --video-danmu-opacity: ${videoDanmuOpacity}; --video-danmu-text-shadow: ${videoDanmuTextShadow}; top: ${danmu.top}%;`}
    on:animationend={() => {
      if (isPlaying) {
        onRemoveActiveDanmu?.(danmu.id);
      }
    }}
  >
    {#each danmu.segments as segment, index (`stable-${danmu.id}-${index}`)}
      {#if segment.kind === "text"}
        <span>{segment.text}</span>
      {:else}
        <img
          class="video-danmu-emote"
          src={segment.src}
          alt={segment.token}
          style={getVideoDanmuEmoteStyle?.(danmu.segments, index)}
        />
      {/if}
    {/each}
  </p>
{/each}

<style>
  .video-danmu-item {
    position: absolute;
    left: 100%;
    display: flex;
    margin: 0;
    padding: 0;
    color: white;
    opacity: var(--video-danmu-opacity, 1);
    font-size: var(--video-danmu-font-size, 20px);
    font-family: var(--video-danmu-font-family, sans-serif);
    font-weight: var(--video-danmu-font-weight, bold);
    line-height: var(--video-danmu-line-height, 1.3);
    white-space: pre;
    align-items: center;
    text-shadow: var(
      --video-danmu-text-shadow,
      1px 0 1px #000,
      0 1px 1px #000,
      0 -1px 1px #000,
      -1px 0 1px #000
    );
    will-change: transform;
  }

  .video-danmu-emote {
    height: calc(1em * var(--video-danmu-emote-scale, 1));
    width: auto;
    max-width: none;
    display: block;
    flex: 0 0 auto;
    position: relative;
    top: var(--video-danmu-emote-offset, 0);
    margin: 0;
    object-fit: contain;
  }

  @keyframes -global-video-danmu-fly {
    from {
      transform: translateX(var(--video-danmu-from, 0px));
    }

    to {
      transform: translateX(calc(-1 * var(--video-danmu-distance, 100vw)));
    }
  }
</style>
