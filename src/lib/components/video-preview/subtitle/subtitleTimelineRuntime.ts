import type { SubtitleItem } from "./subtitleRuntime";

interface TimelineAxisMetrics {
  left: number;
  width: number;
}

interface DraggingSubtitleState {
  index: number;
  isStart: boolean;
}

export function createSubtitleTimelineRuntime(options: {
  timelineAxisSideInsetPx: number;
  getTimelineElement: () => HTMLElement | null;
  getVideoDuration: () => number;
  getSubtitles: () => SubtitleItem[];
  getDraggingSubtitle: () => DraggingSubtitleState | null;
  setDraggingSubtitle: (value: DraggingSubtitleState | null) => void;
  getDraggingBlock: () => number | null;
  setDraggingBlock: (value: number | null) => void;
  getDragOffset: () => number;
  setDragOffset: (value: number) => void;
  updateSubtitleTime: (index: number, isStart: boolean, time: number) => void;
  moveSubtitle: (index: number, newStartTime: number) => void;
}) {
  const getTimelineAxisMetrics = (): TimelineAxisMetrics | null => {
    const timelineElement = options.getTimelineElement();
    if (!timelineElement) {
      return null;
    }
    const rect = timelineElement.getBoundingClientRect();
    const width = Math.max(
      1,
      rect.width - options.timelineAxisSideInsetPx * 2
    );
    return {
      left: rect.left + options.timelineAxisSideInsetPx,
      width,
    };
  };

  const getTimelineTimeFromClientX = (clientX: number, duration: number) => {
    const metrics = getTimelineAxisMetrics();
    if (!metrics || !Number.isFinite(duration) || duration <= 0) {
      return 0;
    }
    const x = Math.max(0, Math.min(clientX - metrics.left, metrics.width));
    return (x / metrics.width) * duration;
  };

  const handleTimelineMouseMove = (event: MouseEvent) => {
    const draggingSubtitle = options.getDraggingSubtitle();
    const duration = options.getVideoDuration();
    if (!draggingSubtitle || !Number.isFinite(duration) || duration <= 0) {
      return;
    }

    const time = getTimelineTimeFromClientX(event.clientX, duration);
    options.updateSubtitleTime(
      draggingSubtitle.index,
      draggingSubtitle.isStart,
      time
    );
  };

  const handleBlockMouseMove = (event: MouseEvent) => {
    const draggingBlock = options.getDraggingBlock();
    const duration = options.getVideoDuration();
    if (draggingBlock === null || !Number.isFinite(duration) || duration <= 0) {
      return;
    }

    const mouseTime = getTimelineTimeFromClientX(event.clientX, duration);
    const newStartTime = mouseTime - options.getDragOffset();
    options.moveSubtitle(draggingBlock, newStartTime);
  };

  const handleTimelineMouseUp = () => {
    options.setDraggingSubtitle(null);
    document.removeEventListener("mousemove", handleTimelineMouseMove);
    document.removeEventListener("mouseup", handleTimelineMouseUp);
  };

  const handleBlockMouseUp = () => {
    options.setDraggingBlock(null);
    document.removeEventListener("mousemove", handleBlockMouseMove);
    document.removeEventListener("mouseup", handleBlockMouseUp);
  };

  const startEdgeDragging = (index: number, isStart: boolean) => {
    options.setDraggingSubtitle({ index, isStart });
    document.addEventListener("mousemove", handleTimelineMouseMove);
    document.addEventListener("mouseup", handleTimelineMouseUp);
  };

  const startBlockDragging = (
    index: number,
    mouseTime: number,
    startTime: number
  ) => {
    options.setDraggingBlock(index);
    options.setDragOffset(mouseTime - startTime);
    document.addEventListener("mousemove", handleBlockMouseMove);
    document.addEventListener("mouseup", handleBlockMouseUp);
  };

  const handleTimelineMouseDown = (
    _event: MouseEvent,
    index: number,
    isStart: boolean
  ) => {
    startEdgeDragging(index, isStart);
  };

  const handleBlockMouseDown = (event: MouseEvent, index: number) => {
    const subtitles = options.getSubtitles();
    const subtitle = subtitles[index];
    const metrics = getTimelineAxisMetrics();
    const duration = options.getVideoDuration();
    if (!subtitle || !metrics || !Number.isFinite(duration) || duration <= 0) {
      return;
    }

    const x = Math.max(
      0,
      Math.min(event.clientX - metrics.left, metrics.width)
    );
    const mouseTime = (x / metrics.width) * duration;
    const blockWidth =
      metrics.width * ((subtitle.endTime - subtitle.startTime) / duration);
    const relativeX = x - metrics.width * (subtitle.startTime / duration);
    const edgeSize = Math.min(5, Math.max(2, blockWidth / 3));

    if (relativeX < edgeSize) {
      startEdgeDragging(index, true);
      return;
    }

    if (blockWidth > edgeSize * 2 && relativeX > blockWidth - edgeSize) {
      startEdgeDragging(index, false);
      return;
    }

    if (blockWidth <= edgeSize * 2 && relativeX > edgeSize) {
      startEdgeDragging(index, false);
      return;
    }

    startBlockDragging(index, mouseTime, subtitle.startTime);
  };

  const cleanup = () => {
    handleTimelineMouseUp();
    handleBlockMouseUp();
  };

  return {
    cleanup,
    getTimelineAxisMetrics,
    getTimelineTimeFromClientX,
    handleTimelineMouseDown,
    handleBlockMouseDown,
  };
}
