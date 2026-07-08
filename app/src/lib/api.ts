/**
 * Typed wrappers around the core operations. Each declares one `CallSpec` that
 * works on both hosts: the Tauri command (in the app) and the equivalent HTTP
 * request against `longxia-server` (in the browser). Call sites import these
 * named functions and never know which transport is active. The TS/Rust
 * contract stays in this one file; the dispatch lives in `transport.ts`.
 */
import { call } from "./transport";

export { isTauri, setApiToken, hasApiToken } from "./transport";

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
  /** Consecutive days (up to today) with at least one review. */
  streak: number;
}

export function getTodaySummary(): Promise<TodaySummary> {
  return call<TodaySummary>({
    command: "get_today_summary",
    http: { method: "GET", path: "/api/today" },
  });
}

export interface DictEntry {
  simplified: string;
  traditional: string | null;
  pinyin: string | null;
  gloss: string | null;
}

/** Look up a headword (usually a single tapped character). */
export function lookup(query: string): Promise<DictEntry[]> {
  return call<DictEntry[]>({
    command: "lookup",
    args: { query },
    http: { method: "GET", path: "/api/lookup", query: { q: query } },
  });
}

export interface Annotated {
  text: string;
  pinyin: string | null;
}

/** Annotate a passage with per-character pinyin for ambient display. */
export function annotate(text: string): Promise<Annotated[]> {
  return call<Annotated[]>({
    command: "annotate",
    args: { text },
    http: { method: "POST", path: "/api/annotate", body: { text } },
  });
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
  return call<ReviewCard[]>({
    command: "get_review_queue",
    http: { method: "GET", path: "/api/review/queue" },
  });
}

export function reviewCard(cardId: number, rating: Rating): Promise<ReviewResult> {
  return call<ReviewResult>({
    command: "review_card",
    args: { cardId, rating },
    http: { method: "POST", path: "/api/review", body: { cardId, rating } },
  });
}

// --- Notebook + AI insights ---

export interface Insight {
  id: number;
  snippet: string;
  explanation: string;
  start: number;
  end: number;
}

export interface Note {
  text: string;
  insights: Insight[];
}

/** Explain a span of Chinese text with Claude (red-pen insight). */
export function explain(text: string): Promise<string> {
  return call<string>({
    command: "explain",
    args: { text },
    http: { method: "POST", path: "/api/explain", body: { text } },
    // The command returns the string directly; the endpoint wraps it.
    fromHttp: (raw) => (raw as { explanation: string }).explanation,
  });
}

export function getNote(): Promise<Note> {
  return call<Note>({
    command: "get_note",
    http: { method: "GET", path: "/api/note" },
  });
}

export function saveNote(text: string): Promise<void> {
  return call<void>({
    command: "save_note",
    args: { text },
    http: { method: "PUT", path: "/api/note", body: { text } },
  });
}

export function addInsight(
  snippet: string,
  explanation: string,
  start: number,
  end: number,
): Promise<Insight> {
  return call<Insight>({
    command: "add_insight",
    args: { snippet, explanation, start, end },
    http: { method: "POST", path: "/api/note/insight", body: { snippet, explanation, start, end } },
  });
}

export function deleteInsight(id: number): Promise<void> {
  return call<void>({
    command: "delete_insight",
    args: { id },
    http: { method: "DELETE", path: `/api/note/insight/${id}` },
  });
}
