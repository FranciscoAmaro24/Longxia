/**
 * Sample speaking phrases. `text` is the full string (used for natural TTS
 * prosody, punctuation included); `tokens` are the characters with pinyin,
 * used for per-syllable playback and tone contours.
 */
export interface Token {
  char: string;
  pinyin: string;
}

export interface Phrase {
  id: string;
  text: string;
  translation: string;
  tokens: Token[];
}

export const PHRASES: readonly Phrase[] = [
  {
    id: "greeting",
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
    id: "library",
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
    id: "weather",
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
] as const;
