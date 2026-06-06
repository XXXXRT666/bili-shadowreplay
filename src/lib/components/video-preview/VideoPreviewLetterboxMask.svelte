<script lang="ts">
  export let visible = false;
  export let top = 0;
  export let left = 0;
  export let width = 0;
  export let height = 0;
  export let zIndex = 0;
</script>

{#if visible && width > 0 && height > 0}
  <div
    class="pointer-events-none absolute inset-x-0 top-0 bg-black"
    style={`height:${top}px;z-index:${zIndex};`}
  />
  <div
    class="pointer-events-none absolute inset-x-0 bottom-0 bg-black"
    style={`top:${top + height}px;z-index:${zIndex};`}
  />
  <div
    class="pointer-events-none absolute left-0 bg-black"
    style={`top:${top}px;height:${height}px;width:${left}px;z-index:${zIndex};`}
  />
  <div
    class="pointer-events-none absolute right-0 bg-black"
    style={`top:${top}px;left:${left + width}px;height:${height}px;z-index:${zIndex};`}
  />
{/if}
