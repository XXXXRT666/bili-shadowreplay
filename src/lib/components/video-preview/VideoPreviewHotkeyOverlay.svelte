<script lang="ts">
  export let showDetail = false;
  export let style = "";
</script>

<div
  id="overlay"
  class="absolute z-[12] rounded-md px-2 py-2 flex flex-col pointer-events-none clip-preview-stable-overlay"
  {style}
>
  <p style="margin: 0;">
    快捷键说明
    <kbd
      style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
      >h</kbd
    >展开
  </p>
  {#if showDetail}
    <span>
      <p style="margin: 0;">
        <kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >[</kbd
        >设定选区开始
      </p>
      <p style="margin: 0;">
        <kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >]</kbd
        >设定选区结束
      </p>
      <p style="margin: 0;">
        <kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >q</kbd
        >跳转到选区开始
      </p>
      <p style="margin: 0;">
        <kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >e</kbd
        >跳转到选区结束
      </p>
      <p style="margin: 0;">
        <kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >g</kbd
        >生成切片
      </p>
      <p style="margin: 0;">
        <kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >c</kbd
        >清除选区
      </p>
      <p style="margin: 0;">
        <kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >Space</kbd
        >播放/暂停
      </p>
      <p style="margin: 0;">
        <kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >←</kbd
        ><kbd
          style="border: 1px solid white; padding: 0 0.2em; border-radius: 0.2em; margin: 4px;"
          >→</kbd
        >前进/后退
      </p>
    </span>
  {/if}
</div>
