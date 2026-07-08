/**
 * Graded passages for the reader, ordered by HSK band (easiest first). Static
 * for now; later these come from a content table (and user-imported text). Keep
 * `text` in simplified characters to match the app's scope.
 */
export interface Passage {
  id: string;
  level: number;
  text: string;
  translation: string;
}

export const PASSAGES: readonly Passage[] = [
  {
    id: "greeting",
    level: 1,
    text: "你好，很高兴认识你。",
    translation: "Hello, nice to meet you.",
  },
  {
    id: "student",
    level: 1,
    text: "我是学生，我学习中文。",
    translation: "I am a student; I study Chinese.",
  },
  {
    id: "park",
    level: 2,
    text: "今天天气很好，我们一起去公园吧。",
    translation: "The weather is nice today; let's go to the park together.",
  },
  {
    id: "family",
    level: 2,
    text: "我家有四口人：爸爸、妈妈、姐姐和我。",
    translation: "My family has four people: dad, mom, older sister, and me.",
  },
  {
    id: "library",
    level: 3,
    text: "我们周末一起去图书馆看书。",
    translation: "On the weekend we go to the library together to read.",
  },
  {
    id: "morning",
    level: 3,
    text: "他每天早上七点起床，然后去上班。",
    translation: "He gets up at seven every morning, then goes to work.",
  },
  {
    id: "work",
    level: 4,
    text: "虽然工作很忙，但是他觉得很快乐。",
    translation: "Although work is busy, he feels very happy.",
  },
] as const;
