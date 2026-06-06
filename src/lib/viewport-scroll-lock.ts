function isScrollableOverflow(value: string) {
  return value === "auto" || value === "scroll" || value === "overlay";
}

function getNearestScrollableAncestor(
  start: HTMLElement | null,
  deltaX: number,
  deltaY: number
) {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return null;
  }

  let current: HTMLElement | null = start;
  while (current && current !== document.body) {
    const style = window.getComputedStyle(current);

    const canScrollY =
      isScrollableOverflow(style.overflowY) && current.scrollHeight > current.clientHeight + 1;
    const canScrollX =
      isScrollableOverflow(style.overflowX) && current.scrollWidth > current.clientWidth + 1;

    if (canScrollY || canScrollX) {
      const canContinueScrollY =
        deltaY < 0
          ? current.scrollTop > 0
          : deltaY > 0
            ? current.scrollTop + current.clientHeight < current.scrollHeight
            : false;
      const canContinueScrollX =
        deltaX < 0
          ? current.scrollLeft > 0
          : deltaX > 0
            ? current.scrollLeft + current.clientWidth < current.scrollWidth
            : false;

      if (canContinueScrollY || canContinueScrollX) {
        return current;
      }
    }

    current = current.parentElement;
  }

  return null;
}

export function installViewportScrollLock() {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return () => {};
  }

  const onWheel = (event: WheelEvent) => {
    if (event.defaultPrevented || event.ctrlKey) {
      return;
    }

    const target = event.target as HTMLElement | null;
    const scrollableAncestor = getNearestScrollableAncestor(
      target,
      event.deltaX,
      event.deltaY
    );

    if (!scrollableAncestor) {
      event.preventDefault();
    }
  };

  window.addEventListener("wheel", onWheel, {
    capture: true,
    passive: false,
  });

  return () => {
    window.removeEventListener("wheel", onWheel, {
      capture: true,
    });
  };
}
