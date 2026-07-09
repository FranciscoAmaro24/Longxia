/**
 * Pinyin helpers shared across features. `toneOf` reads the tone from a
 * syllable's diacritic; 0 means neutral (no mark).
 */
export type Tone = 0 | 1 | 2 | 3 | 4;

const TONE_MARKS: Record<string, 1 | 2 | 3 | 4> = {
  ā: 1, á: 2, ǎ: 3, à: 4,
  ē: 1, é: 2, ě: 3, è: 4,
  ī: 1, í: 2, ǐ: 3, ì: 4,
  ō: 1, ó: 2, ǒ: 3, ò: 4,
  ū: 1, ú: 2, ǔ: 3, ù: 4,
  ǖ: 1, ǘ: 2, ǚ: 3, ǜ: 4,
};

/** The tone of a single pinyin syllable (0 = neutral). */
export function toneOf(syllable: string): Tone {
  for (const ch of syllable) {
    const t = TONE_MARKS[ch];
    if (t) return t;
  }
  return 0;
}
