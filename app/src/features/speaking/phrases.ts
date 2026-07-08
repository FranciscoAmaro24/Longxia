/**
 * Speaking phrases, ordered by HSK band. `text` is the full string (used for
 * natural TTS prosody, punctuation included); `tokens` are the characters with
 * pinyin, used for per-syllable playback and tone contours.
 */
export interface Token {
  char: string;
  pinyin: string;
}

export interface Phrase {
  id: string;
  level: number;
  text: string;
  translation: string;
  tokens: Token[];
}

export const PHRASES: readonly Phrase[] = [
  {
    id: "greeting",
    level: 1,
    text: "你好，很高兴认识你。",
    translation: "Hello, nice to meet you.",
    tokens: [
      { char: "你", pinyin: "nǐ" },
      { char: "好", pinyin: "hǎo" },
      { char: "很", pinyin: "hěn" },
      { char: "高", pinyin: "gāo" },
      { char: "兴", pinyin: "xìng" },
      { char: "认", pinyin: "rèn" },
      { char: "识", pinyin: "shí" },
      { char: "你", pinyin: "nǐ" },
    ],
  },
  {
    id: "weather",
    level: 1,
    text: "今天天气很好。",
    translation: "The weather is nice today.",
    tokens: [
      { char: "今", pinyin: "jīn" },
      { char: "天", pinyin: "tiān" },
      { char: "天", pinyin: "tiān" },
      { char: "气", pinyin: "qì" },
      { char: "很", pinyin: "hěn" },
      { char: "好", pinyin: "hǎo" },
    ],
  },
  {
    id: "thanks",
    level: 1,
    text: "谢谢你，再见。",
    translation: "Thank you, goodbye.",
    tokens: [
      { char: "谢", pinyin: "xiè" },
      { char: "谢", pinyin: "xie" },
      { char: "你", pinyin: "nǐ" },
      { char: "再", pinyin: "zài" },
      { char: "见", pinyin: "jiàn" },
    ],
  },
  {
    id: "coffee",
    level: 2,
    text: "我想喝一杯咖啡。",
    translation: "I'd like to drink a cup of coffee.",
    tokens: [
      { char: "我", pinyin: "wǒ" },
      { char: "想", pinyin: "xiǎng" },
      { char: "喝", pinyin: "hē" },
      { char: "一", pinyin: "yì" },
      { char: "杯", pinyin: "bēi" },
      { char: "咖", pinyin: "kā" },
      { char: "啡", pinyin: "fēi" },
    ],
  },
  {
    id: "restroom",
    level: 2,
    text: "请问，洗手间在哪里？",
    translation: "Excuse me, where is the restroom?",
    tokens: [
      { char: "请", pinyin: "qǐng" },
      { char: "问", pinyin: "wèn" },
      { char: "洗", pinyin: "xǐ" },
      { char: "手", pinyin: "shǒu" },
      { char: "间", pinyin: "jiān" },
      { char: "在", pinyin: "zài" },
      { char: "哪", pinyin: "nǎ" },
      { char: "里", pinyin: "lǐ" },
    ],
  },
  {
    id: "library",
    level: 3,
    text: "我周末去图书馆看书。",
    translation: "On the weekend I go to the library to read.",
    tokens: [
      { char: "我", pinyin: "wǒ" },
      { char: "周", pinyin: "zhōu" },
      { char: "末", pinyin: "mò" },
      { char: "去", pinyin: "qù" },
      { char: "图", pinyin: "tú" },
      { char: "书", pinyin: "shū" },
      { char: "馆", pinyin: "guǎn" },
      { char: "看", pinyin: "kàn" },
      { char: "书", pinyin: "shū" },
    ],
  },
  {
    id: "favor",
    level: 3,
    text: "你能帮我一个忙吗？",
    translation: "Can you do me a favor?",
    tokens: [
      { char: "你", pinyin: "nǐ" },
      { char: "能", pinyin: "néng" },
      { char: "帮", pinyin: "bāng" },
      { char: "我", pinyin: "wǒ" },
      { char: "一", pinyin: "yí" },
      { char: "个", pinyin: "gè" },
      { char: "忙", pinyin: "máng" },
      { char: "吗", pinyin: "ma" },
    ],
  },
] as const;
