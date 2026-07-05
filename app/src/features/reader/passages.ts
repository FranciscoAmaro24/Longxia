/**
 * Sample graded passages for the reader. Static for now; later these come from
 * a content table (and user-imported text). Keep `text` in simplified
 * characters to match the app's scope.
 */
export interface Passage {
  id: string;
  level: number;
  text: string;
  translation: string;
}

export const PASSAGES: readonly Passage[] = [
  {
    id: "library",
    level: 3,
    text: "我们周末一起去图书馆看书。",
    translation: "On the weekend we go to the library together to read.",
  },
  {
    id: "greeting",
    level: 1,
    text: "你好，很高兴认识你。",
    translation: "Hello, nice to meet you.",
  },
] as const;
