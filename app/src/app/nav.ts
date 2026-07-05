/**
 * Navigation model. One source of truth for the sections so the shell, the
 * router (added later), and any deep links stay in sync. `id` is stable and
 * used as a key; labels are display-only.
 */
export interface NavItem {
  id: SectionId;
  zh: string;
  en: string;
}

export type SectionId =
  | "today"
  | "read"
  | "write"
  | "notebook"
  | "speak"
  | "review";

export const NAV_ITEMS: readonly NavItem[] = [
  { id: "today", zh: "今天", en: "Today" },
  { id: "read", zh: "阅读", en: "Read" },
  { id: "write", zh: "书写", en: "Write" },
  { id: "notebook", zh: "笔记", en: "Notebook" },
  { id: "speak", zh: "口语", en: "Speak" },
  { id: "review", zh: "复习", en: "Review" },
] as const;
