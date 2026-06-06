<script lang="ts">
  export let visible = false;
  export let left = 0;
  export let top = 0;
  export let imageSrc = "";
  export let transparentImage = "";
  export let time = 0;
  export let formatTime: ((time: number) => string) | undefined = undefined;
</script>

{#if visible}
  <div class="bpx-player-progress-popup-layer" style={`left: ${left}px; top: ${top}px;`}>
    <div
      class="bpx-player-progress-preview"
      class:bpx-player-progress-preview--transparent={!imageSrc}
    >
      <img
        class="bpx-player-progress-preview-image"
        class:bpx-player-progress-preview-image--transparent={!imageSrc}
        src={imageSrc || transparentImage}
        alt="预览缩略图"
      />
      <div class="bpx-player-progress-preview-time">
        {formatTime?.(time)}
      </div>
    </div>
    <div class="bpx-player-progress-hotspot"></div>
  </div>
{/if}
