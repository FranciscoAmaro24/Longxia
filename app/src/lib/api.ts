/**
 * Typed wrappers around the Rust commands. Keep every `invoke` behind a named
 * function with an explicit return type so call sites never pass raw command
 * strings and the TS/Rust contract stays in one place.
 */
import { invoke } from "@tauri-apps/api/core";

export interface Ring {
  key: string;
  zh: string;
  learned: number;
  target: number;
}

export interface TodaySummary {
  level: number;
  rings: Ring[];
  due: number;
  newCards: number;
}

export function getTodaySummary(): Promise<TodaySummary> {
  return invoke<TodaySummary>("get_today_summary");
}

export interface DictEntry {
  simplified: string;
  traditional: string | null;
  pinyin: string | null;
  gloss: string | null;
}

/** Look up a headword (usually a single tapped character). */
export function lookup(query: string): Promise<DictEntry[]> {
  return invoke<DictEntry[]>("lookup", { query });
}

export interface Annotated {
  text: string;
  pinyin: string | null;
}

/** Annotate a passage with per-character pinyin for ambient display. */
export function annotate(text: string): Promise<Annotated[]> {
  return invoke<Annotated[]>("annotate", { text });
}
