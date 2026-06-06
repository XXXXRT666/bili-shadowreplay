<script lang="ts">
  export let gestureElement: HTMLElement | null = null;
  export let waveformScale = 1;
  export let visible = false;
  export let heightPx = 60;
  export let loading = false;
  export let container: HTMLElement | null = null;
  export let onWheel: ((event: WheelEvent) => void) | undefined = undefined;
</script>

<div
  bind:this={gestureElement}
  class="relative shrink-0"
  style={`width: ${100 * waveformScale}%; min-height: ${visible ? `${heightPx}px` : "0"}; height: ${visible ? `${heightPx}px` : "0"}; overflow: hidden;`}
  on:wheel|preventDefault|stopPropagation={onWheel}
>
  {#if visible && loading}
    <div
      class="absolute inset-0 flex items-center justify-center gap-2 text-gray-400 text-center bg-[#1c1c1e] z-20 pointer-events-none"
    >
      <svg
        class="animate-spin h-4 w-4"
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
      >
        <circle
          class="opacity-25"
          cx="12"
          cy="12"
          r="10"
          stroke="currentColor"
          stroke-width="4"
        ></circle>
        <path
          class="opacity-75"
          fill="currentColor"
          d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
        ></path>
      </svg>
      <span class="text-sm">加载音频波形...</span>
    </div>
  {/if}

  <div class="h-full mx-3" style="width: auto; height: 100%; min-height: 0;">
    <div
      bind:this={container}
      class="w-full h-full waveform-container"
      style="width: 100%; height: 100%; min-height: 0; overflow: hidden;"
      data-waveform-container
    ></div>
  </div>
</div>
