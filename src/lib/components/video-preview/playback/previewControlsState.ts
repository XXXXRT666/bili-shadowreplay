export interface HoverMenuControllerOptions {
  getTimer: () => number | null;
  setTimer: (value: number | null) => void;
  setVisible: (value: boolean) => void;
  canOpen?: () => boolean;
  closeDelayMs?: number;
  onBeforeOpen?: () => void;
}

export interface HoverMenuController {
  cancelHide: () => void;
  close: () => void;
  open: () => void;
  scheduleClose: () => void;
}

export interface MenuRect {
  top: number;
  left: number;
  width: number;
  height: number;
}

export interface ValueLabelOption<T extends string = string> {
  value: T;
  label: string;
}

export function createHoverMenuController(
  options: HoverMenuControllerOptions
): HoverMenuController {
  const closeDelayMs = options.closeDelayMs ?? 100;

  const cancelHide = () => {
    const timer = options.getTimer();
    if (timer !== null && typeof window !== "undefined") {
      window.clearTimeout(timer);
      options.setTimer(null);
    }
  };

  const close = () => {
    cancelHide();
    options.setVisible(false);
  };

  const open = () => {
    if (options.canOpen && !options.canOpen()) {
      return;
    }
    cancelHide();
    options.onBeforeOpen?.();
    options.setVisible(true);
  };

  const scheduleClose = () => {
    cancelHide();

    if (typeof window === "undefined") {
      options.setVisible(false);
      return;
    }

    options.setTimer(
      window.setTimeout(() => {
        options.setTimer(null);
        options.setVisible(false);
      }, closeDelayMs)
    );
  };

  return {
    cancelHide,
    close,
    open,
    scheduleClose,
  };
}

export function captureElementRect(element: HTMLElement | null): MenuRect | null {
  if (!element) {
    return null;
  }

  const rect = element.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) {
    return null;
  }

  return {
    top: rect.top,
    left: rect.left,
    width: rect.width,
    height: rect.height,
  };
}

export function resolveAnchoredMenuMetrics(options: {
  locked: boolean;
  lockedRect: MenuRect | null;
  anchor: HTMLElement | null;
  width: number;
  height: number;
  gap?: number;
}): MenuRect | null {
  if (options.locked && options.lockedRect) {
    return options.lockedRect;
  }

  if (!options.anchor) {
    return null;
  }

  const rect = options.anchor.getBoundingClientRect();
  const gap = options.gap ?? 14;

  return {
    top: rect.top - gap - options.height,
    left: rect.left + rect.width / 2 - options.width / 2,
    width: options.width,
    height: options.height,
  };
}

export function formatPlaybackRateLabel(
  rate: number,
  allowDefaultLabel = true
): string {
  if (Math.abs(rate - 1) < 0.001 && allowDefaultLabel) {
    return "倍速";
  }

  if (Math.abs(rate - Math.round(rate)) < 0.001) {
    return `${rate.toFixed(1)}x`;
  }

  return `${rate.toFixed(2).replace(/\.?0+$/, "")}x`;
}

export function computePlaybackRateButtonLabel(options: {
  selectedRate: number;
  runtimeRate: number;
  isFastForwarding: boolean;
  fastForwardPlaybackRate: number;
}): string {
  if (options.isFastForwarding) {
    return formatPlaybackRateLabel(options.fastForwardPlaybackRate, false);
  }

  if (Math.abs(options.selectedRate - 1) > 0.001) {
    return formatPlaybackRateLabel(options.selectedRate, false);
  }

  if (Math.abs(options.runtimeRate - 1) > 0.001) {
    return formatPlaybackRateLabel(options.runtimeRate, false);
  }

  return "倍速";
}

export function normalizeDanmuFontValue(fontFamily: string): string {
  const normalized = fontFamily.trim().toLowerCase();
  if (normalized.includes("simhei")) {
    return "__simhei__";
  }
  return normalized;
}

export function resolveDanmuFontOptionLabel(
  fontFamily: string,
  options: ReadonlyArray<ValueLabelOption<string>>
): string {
  const normalizedValue = normalizeDanmuFontValue(fontFamily);
  const matchedOption = options.find(
    (option) => normalizeDanmuFontValue(option.value) === normalizedValue
  );
  return matchedOption?.label ?? "自定义字体";
}

export function isDanmuFontValueSelected(
  currentFontFamily: string,
  optionValue: string
): boolean {
  return (
    normalizeDanmuFontValue(currentFontFamily) ===
    normalizeDanmuFontValue(optionValue)
  );
}

export function resolveOptionLabel<T extends string>(
  value: T,
  options: ReadonlyArray<ValueLabelOption<T>>,
  fallbackLabel: string
): string {
  const matchedOption = options.find((option) => option.value === value);
  return matchedOption?.label ?? fallbackLabel;
}
