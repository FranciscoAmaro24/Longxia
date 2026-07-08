/**
 * Practice characters for the Writing module, grouped by HSK band, with their
 * stroke data bundled locally (no CDN, works offline). To add a character:
 * import its JSON from hanzi-writer-data and add an entry to PRACTICE + CHAR_DATA.
 */
import type { CharacterJson } from "hanzi-writer";

import ni from "hanzi-writer-data/你.json";
import hao from "hanzi-writer-data/好.json";
import wo from "hanzi-writer-data/我.json";
import xue from "hanzi-writer-data/学.json";
import xi from "hanzi-writer-data/习.json";
import xie from "hanzi-writer-data/写.json";
import zi from "hanzi-writer-data/字.json";
import ren from "hanzi-writer-data/人.json";
import zhong from "hanzi-writer-data/中.json";
import wen from "hanzi-writer-data/文.json";
import da from "hanzi-writer-data/大.json";
import xiao from "hanzi-writer-data/小.json";
import kou from "hanzi-writer-data/口.json";
import jia from "hanzi-writer-data/家.json";
import peng from "hanzi-writer-data/朋.json";
import you from "hanzi-writer-data/友.json";
import tu from "hanzi-writer-data/图.json";
import shu from "hanzi-writer-data/书.json";
import guan from "hanzi-writer-data/馆.json";
import xie2 from "hanzi-writer-data/谢.json";
import qing from "hanzi-writer-data/请.json";
import wenq from "hanzi-writer-data/问.json";
import jian from "hanzi-writer-data/间.json";

export interface PracticeChar {
  char: string;
  pinyin: string;
  gloss: string;
  /** HSK band this character is practised at. */
  level: number;
}

export const PRACTICE: readonly PracticeChar[] = [
  // Band 1: simplest, highest-frequency forms.
  { char: "人", pinyin: "rén", gloss: "person", level: 1 },
  { char: "大", pinyin: "dà", gloss: "big", level: 1 },
  { char: "小", pinyin: "xiǎo", gloss: "small", level: 1 },
  { char: "口", pinyin: "kǒu", gloss: "mouth", level: 1 },
  { char: "中", pinyin: "zhōng", gloss: "middle; China", level: 1 },
  { char: "我", pinyin: "wǒ", gloss: "I; me", level: 1 },
  { char: "你", pinyin: "nǐ", gloss: "you", level: 1 },
  { char: "好", pinyin: "hǎo", gloss: "good; well", level: 1 },
  // Band 2: study and everyday life.
  { char: "学", pinyin: "xué", gloss: "to study", level: 2 },
  { char: "习", pinyin: "xí", gloss: "to practice", level: 2 },
  { char: "写", pinyin: "xiě", gloss: "to write", level: 2 },
  { char: "字", pinyin: "zì", gloss: "character; word", level: 2 },
  { char: "文", pinyin: "wén", gloss: "writing; language", level: 2 },
  { char: "家", pinyin: "jiā", gloss: "home; family", level: 2 },
  { char: "朋", pinyin: "péng", gloss: "friend (朋友)", level: 2 },
  { char: "友", pinyin: "yǒu", gloss: "friend", level: 2 },
  // Band 3: more strokes, common compounds.
  { char: "图", pinyin: "tú", gloss: "picture; diagram", level: 3 },
  { char: "书", pinyin: "shū", gloss: "book", level: 3 },
  { char: "馆", pinyin: "guǎn", gloss: "building; venue", level: 3 },
  { char: "谢", pinyin: "xiè", gloss: "to thank", level: 3 },
  { char: "请", pinyin: "qǐng", gloss: "please; to invite", level: 3 },
  { char: "问", pinyin: "wèn", gloss: "to ask", level: 3 },
  { char: "间", pinyin: "jiān", gloss: "between; room", level: 3 },
] as const;

export const CHAR_DATA: Record<string, CharacterJson> = {
  你: ni as CharacterJson,
  好: hao as CharacterJson,
  我: wo as CharacterJson,
  学: xue as CharacterJson,
  习: xi as CharacterJson,
  写: xie as CharacterJson,
  字: zi as CharacterJson,
  人: ren as CharacterJson,
  中: zhong as CharacterJson,
  文: wen as CharacterJson,
  大: da as CharacterJson,
  小: xiao as CharacterJson,
  口: kou as CharacterJson,
  家: jia as CharacterJson,
  朋: peng as CharacterJson,
  友: you as CharacterJson,
  图: tu as CharacterJson,
  书: shu as CharacterJson,
  馆: guan as CharacterJson,
  谢: xie2 as CharacterJson,
  请: qing as CharacterJson,
  问: wenq as CharacterJson,
  间: jian as CharacterJson,
};

/** Distinct HSK bands present in the practice set, ascending. */
export const PRACTICE_BANDS: readonly number[] = [
  ...new Set(PRACTICE.map((p) => p.level)),
].sort((a, b) => a - b);
