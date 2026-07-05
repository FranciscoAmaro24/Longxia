/**
 * Practice characters for the Writing module, with their stroke data bundled
 * locally (no CDN, works offline). To add a character: import its JSON from
 * hanzi-writer-data and add an entry to PRACTICE + CHAR_DATA.
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

export interface PracticeChar {
  char: string;
  pinyin: string;
  gloss: string;
}

export const PRACTICE: readonly PracticeChar[] = [
  { char: "你", pinyin: "nǐ", gloss: "you" },
  { char: "好", pinyin: "hǎo", gloss: "good; well" },
  { char: "我", pinyin: "wǒ", gloss: "I; me" },
  { char: "学", pinyin: "xué", gloss: "to study" },
  { char: "习", pinyin: "xí", gloss: "to practice" },
  { char: "写", pinyin: "xiě", gloss: "to write" },
  { char: "字", pinyin: "zì", gloss: "character; word" },
  { char: "人", pinyin: "rén", gloss: "person" },
  { char: "中", pinyin: "zhōng", gloss: "middle; China" },
  { char: "文", pinyin: "wén", gloss: "writing; language" },
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
};
