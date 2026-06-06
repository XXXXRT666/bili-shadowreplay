import { convertFileSrc } from "@tauri-apps/api/core";
import type { DanmakuEmoteMap, DanmakuStyle } from "./interface";
import { invoke, log, TAURI_ENV } from "./invoker";

export const DANMAKU_EMOTE_STATIC_BASE = "/danmaku-emotes";
export const DANMAKU_LINE_HEIGHT = 1.125;
export const DANMAKU_FONT_SIZE_PX = 25;
export const DANMAKU_FONT_SCALE = 1;
export const DANMAKU_FONT_FAMILY =
  'SimHei, "Microsoft JhengHei", Arial, Helvetica, sans-serif';
export const DANMAKU_FONT_WEIGHT = "bold";
export const DANMAKU_EMOTE_SCALE = 1;
export const DANMAKU_EMOTE_VERTICAL_OFFSET_EM = 0;
export const DANMAKU_EMOTE_TEXT_GAP_EM = 0.1;
export const DANMAKU_OPACITY = 1;
export const DANMAKU_DURATION_SECONDS = 9;
export const DANMAKU_TEXT_SHADOW_OUTLINE_PX = 0.5;
export const DANMAKU_TEXT_SHADOW_COLOR = "#000000";

export type DanmakuSegment =
  | {
      kind: "text";
      text: string;
    }
  | {
      kind: "emote";
      token: string;
      src: string;
    };

export type DanmakuEmoteRenderKind = "none" | "mixed" | "emote-only";

export function buildDanmakuTextShadow(
  outlineWidth: number,
  outlineColor: string
): string {
  if (outlineWidth <= 0) {
    return "none";
  }

  return `
    ${outlineWidth}px 0 0 ${outlineColor},
    -${outlineWidth}px 0 0 ${outlineColor},
    0 ${outlineWidth}px 0 ${outlineColor},
    0 -${outlineWidth}px 0 ${outlineColor},
    ${outlineWidth}px ${outlineWidth}px 0 ${outlineColor},
    -${outlineWidth}px ${outlineWidth}px 0 ${outlineColor},
    ${outlineWidth}px -${outlineWidth}px 0 ${outlineColor},
    -${outlineWidth}px -${outlineWidth}px 0 ${outlineColor}
  `;
}

export const DANMAKU_TEXT_SHADOW = buildDanmakuTextShadow(
  DANMAKU_TEXT_SHADOW_OUTLINE_PX,
  DANMAKU_TEXT_SHADOW_COLOR
);

export function defaultDanmakuStyle(): DanmakuStyle {
  return {
    fontScale: DANMAKU_FONT_SCALE,
    opacity: DANMAKU_OPACITY,
    displayArea: 100,
    speedPreset: 2,
    maxOnScreen: -1,
    bold: true,
    fontFamily: DANMAKU_FONT_FAMILY,
  };
}

export function loadDanmakuStyle(roomId: string): DanmakuStyle {
  if (typeof window === "undefined" || !roomId) {
    return defaultDanmakuStyle();
  }

  const savedStyle = window.localStorage.getItem(`danmaku_style_${roomId}`);
  if (!savedStyle) {
    return defaultDanmakuStyle();
  }

  try {
    return {
      ...defaultDanmakuStyle(),
      ...JSON.parse(savedStyle),
    };
  } catch (error) {
    log.warn("failed to parse danmaku style", { roomId, error });
    return defaultDanmakuStyle();
  }
}

export function saveDanmakuStyle(roomId: string, style: DanmakuStyle) {
  if (typeof window === "undefined" || !roomId) {
    return;
  }

  window.localStorage.setItem(`danmaku_style_${roomId}`, JSON.stringify(style));
}

function getFileName(path: string): string {
  return path.replace(/\\/g, "/").split("/").pop() || "";
}

function getPngStem(fileName: string): string | null {
  const match = fileName.match(/^(.*)\.png$/i);
  if (!match || !match[1]) {
    return null;
  }

  return match[1];
}

export async function loadDanmakuEmoteMap(): Promise<DanmakuEmoteMap> {
  let files: string[] = [];
  try {
    files = (await invoke("get_danmaku_emote_files")) as string[];
  } catch (error) {
    log.warn("failed to load danmaku emotes", {
      tauri: TAURI_ENV,
      source: "backend",
      error,
    });
    return {};
  }

  log.debug("loading danmaku emotes", {
    source: "backend",
    fileCount: files.length,
  });

  const emoteMap: DanmakuEmoteMap = {};

  for (const file of files) {
    const fileName = getFileName(file);
    const stem = getPngStem(fileName);
    if (!stem) {
      continue;
    }

    emoteMap[`[${stem}]`] = TAURI_ENV
      ? convertFileSrc(file)
      : `${DANMAKU_EMOTE_STATIC_BASE}/${encodeURIComponent(fileName)}`;
  }

  log.debug("danmaku emote map ready", {
    count: Object.keys(emoteMap).length,
  });

  return emoteMap;
}

export function parseDanmakuContent(
  content: string,
  emoteMap: DanmakuEmoteMap
): DanmakuSegment[] {
  if (!content) {
    return [];
  }

  const segments: DanmakuSegment[] = [];
  const tokenPattern = /\[[^\][]+?\]/g;
  let lastIndex = 0;

  for (const match of content.matchAll(tokenPattern)) {
    const token = match[0];
    const index = match.index ?? 0;

    if (index > lastIndex) {
      segments.push({
        kind: "text",
        text: content.slice(lastIndex, index),
      });
    }

    const src = emoteMap[token];
    if (src) {
      segments.push({
        kind: "emote",
        token,
        src,
      });
    } else {
      segments.push({
        kind: "text",
        text: token,
      });
    }

    lastIndex = index + token.length;
  }

  if (lastIndex < content.length) {
    segments.push({
      kind: "text",
      text: content.slice(lastIndex),
    });
  }

  return segments;
}

export function getDanmakuEmoteRenderKind(
  segments: DanmakuSegment[]
): DanmakuEmoteRenderKind {
  let hasEmote = false;
  let hasVisibleText = false;

  for (const segment of segments) {
    if (segment.kind === "emote") {
      hasEmote = true;
      continue;
    }

    if (segment.text.trim().length > 0) {
      hasVisibleText = true;
    }
  }

  if (!hasEmote) {
    return "none";
  }

  return hasVisibleText ? "mixed" : "emote-only";
}

function segmentHasVisibleText(segment: DanmakuSegment | undefined): boolean {
  return !!segment && segment.kind === "text" && segment.text.trim().length > 0;
}

export function getDanmakuEmoteTextGap(
  segments: DanmakuSegment[],
  index: number
): { marginLeftEm: number; marginRightEm: number } {
  return {
    marginLeftEm: segmentHasVisibleText(segments[index - 1])
      ? DANMAKU_EMOTE_TEXT_GAP_EM
      : 0,
    marginRightEm: segmentHasVisibleText(segments[index + 1])
      ? DANMAKU_EMOTE_TEXT_GAP_EM
      : 0,
  };
}
