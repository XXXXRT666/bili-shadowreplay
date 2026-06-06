<script lang="ts">
  import BuiProgressControl from "../BuiProgressControl.svelte";
  import LottieIcon from "../LottieIcon.svelte";
  import playLottieData from "../../../assets/lottie/player-ctrl-play.json";
  import pauseLottieData from "../../../assets/lottie/player-ctrl-pause.json";
  import settingLottieData from "../../../assets/lottie/player-ctrl-setting-hover.json";
  import muteLottieData from "../../../assets/lottie/player-ctrl-muted.json";
  import volumeLottieData from "../../../assets/lottie/player-ctrl-volume.json";
  import muteHoverLottieData from "../../../assets/lottie/player-ctrl-muted-hover.json";
  import volumeHoverLottieData from "../../../assets/lottie/player-ctrl-volume-hover.json";
  import fullscreenHoverLottieData from "../../../assets/lottie/player-ctrl-full-hover.json";
  import webFullscreenEnterHoverLottieData from "../../../assets/lottie/player-ctrl-web-enter-hover.json";
  import webFullscreenExitHoverLottieData from "../../../assets/lottie/player-ctrl-web-leave-hover.json";
  import wideEnterHoverLottieData from "../../../assets/lottie/player-ctrl-wide-enter-hover.json";
  import wideExitHoverLottieData from "../../../assets/lottie/player-ctrl-wide-leave-hover.json";

  type HoverMenuController = {
    open: () => void;
    scheduleClose: () => void;
  };

  export let isPlaying = false;
  export let togglePlay: (() => void) | undefined = undefined;
  export let playIconSeed = 0;
  export let openTimeSeekInput: (() => void) | undefined = undefined;
  export let handleTimeSeekKeydown: ((event: KeyboardEvent) => void) | undefined =
    undefined;
  export let isTimeSeekEditing = false;
  export let timeSeekValue = "";
  export let timeSeekInput: HTMLInputElement | null = null;
  export let handleTimeSeekInputKeydown:
    | ((event: KeyboardEvent) => void | Promise<void>)
    | undefined = undefined;
  export let closeTimeSeekInput:
    | ((applyValue: boolean) => void | Promise<void>)
    | undefined = undefined;
  export let currentTime = 0;
  export let playbackDurationValue = 0;
  export let formatTimeForSeekInput: ((time: number) => string) | undefined =
    undefined;
  export let timelineZoomControlElement: HTMLElement | null = null;
  export let timelineSliderValue = 0;
  export let timelineZoomSteps = 0;
  export let timelineZoomNotchCount = 0;
  export let handleTimelineZoomControlWheel:
    | ((event: WheelEvent) => void)
    | undefined = undefined;
  export let handleScaleChange: (() => void) | undefined = undefined;
  export let handleScaleCommit: (() => void) | undefined = undefined;

  export let playbackRateTriggerElement: HTMLButtonElement | null = null;
  export let isPlaybackRateMenuVisible = false;
  export let canAdjustPlaybackRateValue = true;
  export let playbackRateMenuController: HoverMenuController;
  export let playbackRateDisplayLabelValue = "";
  export let playbackRateOptions: ReadonlyArray<{ value: number; label: string }> =
    [];
  export let getSelectedPlaybackRate: (() => number) | undefined = undefined;
  export let handlePlaybackRateSelect: ((nextRate: number) => void) | undefined =
    undefined;

  export let previewSettingsControlElement: HTMLDivElement | null = null;
  export let previewSettingsButtonElement: HTMLButtonElement | null = null;
  export let handleSettingsMouseEnter: (() => void) | undefined = undefined;
  export let handleSettingsMouseLeave: (() => void) | undefined = undefined;
  export let isSettingsHover = false;
  export let isPreviewDisplayMenuVisible = false;
  export let openPreviewDisplayMenu: (() => void) | undefined = undefined;
  export let previewDisplayMenuController: HoverMenuController;

  export let isVolumeMutedValue = false;
  export let isVolumeMenuVisible = false;
  export let volumeMenuController: HoverMenuController;
  export let toggleMute: (() => void) | undefined = undefined;
  export let isVolumeHover = false;
  export let isVolumeClickAnimating = false;
  export let isVolumeHoverSuppressed = false;
  export let volumeIconSeed = 0;
  export let effectiveVolumePercentValue = 100;
  export let volume = 1;
  export let handleVolumeChange: ((event: Event) => void) | undefined = undefined;
  export let handleVolumeMouseEnter: (() => void) | undefined = undefined;
  export let handleVolumeMouseLeave: (() => void) | undefined = undefined;

  export let isWindowWide = false;
  export let toggleWindowWide: (() => void | Promise<void>) | undefined = undefined;
  export let isWideHover = false;
  export let onWideHoverChange: ((value: boolean) => void) | undefined = undefined;

  export let isWebFullscreen = false;
  export let toggleWebFullscreen: (() => void) | undefined = undefined;
  export let isWebFullscreenHover = false;
  export let onWebFullscreenHoverChange:
    | ((value: boolean) => void)
    | undefined = undefined;

  export let isDocumentFullscreen = false;
  export let toggleFullscreen: (() => void | Promise<void>) | undefined =
    undefined;
  export let isFullscreenHover = false;
  export let onFullscreenHoverChange: ((value: boolean) => void) | undefined =
    undefined;
</script>

<div class="relative z-40 flex h-8 items-center gap-4">
  <div class="bpx-player-control-bottom min-w-0 flex-1 px-2">
    <div class="bpx-player-control-bottom-left">
      <button
        type="button"
        class="bpx-player-ctrl-btn bpx-player-ctrl-play"
        aria-label={isPlaying ? "暂停" : "播放"}
        on:click={togglePlay}
      >
        <span class="bpx-player-ctrl-btn-icon">
          {#key playIconSeed}
            <LottieIcon
              animationData={isPlaying ? playLottieData : pauseLottieData}
              autoplay={playIconSeed > 0}
              loop={false}
              initialProgress={1}
              className="lottie-icon lottie-icon--18"
            />
          {/key}
        </span>
      </button>

      <div
        class="bpx-player-ctrl-btn bpx-player-ctrl-time"
        role="button"
        tabindex="0"
        aria-label="跳转时间"
        on:click={openTimeSeekInput}
        on:keydown={handleTimeSeekKeydown}
      >
        <input
          class="bpx-player-ctrl-time-seek"
          class:is-hidden={!isTimeSeekEditing}
          bind:this={timeSeekInput}
          bind:value={timeSeekValue}
          on:keydown={handleTimeSeekInputKeydown}
          on:blur={() => closeTimeSeekInput?.(true)}
          on:click|stopPropagation
        />
        <div
          class="bpx-player-ctrl-time-label"
          class:is-hidden={isTimeSeekEditing}
        >
          <span class="bpx-player-ctrl-time-current">
            {formatTimeForSeekInput?.(Math.max(0, currentTime))}
          </span>
          <span class="bpx-player-ctrl-time-divide">/</span>
          <span class="bpx-player-ctrl-time-duration">
            {playbackDurationValue > 0
              ? formatTimeForSeekInput?.(playbackDurationValue)
              : "--:--"}
          </span>
        </div>
      </div>

      <div
        class="clip-timeline-zoom-control"
        bind:this={timelineZoomControlElement}
        on:wheel|preventDefault|stopPropagation={handleTimelineZoomControlWheel}
      >
        <BuiProgressControl
          title="缩放"
          bind:value={timelineSliderValue}
          min={0}
          max={timelineZoomSteps}
          step={1}
          displayValue=""
          showSteps={true}
          stepsCount={timelineZoomNotchCount}
          rootClass="clip-timeline-zoom-progress"
          contentClass="clip-timeline-zoom-progress-content"
          on:input={handleScaleChange}
          on:change={handleScaleCommit}
        />
      </div>
    </div>

    <div class="bpx-player-control-bottom-center"></div>

    <div class="bpx-player-control-bottom-right">
      <div class="relative z-[60] overflow-visible">
        <button
          bind:this={playbackRateTriggerElement}
          type="button"
          class="bpx-player-ctrl-btn bpx-player-ctrl-playbackrate"
          class:opacity-60={!canAdjustPlaybackRateValue}
          aria-label="选择播放速度"
          aria-expanded={isPlaybackRateMenuVisible}
          on:mouseenter={playbackRateMenuController.open}
          on:mouseleave={playbackRateMenuController.scheduleClose}
          on:focus={playbackRateMenuController.open}
          on:blur={playbackRateMenuController.scheduleClose}
        >
          <span class="bpx-player-ctrl-playbackrate-result">
            {playbackRateDisplayLabelValue}
          </span>
        </button>
        {#if isPlaybackRateMenuVisible && canAdjustPlaybackRateValue}
          <div
            class="bpx-player-ctrl-playbackrate-menu pointer-events-auto"
            on:mouseenter={playbackRateMenuController.open}
            on:mouseleave={playbackRateMenuController.scheduleClose}
          >
            {#each [...playbackRateOptions].reverse() as option}
              <button
                type="button"
                class="bpx-player-ctrl-playbackrate-menu-item"
                class:bpx-state-active={option.value === getSelectedPlaybackRate?.()}
                on:click={() => handlePlaybackRateSelect?.(option.value)}
              >
                {option.label}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <div
        bind:this={previewSettingsControlElement}
        class="relative z-[60]"
        on:mouseenter={handleSettingsMouseEnter}
        on:mouseleave={handleSettingsMouseLeave}
      >
        <button
          bind:this={previewSettingsButtonElement}
          type="button"
          class="bpx-player-ctrl-btn bpx-player-ctrl-setting"
          aria-label="预览显示设置"
          aria-expanded={isPreviewDisplayMenuVisible}
          on:focus={openPreviewDisplayMenu}
          on:blur={previewDisplayMenuController.scheduleClose}
        >
          <span class="bpx-player-ctrl-btn-icon">
            {#if isSettingsHover}
              <LottieIcon
                animationData={settingLottieData}
                autoplay={true}
                loop={false}
                className="lottie-icon lottie-icon--17"
              />
            {:else}
              <LottieIcon
                animationData={settingLottieData}
                autoplay={false}
                loop={false}
                initialProgress={0}
                className="lottie-icon lottie-icon--17"
              />
            {/if}
          </span>
        </button>
      </div>

      <div
        class="relative z-[60]"
        on:mouseenter={handleVolumeMouseEnter}
        on:mouseleave={handleVolumeMouseLeave}
      >
        <button
          type="button"
          class="bpx-player-ctrl-btn bpx-player-ctrl-volume"
          aria-label={isVolumeMutedValue ? "取消静音" : "静音"}
          aria-expanded={isVolumeMenuVisible}
          on:click={toggleMute}
          on:focus={volumeMenuController.open}
          on:blur={volumeMenuController.scheduleClose}
        >
          <span class="bpx-player-ctrl-btn-icon bpx-player-ctrl-volume-icon">
            {#if isVolumeHover && !isVolumeClickAnimating && !isVolumeHoverSuppressed}
              <LottieIcon
                animationData={isVolumeMutedValue
                  ? muteHoverLottieData
                  : volumeHoverLottieData}
                autoplay={true}
                loop={false}
                className="lottie-icon lottie-icon--17"
              />
            {:else}
              {#key volumeIconSeed}
                <LottieIcon
                  animationData={isVolumeMutedValue
                    ? muteLottieData
                    : volumeLottieData}
                  autoplay={isVolumeClickAnimating}
                  loop={false}
                  initialProgress={1}
                  className="lottie-icon lottie-icon--17"
                />
              {/key}
            {/if}
          </span>
        </button>

        {#if isVolumeMenuVisible}
          <div
            class="bpx-player-ctrl-volume-box"
            on:mouseenter={volumeMenuController.open}
            on:mouseleave={volumeMenuController.scheduleClose}
          >
            <div class="bpx-player-ctrl-volume-number">
              {effectiveVolumePercentValue}
            </div>
            <div class="bpx-player-ctrl-volume-progress">
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                bind:value={volume}
                on:input={handleVolumeChange}
                class="clip-volume-slider"
                style={`--volume-percent: ${effectiveVolumePercentValue}%;`}
                aria-label="调整音量"
              />
            </div>
          </div>
        {/if}
      </div>

      <button
        type="button"
        class="bpx-player-ctrl-btn bpx-player-ctrl-wide"
        aria-label={isWindowWide ? "退出宽屏" : "宽屏"}
        on:click={toggleWindowWide}
        on:mouseenter={() => onWideHoverChange?.(true)}
        on:mouseleave={() => onWideHoverChange?.(false)}
      >
        <span class="bpx-player-ctrl-btn-icon">
          {#if isWideHover}
            <LottieIcon
              animationData={isWindowWide
                ? wideExitHoverLottieData
                : wideEnterHoverLottieData}
              autoplay={true}
              loop={false}
              className="lottie-icon lottie-icon--17"
            />
          {:else}
            <LottieIcon
              animationData={isWindowWide
                ? wideExitHoverLottieData
                : wideEnterHoverLottieData}
              autoplay={false}
              loop={false}
              initialProgress={0}
              className="lottie-icon lottie-icon--17"
            />
          {/if}
        </span>
      </button>

      <button
        type="button"
        class="bpx-player-ctrl-btn bpx-player-ctrl-web"
        aria-label={isWebFullscreen ? "退出网页全屏" : "网页全屏"}
        on:click={toggleWebFullscreen}
        on:mouseenter={() => onWebFullscreenHoverChange?.(true)}
        on:mouseleave={() => onWebFullscreenHoverChange?.(false)}
      >
        <span class="bpx-player-ctrl-btn-icon">
          {#if isWebFullscreenHover}
            <LottieIcon
              animationData={isWebFullscreen
                ? webFullscreenExitHoverLottieData
                : webFullscreenEnterHoverLottieData}
              autoplay={true}
              loop={false}
              className="lottie-icon lottie-icon--17"
            />
          {:else}
            <LottieIcon
              animationData={isWebFullscreen
                ? webFullscreenExitHoverLottieData
                : webFullscreenEnterHoverLottieData}
              autoplay={false}
              loop={false}
              initialProgress={0}
              className="lottie-icon lottie-icon--17"
            />
          {/if}
        </span>
      </button>

      <button
        type="button"
        class="bpx-player-ctrl-btn bpx-player-ctrl-full"
        aria-label={isDocumentFullscreen ? "退出全屏" : "进入全屏"}
        on:click={toggleFullscreen}
        on:mouseenter={() => onFullscreenHoverChange?.(true)}
        on:mouseleave={() => onFullscreenHoverChange?.(false)}
      >
        <span class="bpx-player-ctrl-btn-icon">
          {#if isFullscreenHover}
            <LottieIcon
              animationData={fullscreenHoverLottieData}
              autoplay={true}
              loop={false}
              className="lottie-icon lottie-icon--17"
            />
          {:else}
            <LottieIcon
              animationData={fullscreenHoverLottieData}
              autoplay={false}
              loop={false}
              initialProgress={0}
              className="lottie-icon lottie-icon--17"
            />
          {/if}
        </span>
      </button>
    </div>
  </div>
</div>
