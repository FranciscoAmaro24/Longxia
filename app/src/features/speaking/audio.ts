/**
 * Speech helpers: text-to-speech via the system Chinese voice, and tone
 * detection + contour shapes derived from tone-marked pinyin.
 */

export type Tone = 0 | 1 | 2 | 3 | 4;

// Combining tone marks (NFD-decomposed).
const MACRON = 0x0304; // ā  -> tone 1
const ACUTE = 0x0301; //  á  -> tone 2
const CARON = 0x030c; //  ǎ  -> tone 3
const GRAVE = 0x0300; //  à  -> tone 4

/** Detect the tone (1-4, 0 = neutral) from a tone-marked pinyin syllable. */
export function toneOf(pinyin: string): Tone {
  for (const ch of pinyin.normalize("NFD")) {
    switch (ch.codePointAt(0)) {
      case MACRON:
        return 1;
      case ACUTE:
        return 2;
      case CARON:
        return 3;
      case GRAVE:
        return 4;
    }
  }
  return 0;
}

/** SVG path (in a 24x16 box) drawing the tone's pitch contour. */
export function tonePath(tone: Tone): string {
  switch (tone) {
    case 1:
      return "M2,4 L22,4"; // high level
    case 2:
      return "M2,13 L22,4"; // rising
    case 3:
      return "M2,6 L11,14 L22,5"; // dip
    case 4:
      return "M2,4 L22,14"; // falling
    default:
      return "M9,9 L15,9"; // neutral
  }
}

export function ttsAvailable(): boolean {
  return typeof window !== "undefined" && "speechSynthesis" in window;
}

function chineseVoice(): SpeechSynthesisVoice | null {
  const voices = window.speechSynthesis.getVoices();
  return voices.find((v) => v.lang.toLowerCase().startsWith("zh")) ?? null;
}

/** Speak Chinese text. `rate` < 1 is slower (useful for learners). */
export function speak(text: string, rate = 1): void {
  if (!ttsAvailable()) return;
  const synth = window.speechSynthesis;
  synth.cancel();
  const utter = new SpeechSynthesisUtterance(text);
  utter.lang = "zh-CN";
  const voice = chineseVoice();
  if (voice) utter.voice = voice;
  utter.rate = rate;
  synth.speak(utter);
}
