<script lang="ts">
  import VideoPreviewWaveformPanel from "./VideoPreviewWaveformPanel.svelte";
  import type { ClipSelection } from "./clip/clipSelectionRuntime";
  import type { SubtitleItem } from "./subtitle/subtitleRuntime";

  export let timelineContainer: HTMLElement | null = null;
  export let handleWheel: ((event: WheelEvent) => void) | undefined = undefined;
  export let handleTimelineScroll: (() => void) | undefined = undefined;
  export let waveformGestureElement: HTMLElement | null = null;
  export let waveformScale = 1;
  export let showWaveformLayoutVisible = false;
  export let waveformPanelHeightPx = 60;
  export let showWaveform = false;
  export let isWaveformLoading = false;
  export let waveformContainer: HTMLElement | null = null;
  export let timelineElement: HTMLElement | null = null;
  export let timelineScale = 1;
  export let showSubtitleTimelineLayoutVisible = false;
  export let scheduleTimelineRefresh: (() => void) | undefined = undefined;
  export let isDraggingSeekbar = false;
  export let handleTimelineClick:
    | ((event: MouseEvent) => void | Promise<void>)
    | undefined = undefined;
  export let canClip = false;
  export let clipSelections: ClipSelection[] = [];
  export let activeClipSelectionId: string | null = null;
  export let hasPendingClipStartMarker = false;
  export let pendingClipStartTime = 0;
  export let videoDuration = 0;
  export let seekbarElement: HTMLElement | null = null;
  export let seekbarProgressElement: HTMLElement | null = null;
  export let handleSeekbarMouseDown:
    | ((event: MouseEvent) => void | Promise<void>)
    | undefined = undefined;
  export let handleSeekbarMouseEnter: ((event: MouseEvent) => void) | undefined =
    undefined;
  export let handleSeekbarMouseHoverMove:
    | ((event: MouseEvent) => void)
    | undefined = undefined;
  export let handleSeekbarMouseLeave: (() => void) | undefined = undefined;
  export let isSeekbarTrackHovering = false;
  export let seekbarCurrentRatioValue = 0;
  export let isSeekbarHovering = false;
  export let seekbarCurrentXValue = 0;
  export let seekbarThumbSize = 22;
  export let showSeekbarMoveIndicatorValue = false;
  export let seekbarPointerXValue = 0;
  export let timeMarkers: number[] = [];
  export let formatTimelineMarkerTime:
    | ((time: number) => string)
    | undefined = undefined;
  export let showSubtitleTimelineEffective = false;
  export let subtitles: SubtitleItem[] = [];
  export let getSubtitleStyle: ((subtitle: SubtitleItem) => string) | undefined =
    undefined;
  export let handleBlockMouseDown:
    | ((event: MouseEvent, index: number) => void)
    | undefined = undefined;
  export let handleTimelineMouseDown:
    | ((event: MouseEvent, index: number, isStart: boolean) => void)
    | undefined = undefined;
</script>

<div class="relative z-40 bg-[#1c1c1e]">
  <div class="pointer-events-none absolute inset-0 bg-[#1c1c1e]"></div>
  <div
    class="relative z-10 flex min-w-0 flex-col overflow-x-scroll overflow-y-visible sidebar-scrollbar clip-timeline-scroll-host"
    bind:this={timelineContainer}
    on:wheel|preventDefault={handleWheel}
    on:scroll={handleTimelineScroll}
  >
    <VideoPreviewWaveformPanel
      bind:gestureElement={waveformGestureElement}
      {waveformScale}
      visible={showWaveformLayoutVisible}
      heightPx={waveformPanelHeightPx}
      loading={showWaveform && isWaveformLoading}
      bind:container={waveformContainer}
      onWheel={handleWheel}
    />

    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
      bind:this={timelineElement}
      class="relative group shrink-0"
      style={`width: ${100 * timelineScale}%; height: ${showSubtitleTimelineLayoutVisible ? "60px" : "44px"};`}
      on:mouseenter={scheduleTimelineRefresh}
      on:click|preventDefault|stopPropagation={(event) => {
        if (!isDraggingSeekbar) {
          handleTimelineClick?.(event);
        }
      }}
    >
      {#if canClip && (clipSelections.length > 0 || hasPendingClipStartMarker)}
        <div
          class="absolute top-0 left-0 right-0 h-1 group-hover:h-1.5 transition-all duration-200 z-15"
        >
          {#each clipSelections as clipSelection (clipSelection.id)}
            <div
              class={`absolute h-full transition-all duration-200 ${
                clipSelection.id === activeClipSelectionId
                  ? "bg-green-400/85"
                  : "bg-green-400/50"
              }`}
              style="left: {(clipSelection.startTime / (videoDuration || 1)) *
                100}%; right: {100 -
                (clipSelection.endTime / (videoDuration || 1)) * 100}%"
            ></div>
            <div
              class={`absolute h-full w-0.5 transition-all duration-200 ${
                clipSelection.id === activeClipSelectionId
                  ? "bg-green-200"
                  : "bg-green-500"
              }`}
              style="left: {(clipSelection.startTime / (videoDuration || 1)) *
                100}%"
            ></div>
            <div
              class={`absolute h-full w-0.5 transition-all duration-200 ${
                clipSelection.id === activeClipSelectionId
                  ? "bg-green-200"
                  : "bg-green-500"
              }`}
              style="left: {(clipSelection.endTime / (videoDuration || 1)) *
                100}%; transform: translateX(-100%)"
            ></div>
          {/each}

          {#if hasPendingClipStartMarker}
            <div
              class="absolute h-full w-0.5 bg-green-300/90 transition-all duration-200"
              style="left: {(pendingClipStartTime / (videoDuration || 1)) * 100}%"
            ></div>
          {/if}
        </div>
      {/if}

      <div class="bpx-player-control-top bpx-player-control-top--timeline">
        <div
          bind:this={seekbarElement}
          class="bpx-player-progress-area"
          on:mousedown={handleSeekbarMouseDown}
          on:mouseenter={handleSeekbarMouseEnter}
          on:mousemove={handleSeekbarMouseHoverMove}
          on:mouseleave={handleSeekbarMouseLeave}
          on:click|preventDefault|stopPropagation
        >
          <div class="bpx-player-progress-freezone"></div>
          <div class="bpx-player-progress-wrap">
            <div
              bind:this={seekbarProgressElement}
              class="bpx-player-progress"
              class:is-active={isSeekbarTrackHovering}
              class:is-dragging={isDraggingSeekbar}
            >
              <div class="bpx-player-progress-schedule-wrap">
                <div class="bpx-player-progress-schedule">
                  <div
                    class="bpx-player-progress-schedule-buffer bpx-player-progress-schedule-buffer--hidden"
                    style="transform: scaleX(0);"
                  ></div>
                  <div
                    class="bpx-player-progress-schedule-current"
                    style={`transform: scaleX(${seekbarCurrentRatioValue});`}
                  ></div>
                </div>
              </div>
              <div class="bpx-player-progress-point-wrap"></div>
              <div
                class="bpx-player-progress-thumb"
                class:is-active={isSeekbarHovering || isDraggingSeekbar}
                style={`transform: translateX(${seekbarCurrentXValue - seekbarThumbSize / 2}px);`}
              >
                <div class="bpx-player-progress-thumb-icon">
                  <svg
                    class="bpx-player-progress-thumb-svg"
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 18 18"
                    width="18"
                    height="18"
                    preserveAspectRatio="xMidYMid meet"
                    aria-hidden="true"
                  >
                    <g
                      transform="matrix(0.9883429408073425,-0.7275781631469727,0.6775955557823181,0.920446515083313,7.3224687576293945,-0.7606706619262695)"
                    >
                      <g
                        transform="matrix(0.9937776327133179,-0.11138220876455307,0.11138220876455307,0.9937776327133179,-2.5239999294281006,1.3849999904632568)"
                      >
                        <path
                          fill="rgb(51,51,51)"
                          fill-opacity="1"
                          d="M0.75,-1.25C0.75,-1.25 0.75,1.25 0.75,1.25C0.75,1.663925051689148 0.4139249920845032,2 0,2C0,2 0,2 0,2C-0.4139249920845032,2 -0.75,1.663925051689148 -0.75,1.25C-0.75,1.25 -0.75,-1.25 -0.75,-1.25C-0.75,-1.663925051689148 -0.4139249920845032,-2 0,-2C0,-2 0,-2 0,-2C0.4139249920845032,-2 0.75,-1.663925051689148 0.75,-1.25z"
                        ></path>
                      </g>
                    </g>
                    <g
                      transform="matrix(1.1436611413955688,0.7535901665687561,-0.6317168474197388,0.9587040543556213,16.0070743560791,2.902894973754883)"
                    >
                      <g
                        transform="matrix(0.992861807346344,0.1192704513669014,-0.1192704513669014,0.992861807346344,-2.5239999294281006,1.3849999904632568)"
                      >
                        <path
                          fill="rgb(51,51,51)"
                          fill-opacity="1"
                          d="M0.75,-1.25C0.75,-1.25 0.75,1.25 0.75,1.25C0.75,1.663925051689148 0.4139249920845032,2 0,2C0,2 0,2 0,2C-0.4139249920845032,2 -0.75,1.663925051689148 -0.75,1.25C-0.75,1.25 -0.75,-1.25 -0.75,-1.25C-0.75,-1.663925051689148 -0.4139249920845032,-2 0,-2C0,-2 0,-2 0,-2C0.4139249920845032,-2 0.75,-1.663925051689148 0.75,-1.25z"
                        ></path>
                      </g>
                    </g>
                    <g
                      transform="matrix(1,0,0,1,8.890999794006348,8.406000137329102)"
                    >
                      <g
                        transform="matrix(1,0,0,1,0.09099999815225601,1.1009999513626099)"
                      >
                        <path
                          fill="rgb(255,255,255)"
                          fill-opacity="1"
                          d="M7,-3C7,-3 7,3 7,3C7,4.379749774932861 5.879749774932861,5.5 4.5,5.5C4.5,5.5 -4.5,5.5 -4.5,5.5C-5.879749774932861,5.5 -7,4.379749774932861 -7,3C-7,3 -7,-3 -7,-3C-7,-4.379749774932861 -5.879749774932861,-5.5 -4.5,-5.5C-4.5,-5.5 4.5,-5.5 4.5,-5.5C5.879749774932861,-5.5 7,-4.379749774932861 7,-3z"
                        ></path>
                        <path
                          stroke-linecap="butt"
                          stroke-linejoin="miter"
                          fill-opacity="0"
                          stroke-miterlimit="4"
                          stroke="rgb(51,51,51)"
                          stroke-opacity="1"
                          stroke-width="1.5"
                          d="M7,-3C7,-3 7,3 7,3C7,4.379749774932861 5.879749774932861,5.5 4.5,5.5C4.5,5.5 -4.5,5.5 -4.5,5.5C-5.879749774932861,5.5 -7,4.379749774932861 -7,3C-7,3 -7,-3 -7,-3C-7,-4.379749774932861 -5.879749774932861,-5.5 -4.5,-5.5C-4.5,-5.5 4.5,-5.5 4.5,-5.5C5.879749774932861,-5.5 7,-4.379749774932861 7,-3z"
                        ></path>
                      </g>
                    </g>
                    <g
                      transform="matrix(1,0,0,1,8.89900016784668,8.083999633789062)"
                    >
                      <g
                        transform="matrix(1,0,0,1,-2.5239999294281006,1.3849999904632568)"
                      >
                        <path
                          fill="rgb(51,51,51)"
                          fill-opacity="1"
                          d="M0.875,-1.125C0.875,-1.125 0.875,1.125 0.875,1.125C0.875,1.607912540435791 0.48291251063346863,2 0,2C0,2 0,2 0,2C-0.48291251063346863,2 -0.875,1.607912540435791 -0.875,1.125C-0.875,1.125 -0.875,-1.125 -0.875,-1.125C-0.875,-1.607912540435791 -0.48291251063346863,-2 0,-2C0,-2 0,-2 0,-2C0.48291251063346863,-2 0.875,-1.607912540435791 0.875,-1.125z"
                        ></path>
                      </g>
                    </g>
                    <g
                      transform="matrix(1,0,0,1,14.008999824523926,8.083999633789062)"
                    >
                      <g
                        transform="matrix(1,0,0,1,-2.5239999294281006,1.3849999904632568)"
                      >
                        <path
                          fill="rgb(51,51,51)"
                          fill-opacity="1"
                          d="M0.8999999761581421,-1.100000023841858C0.8999999761581421,-1.100000023841858 0.8999999761581421,1.100000023841858 0.8999999761581421,1.100000023841858C0.8999999761581421,1.596709966659546 0.4967099726200104,2 0,2C0,2 0,2 0,2C-0.4967099726200104,2 -0.8999999761581421,1.596709966659546 -0.8999999761581421,1.100000023841858C-0.8999999761581421,1.100000023841858 -0.8999999761581421,-1.100000023841858 -0.8999999761581421,-1.100000023841858C-0.8999999761581421,-1.596709966659546 -0.4967099726200104,-2 0,-2C0,-2 0,-2 0,-2C0.4967099726200104,-2 0.8999999761581421,-1.596709966659546 0.8999999761581421,-1.100000023841858z"
                        ></path>
                      </g>
                    </g>
                  </svg>
                </div>
              </div>

              {#if showSeekbarMoveIndicatorValue}
                <div
                  class="bpx-player-progress-move-indicator"
                  style={`left: ${seekbarPointerXValue}px;`}
                >
                  <svg
                    class="bpx-player-progress-move-indicator-svg"
                    viewBox="0 0 18 18"
                    aria-hidden="true"
                  >
                    <polygon class="ql-stroke" points="5 2 9 6 13 2 5 2"></polygon>
                    <polygon
                      class="ql-stroke"
                      points="5 16 9 12 13 16 5 16"
                    ></polygon>
                  </svg>
                </div>
              {/if}

              <div
                class="bpx-player-progress-cursor"
                style={`transform: translateX(${seekbarPointerXValue}px);`}
              ></div>
              <div class="bpx-player-progress-scaleplate"></div>
            </div>
          </div>
        </div>
      </div>

      <div class="flex h-full flex-col pt-4">
        <div class="relative h-8 select-none pointer-events-none">
          <div class="absolute inset-y-0 left-3 right-3">
            {#each timeMarkers as time}
              <div
                class="absolute top-0 -translate-x-1/2 flex flex-col items-center gap-0.5"
                style="left: {(time / (videoDuration || 1)) * 100}%"
              >
                <div class="h-1.5 w-px bg-gray-500 rounded-full"></div>
                <div class="text-[11px] leading-none text-gray-400 whitespace-nowrap">
                  {formatTimelineMarkerTime?.(time)}
                </div>
              </div>
            {/each}
          </div>
        </div>

        {#if showSubtitleTimelineLayoutVisible}
          <div class="relative h-6 -mt-2">
            <div class="relative h-full mx-3">
              {#if showSubtitleTimelineEffective}
                {#each subtitles as subtitle, index}
                  <div
                    class="absolute top-0 bottom-0 bg-[#0A84FF]/20 border border-[#0A84FF]/35 rounded-md cursor-move"
                    style={getSubtitleStyle?.(subtitle)}
                    on:mousedown={(event) => handleBlockMouseDown?.(event, index)}
                  >
                    <div
                      class="absolute left-0 top-0 bottom-0 w-1 bg-[#0A84FF] rounded-l cursor-ew-resize"
                      on:mousedown|stopPropagation={(event) =>
                        handleTimelineMouseDown?.(event, index, true)}
                    />
                    <div
                      class="absolute right-0 top-0 bottom-0 w-1 bg-[#0A84FF] rounded-r cursor-ew-resize"
                      on:mousedown|stopPropagation={(event) =>
                        handleTimelineMouseDown?.(event, index, false)}
                    />
                    <div
                      class="absolute inset-x-1.5 inset-y-0.5 flex items-center justify-center text-[11px] text-white text-center line-clamp-1 rounded"
                    >
                      {subtitle.text || "空字幕"}
                    </div>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>
