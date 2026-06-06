<script lang="ts">
  export let title = "";
  export let value = 0;
  export let min = 0;
  export let max = 100;
  export let step = 1;
  export let displayValue = "";
  export let disabled = false;
  export let showSteps = false;
  export let stepsCount = 0;
  export let rootClass = "";
  export let contentClass = "";

  function clamp(valueToClamp: number, lower: number, upper: number) {
    return Math.min(upper, Math.max(lower, valueToClamp));
  }

  $: normalizedMax = max > min ? max : min + 1;
  $: clampedValue = clamp(value, min, normalizedMax);
  $: progressPercent = ((clampedValue - min) / (normalizedMax - min)) * 100;
  $: markerCount = showSteps
    ? Math.max(2, stepsCount || Math.round((normalizedMax - min) / Math.max(step, 1)) + 1)
    : 0;
  $: markerIndices = Array.from({ length: markerCount }, (_, index) => index);
</script>

<div class={`bpx-player-dm-setting-progress bui bui-progress ${rootClass} ${disabled ? "is-disabled" : ""}`}>
  <div class="bpx-player-dm-setting-progress-title">{title}</div>
  <div class={`bpx-player-dm-setting-progress-content ${contentClass}`}>
    <div class="bui-area">
      <div
        class="bui-progress-wrap"
        style={`--progress-percent: ${progressPercent};`}
      >
        <div class="bui-progress-bar" style={`width: ${progressPercent}%;`}></div>
        <span class="bui-progress-dot"></span>
        {#if markerCount > 0}
          <div class="bui-progress-step">
            {#each markerIndices as markerIndex}
              <div
                class="bui-progress-item"
                style={`left: clamp(var(--bui-step-edge-inset), ${
                  (markerIndex / (markerCount - 1 || 1)) * 100
                }%, calc(100% - var(--bui-step-edge-inset)));`}
              >
                <div class="bui-progress-lab"></div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
      <input
        class="bui-progress-input"
        type="range"
        bind:value
        {min}
        max={normalizedMax}
        {step}
        {disabled}
        aria-label={title}
        on:input
        on:change
      />
    </div>
  </div>
  <div class="bui-progress-val">{displayValue}</div>
</div>

<style>
  .bpx-player-dm-setting-progress {
    display: flex;
    align-items: center;
    width: 100%;
    min-height: 24px;
    gap: 8px;
    cursor: default;
  }

  .bpx-player-dm-setting-progress-title {
    width: var(--bui-progress-title-width, 46px);
    flex: 0 0 auto;
    color: hsla(0, 0%, 100%, 0.8);
    font-size: 12px;
    line-height: 16px;
    user-select: none;
    white-space: nowrap;
  }

  .bpx-player-dm-setting-progress.is-title-wide {
    --bui-progress-title-width: 58px;
  }

  .bpx-player-dm-setting-progress-content {
    flex: 1;
    min-width: 0;
    height: 12px;
  }

  .bui-area {
    position: relative;
    width: 100%;
    height: 12px;
  }

  .bui-progress-wrap {
    --bui-thumb-size: 14px;
    --bui-thumb-radius: 7px;
    --bui-step-edge-inset: 4px;
    background-color: hsla(0, 0%, 100%, 0.2);
    border-radius: 6px;
    height: 6px;
    left: 0;
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
  }

  .bui-progress-bar {
    background-color: var(--bpx-primary-color, #00a1d6);
    border-radius: inherit;
    height: 100%;
    left: 0;
    position: absolute;
    top: 0;
    min-width: 0;
  }

  .bui-progress-dot {
    background-color: #fff;
    border-radius: 50%;
    height: var(--bui-thumb-size);
    position: absolute;
    left: clamp(
      var(--bui-thumb-radius),
      calc(var(--progress-percent) * 1%),
      calc(100% - var(--bui-thumb-radius))
    );
    top: 50%;
    transform: translate3d(-50%, -50%, 0);
    width: var(--bui-thumb-size);
    z-index: 4;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    pointer-events: none;
  }

  .bui-progress-step {
    position: absolute;
    inset: 0;
  }

  .bui-progress-item {
    height: 4px;
    left: 0;
    position: absolute;
    top: 50%;
    transform: translate3d(-50%, -50%, 0);
    white-space: nowrap;
    pointer-events: none;
  }

  .bui-progress-lab {
    background: #fff;
    border-radius: 100%;
    height: 4px;
    width: 4px;
    opacity: 0.9;
  }

  .bui-progress-input {
    appearance: none;
    background: transparent;
    cursor: pointer;
    height: 100%;
    inset: 0;
    margin: 0;
    opacity: 0;
    position: absolute;
    width: 100%;
  }

  .bui-progress-input:disabled {
    cursor: not-allowed;
  }

  .bui-progress-val {
    min-width: 44px;
    width: auto;
    flex: 0 0 auto;
    box-sizing: border-box;
    color: hsla(0, 0%, 100%, 0.8);
    cursor: default;
    font-size: 12px;
    line-height: 16px;
    padding-left: 0;
    text-align: right;
    user-select: none;
    text-wrap: nowrap;
  }

  .is-disabled {
    opacity: 0.45;
  }
</style>
