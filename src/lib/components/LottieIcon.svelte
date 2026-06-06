<script lang="ts">
  import lottie, { type AnimationItem } from "lottie-web";
  import { onDestroy, onMount } from "svelte";

  export let animationData: Record<string, unknown>;
  export let loop = false;
  export let autoplay = false;
  export let speed = 1;
  export let initialProgress: number | null = null;
  export let className = "";

  let container: HTMLDivElement | null = null;
  let animation: AnimationItem | null = null;

  function applyInitialProgress() {
    if (!animation || initialProgress === null || autoplay) {
      return;
    }
    const totalFrames = animation.getDuration(true);
    const targetFrame = Math.max(0, totalFrames * initialProgress);
    animation.goToAndStop(targetFrame, true);
  }

  onMount(() => {
    if (!container) {
      return;
    }
    animation = lottie.loadAnimation({
      container,
      renderer: "svg",
      loop,
      autoplay,
      animationData
    });
    animation.setSpeed(speed);
    applyInitialProgress();
  });

  $: if (animation) {
    animation.setSpeed(speed);
    applyInitialProgress();
  }

  onDestroy(() => {
    animation?.destroy();
    animation = null;
  });

  export function playFromStart() {
    if (!animation) {
      return;
    }
    animation.stop();
    animation.play();
  }
</script>

<div bind:this={container} class={className} aria-hidden="true"></div>
