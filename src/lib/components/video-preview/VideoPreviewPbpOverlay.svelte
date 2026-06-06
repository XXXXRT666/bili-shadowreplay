<script lang="ts">
  export let visible = false;
  export let onMouseDown: ((event: MouseEvent) => void | Promise<void>) | undefined =
    undefined;
  export let viewBox = "";
  export let curveClipPathId = "";
  export let playedClipPathId = "";
  export let curvePath = "";
  export let playedWidth = 0;
  export let viewboxWidth = 0;
  export let viewboxHeight = 0;
</script>

{#if visible}
  <div
    class="bpx-player-pbp show bpx-player-pbp--control-overlay"
    aria-hidden="true"
    on:mousedown={onMouseDown}
  >
    <svg viewBox={viewBox} preserveAspectRatio="none" width="100%" height="100%">
      <defs>
        <clipPath id={curveClipPathId} clipPathUnits="userSpaceOnUse">
          <path d={curvePath}></path>
        </clipPath>
        <clipPath id={playedClipPathId} clipPathUnits="userSpaceOnUse">
          <rect
            class="bpx-player-pbp-played-path"
            x="0"
            y="0"
            width={playedWidth}
            height={viewboxHeight}
          ></rect>
        </clipPath>
      </defs>
      <g class="bpx-player-pbp-videoshot" clip-path={`url(#${curveClipPathId})`}>
        <rect
          x="0"
          y="0"
          width={viewboxWidth}
          height={viewboxHeight}
          fill="rgba(255, 255, 255, 0.2)"
        ></rect>
        <rect
          x="0"
          y="0"
          width={viewboxWidth}
          height={viewboxHeight}
          fill="rgba(0, 161, 214, 0.68)"
          clip-path={`url(#${playedClipPathId})`}
        ></rect>
      </g>
    </svg>
  </div>
{/if}
