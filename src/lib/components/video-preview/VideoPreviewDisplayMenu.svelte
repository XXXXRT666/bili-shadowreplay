<script lang="ts">
  type HoverMenuController = {
    open: () => void;
    scheduleClose: () => void;
  };

  type PreviewDisplayMetrics = {
    top: number;
    left: number;
    width: number;
    height: number;
  };

  const previewSubtitleTimelineToggleId = `preview-subtitle-timeline-${Math.random()
    .toString(36)
    .slice(2)}`;
  const previewWaveformToggleId = `preview-waveform-${Math.random()
    .toString(36)
    .slice(2)}`;
  const previewPbpToggleId = `preview-pbp-${Math.random().toString(36).slice(2)}`;

  export let visible = false;
  export let hiddenDuringPanelTransition = false;
  export let element: HTMLDivElement | null = null;
  export let interactionLocked = false;
  export let getMetrics: (() => PreviewDisplayMetrics | null) | undefined =
    undefined;
  export let width = 156;
  export let height = 108;
  export let open: (() => void) | undefined = undefined;
  export let controller: HoverMenuController;
  export let showSubtitleTimeline = false;
  export let toggleSubtitleTimeline: (() => void | Promise<void>) | undefined =
    undefined;
  export let showWaveform = false;
  export let toggleWaveform: (() => void | Promise<void>) | undefined = undefined;
  export let showPbpOverlay = false;
  export let togglePbpOverlay: (() => void | Promise<void>) | undefined =
    undefined;
</script>

{#if visible && !hiddenDuringPanelTransition}
  <div
    bind:this={element}
    class="bpx-player-ctrl-setting-box preview-display-menu preview-display-menu--floating z-[85]"
    class:pointer-events-auto={!interactionLocked}
    class:pointer-events-none={interactionLocked}
    style:top={`${getMetrics?.()?.top ?? 0}px`}
    style:left={`${getMetrics?.()?.left ?? 0}px`}
    style:width={`${getMetrics?.()?.width ?? width}px`}
    style:height={`${getMetrics?.()?.height ?? height}px`}
    on:mouseenter={open}
    on:mouseleave={controller.scheduleClose}
  >
    <div class="bpx-player-ctrl-setting-menu">
      <div class="preview-display-menu-item bui bui-switch">
        <div class="bui-area">
          <input
            id={previewSubtitleTimelineToggleId}
            class="bui-switch-input"
            type="checkbox"
            aria-label="字幕轴"
            checked={showSubtitleTimeline}
            on:change={toggleSubtitleTimeline}
          />
          <label class="bui-switch-label" for={previewSubtitleTimelineToggleId}>
            <span class="bui-switch-name">字幕轴</span>
            <span class="bui-switch-body">
              <span class="bui-switch-dot">
                <span></span>
              </span>
            </span>
          </label>
        </div>
      </div>

      <div class="preview-display-menu-item bui bui-switch">
        <div class="bui-area">
          <input
            id={previewWaveformToggleId}
            class="bui-switch-input"
            type="checkbox"
            aria-label="波形"
            checked={showWaveform}
            on:change={toggleWaveform}
          />
          <label class="bui-switch-label" for={previewWaveformToggleId}>
            <span class="bui-switch-name">波形</span>
            <span class="bui-switch-body">
              <span class="bui-switch-dot">
                <span></span>
              </span>
            </span>
          </label>
        </div>
      </div>

      <div class="preview-display-menu-item bui bui-switch">
        <div class="bui-area">
          <input
            id={previewPbpToggleId}
            class="bui-switch-input"
            type="checkbox"
            aria-label="高能进度条"
            checked={showPbpOverlay}
            on:change={togglePbpOverlay}
          />
          <label class="bui-switch-label" for={previewPbpToggleId}>
            <span class="bui-switch-name">高能进度条</span>
            <span class="bui-switch-body">
              <span class="bui-switch-dot">
                <span></span>
              </span>
            </span>
          </label>
        </div>
      </div>
    </div>
  </div>
{/if}
