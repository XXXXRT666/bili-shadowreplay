export interface SubtitleItem {
  startTime: number;
  endTime: number;
  text: string;
}

function sortSubtitles(subtitles: SubtitleItem[]): SubtitleItem[] {
  return [...subtitles].sort((a, b) => a.startTime - b.startTime);
}

export function parseSrtTime(time: string): number {
  const timeParts = time.split(",");
  if (timeParts.length !== 2) {
    console.warn("Invalid time format (missing comma):", time);
    return 0;
  }

  const timeWithoutMs = timeParts[0];
  const millisecondsStr = timeParts[1];
  const parts = timeWithoutMs.split(":");
  if (parts.length !== 3) {
    console.warn("Invalid time format:", time);
    return 0;
  }

  const [hours, minutes, seconds] = parts;
  const hoursNum = parseInt(hours, 10);
  const minutesNum = parseInt(minutes, 10);
  const secondsNum = parseInt(seconds, 10);
  const millisecondsNum = parseInt(millisecondsStr.padEnd(3, "0"), 10);

  if (
    Number.isNaN(hoursNum) ||
    Number.isNaN(minutesNum) ||
    Number.isNaN(secondsNum) ||
    Number.isNaN(millisecondsNum)
  ) {
    console.warn("Invalid time values:", time);
    return 0;
  }

  return (
    hoursNum * 3600 + minutesNum * 60 + secondsNum + millisecondsNum / 1000
  );
}

export function formatSrtTime(time: number): string {
  const hours = Math.floor(time / 3600);
  const minutes = Math.floor((time % 3600) / 60);
  const seconds = Math.floor(time % 60);
  const milliseconds = Math.floor((time % 1) * 1000);
  return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")},${milliseconds.toString().padStart(3, "0")}`;
}

export function srtToSubtitles(srt: string): SubtitleItem[] {
  if (!srt.trim()) {
    return [];
  }

  return srt
    .split(/\n\s*\n/)
    .map((block) => {
      const lines = block.split("\n").filter((line) => line.trim());
      if (lines.length < 3) {
        return null;
      }

      const timeLine = lines[1];
      const text = lines.slice(2).join("\n");
      const timeParts = timeLine.split(/\s*-->\s*/);
      if (timeParts.length !== 2) {
        console.warn("Invalid time line format:", timeLine);
        return null;
      }

      const startTime = parseSrtTime(timeParts[0].trim());
      const endTime = parseSrtTime(timeParts[1].trim());
      if (Number.isNaN(startTime) || Number.isNaN(endTime)) {
        console.warn("Failed to parse time values:", timeLine);
        return null;
      }

      return { startTime, endTime, text };
    })
    .filter((subtitle): subtitle is SubtitleItem => subtitle !== null)
    .sort((a, b) => a.startTime - b.startTime);
}

export function subtitlesToSrt(subtitles: SubtitleItem[]): string {
  return subtitles
    .map(
      (subtitle, index) =>
        `${index + 1}\n${formatSrtTime(subtitle.startTime)} --> ${formatSrtTime(subtitle.endTime)}\n${subtitle.text}\n`
    )
    .join("\n");
}

export async function saveSubtitles(options: {
  videoFile?: string | null;
  videoId?: number | null;
  subtitles: SubtitleItem[];
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
}): Promise<void> {
  if (!options.videoFile || options.videoId == null) {
    return;
  }

  try {
    await options.invoke("update_video_subtitle", {
      id: options.videoId,
      subtitle: subtitlesToSrt(options.subtitles),
    });
  } catch (error) {
    console.warn(error);
  }
}

export async function loadSubtitles(options: {
  videoFile?: string | null;
  videoId?: number | null;
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
}): Promise<SubtitleItem[]> {
  if (!options.videoFile || options.videoId == null) {
    return [];
  }

  const savedSubtitles = (await options.invoke("get_video_subtitle", {
    id: options.videoId,
  })) as string;
  return savedSubtitles ? srtToSubtitles(savedSubtitles) : [];
}

export async function generateSubtitles(options: {
  videoFile?: string | null;
  videoId?: number | null;
  generateEventId: () => string;
  listen: (
    event: string,
    handler: (payload: any) => void
  ) => Promise<() => void>;
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
  setCurrentGenerateEventId: (value: string | null) => void;
  updateGeneratePrompt: (content: string) => void;
  reportError: (message: string) => void;
}): Promise<SubtitleItem[]> {
  if (!options.videoFile || options.videoId == null) {
    return [];
  }

  const eventId = options.generateEventId();
  options.setCurrentGenerateEventId(eventId);

  const clearUpdateListener = await options.listen(
    `progress-update:${eventId}`,
    (e) => {
      options.updateGeneratePrompt(e.payload.content);
    }
  );

  const clearFinishedListener = await options.listen(
    `progress-finished:${eventId}`,
    (e) => {
      options.updateGeneratePrompt("AI 生成字幕");
      if (!e.payload.success) {
        options.reportError(`生成字幕失败: ${e.payload.message}`);
      }
      options.setCurrentGenerateEventId(null);
      clearUpdateListener();
      clearFinishedListener();
    }
  );

  const savedSubtitles = (await options.invoke("generate_video_subtitle", {
    eventId,
    id: options.videoId,
  })) as string;

  return srtToSubtitles(savedSubtitles);
}

export function createSubtitleAtTime(options: {
  subtitles: SubtitleItem[];
  currentTime: number;
  duration: number;
}): SubtitleItem[] {
  const nextSubtitles = [
    ...options.subtitles,
    {
      startTime: options.currentTime,
      endTime: Math.min(options.currentTime + 5, options.duration),
      text: "",
    },
  ];
  return sortSubtitles(nextSubtitles);
}

export function updateSubtitleTime(options: {
  subtitles: SubtitleItem[];
  index: number;
  isStart: boolean;
  time: number;
  duration: number;
}): SubtitleItem[] {
  return sortSubtitles(
    options.subtitles.map((sub, i) => {
      if (i !== options.index) {
        return sub;
      }
      if (options.isStart) {
        return {
          ...sub,
          startTime: Math.max(0, Math.min(options.time, sub.endTime - 0.1)),
        };
      }
      return {
        ...sub,
        endTime: Math.min(
          options.duration,
          Math.max(options.time, sub.startTime + 0.1)
        ),
      };
    })
  );
}

export function moveSubtitle(options: {
  subtitles: SubtitleItem[];
  index: number;
  newStartTime: number;
  duration: number;
}): SubtitleItem[] {
  const subtitle = options.subtitles[options.index];
  if (!subtitle) {
    return options.subtitles;
  }

  const subtitleDuration = subtitle.endTime - subtitle.startTime;
  const finalStartTime = Math.max(
    0,
    Math.min(options.newStartTime, options.duration - subtitleDuration)
  );
  const finalEndTime = finalStartTime + subtitleDuration;

  return sortSubtitles(
    options.subtitles.map((item, i) =>
      i === options.index
        ? { ...item, startTime: finalStartTime, endTime: finalEndTime }
        : item
    )
  );
}

export function removeSubtitle(subtitles: SubtitleItem[], index: number): SubtitleItem[] {
  return subtitles.filter((_, i) => i !== index);
}

export function clearSubtitles(): SubtitleItem[] {
  return [];
}

export function adjustSubtitleTime(options: {
  subtitles: SubtitleItem[];
  index: number;
  isStart: boolean;
  delta: number;
  duration: number;
}): SubtitleItem[] {
  const subtitle = options.subtitles[options.index];
  if (!subtitle) {
    return options.subtitles;
  }

  if (options.isStart) {
    const newTime = Math.max(0, subtitle.startTime + options.delta);
    if (newTime >= subtitle.endTime - 0.1) {
      return options.subtitles;
    }
    return sortSubtitles(
      options.subtitles.map((item, i) =>
        i === options.index ? { ...item, startTime: newTime } : item
      )
    );
  }

  const newTime = Math.min(options.duration, subtitle.endTime + options.delta);
  if (newTime <= subtitle.startTime + 0.1) {
    return options.subtitles;
  }
  return sortSubtitles(
    options.subtitles.map((item, i) =>
      i === options.index ? { ...item, endTime: newTime } : item
    )
  );
}

export function getSubtitleStyle(options: {
  subtitle: SubtitleItem;
  isVideoLoaded: boolean;
  duration: number;
}): string {
  if (!options.isVideoLoaded || options.duration <= 0) {
    return "";
  }
  const start = (options.subtitle.startTime / options.duration) * 100;
  const width =
    ((options.subtitle.endTime - options.subtitle.startTime) / options.duration) *
    100;
  return `left: ${start}%; width: ${width}%;`;
}
