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

export interface ReviewCard {
  id: number;
  headword: string;
  pinyin: string | null;
  gloss: string | null;
  /** Seconds until due for each rating. */
  again: number;
  hard: number;
  good: number;
  easy: number;
}

export interface ReviewResult {
  due: number;
  state: string;
}

/** Rating values match the Rust side: 1=Again, 2=Hard, 3=Good, 4=Easy. */
export type Rating = 1 | 2 | 3 | 4;

export function getReviewQueue(): Promise<ReviewCard[]> {
  return invoke<ReviewCard[]>("get_review_queue");
}

export function reviewCard(cardId: number, rating: Rating): Promise<ReviewResult> {
  return invoke<ReviewResult>("review_card", { cardId, rating });
}
