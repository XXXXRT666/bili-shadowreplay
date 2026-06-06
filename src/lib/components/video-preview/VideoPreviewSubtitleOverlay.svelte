<script lang="ts">
  import { buildSubtitlePreviewTextShadow, type SubtitleStyle } from "../../interface";

  export let currentSubtitle = "";
  export let subtitleStyle: SubtitleStyle;
  export let previewDisplayHeight = 720;
  export let previewScaleBaseHeight = 720;
</script>

{#if currentSubtitle}
  <div
    class="absolute bottom-8 box-border px-4 py-2 rounded-lg text-white"
    style="
      left: {previewDisplayHeight *
        (subtitleStyle.marginL / previewScaleBaseHeight)}px;
      right: {previewDisplayHeight *
        (subtitleStyle.marginR / previewScaleBaseHeight)}px;
      font-family: {subtitleStyle.fontName};
      font-size: {previewDisplayHeight *
        (subtitleStyle.fontSize / previewScaleBaseHeight)}px;
      font-weight: {subtitleStyle.fontWeight};
      color: {subtitleStyle.fontColor};
      text-shadow: {buildSubtitlePreviewTextShadow(
        subtitleStyle.outlineWidth,
        subtitleStyle.outlineColor,
      )};
      text-align: {subtitleStyle.alignment === 1
        ? 'left'
        : subtitleStyle.alignment === 2
          ? 'center'
          : 'right'};
      margin-bottom: {previewDisplayHeight *
        (subtitleStyle.marginV / previewScaleBaseHeight)}px;
      white-space: pre-wrap;
      overflow-wrap: break-word;
    "
  >
    {currentSubtitle}
  </div>
{/if}
