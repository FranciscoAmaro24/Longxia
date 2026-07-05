# Chinese Learning App - Plan

_Last updated: 2026-07-03_

A Chinese-learning app with **notebook features, interactive character drawing, and AI
insights** on specific parts of the language/writing. Organized around the HSK 3.0
(three-level / nine-band) exam, with dedicated **Reading**, **Writing**, and **Speaking**
learning sections.

---

## 1. Tech stack (decided)

**Tauri 2.0 + React + TypeScript (UI) + Rust (core).** One codebase → desktop
(macOS/Windows/Linux) **and** mobile (iOS/Android). Offline-first.

Rule of thumb: **data, scheduling, and matching logic in Rust; everything visual/interactive
in React.**

```
┌─────────────────────── React (TypeScript) UI ───────────────────────┐
│  Reader · Writing canvas (Hanzi Writer) · Notebook · Speaking · SRS  │
│         review UI · Dashboards · Settings                            │
└───────────────▲──────────────────────────────────┬──────────────────┘
                │  Tauri commands (invoke)          │  events
┌───────────────┴──────────────────────────────────▼──────────────────┐
│                        Rust core (tauri::command)                    │
│  • SQLite access (sqlx/rusqlite)      • FSRS scheduler               │
│  • Dictionary + stroke-data queries   • Handwriting stroke matching  │
│  • Claude API client (keeps key off the UI)   • Import/sync jobs     │
└──────────────────────────────────────────────────────────────────────┘
                                   │
                      SQLite (local, offline-first)
```

Why: gives us Rust for perf-sensitive parts (SRS, stroke matching, local dictionary),
keeps the **Claude API key in the Rust layer** (out of the JS bundle), and works fully
offline - essential for a study app. HTML `<canvas>` handles interactive drawing.

### Key libraries
- **Hanzi Writer** - stroke-order animation + tracing/quiz (JS, drop-in)
- **CC-CEDICT** - free open Chinese–English dictionary
- **Make Me a Hanzi** - free stroke-order + character decomposition data
- **FSRS** - spaced-repetition algorithm (maintained Rust crate)
- **SQLite** - local store for cards, notes, progress
- **Claude API** - AI insights (grammar explanation, writing/pronunciation feedback,
  level-appropriate conversation); multimodal input for drawn-character feedback
- TTS/STT - platform speech APIs, or a cloud provider for tone scoring

---

## 2. Data model (SQLite)

The app is fundamentally "cards + notes + a schedule."

```
characters   (id, hanzi, pinyin, stroke_count, radicals,
              decomposition, stroke_data_json, freq_rank, hsk_level,
              must_handwrite BOOL)   -- recognize vs. hand-write subset
words        (id, simplified, traditional, pinyin, definitions_json,
              hsk_level, measure_words)
grammar      (id, title, pattern, explanation, examples_json, hsk_level)

cards        (id, kind[char|word|grammar|note], ref_id,
              -- FSRS state ↓
              stability, difficulty, due, last_review, reps, lapses, state)
reviews      (id, card_id, rating[1-4], reviewed_at, elapsed_ms)

notes        (id, title, body_json, created, updated)     -- rich content blocks
note_spans   (id, note_id, start, end, ai_insight_json)   -- AI bound to a text range
drawings     (id, note_id, strokes_json, target_char, score)

decks        (id, name, hsk_level)   deck_cards (deck_id, card_id)
progress     (hsk_level, chars_learned, words_learned, grammar_learned, …)

import_versions (id, source, standard, version, imported_at)  -- versioned syllabus data

dictionary   (id, simplified, traditional, pinyin, gloss)  -- CC-CEDICT lookup source,
                                                           -- distinct from curated `words`
```

Design decisions:
- **One `cards` table drives SRS**, pointing at char/word/grammar/note via `kind` + `ref_id`.
  Anything - including a notebook snippet - can become a review card.
- **`body_json`** = structured note document (text blocks + embedded drawings + embedded
  cards), so the notebook mixes typed text, drawn characters, and saved dictionary entries.
- **`note_spans`** implements "AI insight on a specific part": highlight a range → store the
  AI response bound to that character offset.
- **`must_handwrite`** flag: the exam distinguishes characters you must *recognize* from the
  smaller set you must *hand-write*. The Writing module reviews that subset.
- **`import_versions`**: keep syllabus data versioned (the standard just went live and may
  get errata) rather than baked into code.

---

## 3. HSK 3.0 content spine - VERIFY AGAINST OFFICIAL FILES BEFORE BUILDING

**There are two official documents - do not confuse them:**

1. **GF0025-2021** - Ministry of Education *curriculum standard*
   (《国际中文教育中文水平等级标准》), March 2021.
   Totals: **1,110 syllables · 3,000 characters · 11,092 words · 572 grammar points.**
2. **HSK 3.0 exam syllabus** - CLEC/CTI, published **Nov 2025**, effective **July 1, 2026**
   (406 pages). Revised vocabulary → **10,896 words**. This is what the exam tests.

→ Track the app's curriculum against **#2 (exam syllabus)**; use #1 as an advanced superset.

**Corrected vocabulary progression (2025 exam syllabus, cumulative):**

| Level | Words added | Words (cumulative) |
|------:|------------:|-------------------:|
| 1     | 300         | 300     |
| 2     | 197         | 496     |
| 3     | 493         | 988     |
| 4     | 990         | 1,978   |
| 5     | 1,579       | 3,557   |
| 6     | 1,777       | 5,334   |
| 7–9   | 5,562       | 10,896  |

**Character and grammar per-level counts: secondary sources CONFLICT.** Do NOT hardcode
these from any blog - import the official CTI lists and derive counts from the data.

**Structural facts:**
- **Levels 7–9 are one combined exam** - a single score places you at 7, 8, or 9. Model as
  one deck pool with depth tiers, not three separate syllabi.
- **Skills tested vary by band**: listening + reading at the bottom, writing added mid-bands.
  The **7–9 speaking format was not officially released as of April 2026** → keep advanced
  Speaking flexible/stubbed.
- Separate **"writing characters"** requirement (~101 → ~1,208 across bands) → drives the
  `must_handwrite` subset and validates the interactive-drawing feature.

**Where the authoritative data lives:**
- `chinesetest.cn` (CTI) - official query system for syllables/characters/vocab/grammar +
  exam registration portal.
- MoE "应用解读本" (Application Interpretation Books) - per-level tables.
- GF0025-2021 PDF - via moe.gov.cn.
- No clean one-click XLSX; import via the CTI query system + community machine-readable
  mirrors, then **validate counts against the official totals above.**

---

## 4. SRS engine (FSRS)
- Use **FSRS** (better than legacy SM-2). Each card stores `stability`, `difficulty`, `due`,
  `state`. Reviews rated **Again/Hard/Good/Easy (1–4)**; Rust recomputes next due date.
- **New-card introduction gated per HSK level** so learners aren't flooded.
- **Unified scheduler**: a word can be tested as reading (recognize) AND writing (produce)
  via two card templates over the same `ref_id` - cleaner than parallel schedulers.

---

## 5. AI insights (Claude API)
All calls go through the **Rust core** (key stays out of the JS bundle). Use the latest
Claude model; multimodal input enables drawn-character feedback. Pull exact model IDs +
pricing from the current API reference at build time.

| Feature | Input → Output | Fires from |
|---|---|---|
| Sentence/grammar explain | selected text → per-word gloss + grammar pattern + measure-word rationale | Reader tap, Notebook highlight |
| Writing feedback | image of drawn character + target → stroke-order/structure/balance notes | Writing canvas |
| Pronunciation coach | transcript + target → tone/initial/final error notes | Speaking (post-STT) |
| Level-aware chat | conversation capped to user's HSK vocabulary | Speaking practice |
| Text difficulty grading | passage → estimated HSK level + hard-word list | Reader "import text" |

- **Cache** AI explanations keyed by `(text, feature)` in SQLite → free + offline on re-tap.
- Use **structured/tool-formatted output** so responses render as UI components (word chips,
  pattern cards), not walls of text.
- One-tap "turn this explanation into an SRS card."

---

## 6. Feature modules (React)
- **Reader** - tokenized text, tap-for-dictionary, tone coloring, "explain", import-your-own-text.
- **Writing** - Hanzi Writer animation + guided tracing + free-draw quiz; 田字格 grid paper;
  stroke matching in Rust; AI structure feedback. Reviews the `must_handwrite` subset.
- **Notebook** - block editor mixing text/drawings/embedded cards; highlight→AI; send-to-SRS.
- **Speaking** - TTS on any token, record + STT, tone scoring, level-capped roleplay chat.
- **Review** - FSRS queue with per-skill card templates.
- **Dashboard** - HSK 3.0 progress rings, streaks, review heatmap.

---

## 7. Build phases
1. **Foundation** - Tauri+React scaffold, SQLite schema, import CC-CEDICT + stroke data,
   HSK 3.0 level structure.
2. **Data import** _(real milestone)_ - pull official CTI syllabus lists, validate counts,
   load into versioned import tables.
3. **SRS + Reading** - FSRS engine, tap-to-lookup reader, HSK vocab decks. First usable build.
4. **Writing** - Hanzi Writer canvas, tracing, stroke-order practice, 田字格 notebook paper.
5. **Notebook + AI** - freeform notes, highlight→AI-insight, send-to-SRS, grammar explanations.
6. **Speaking** - TTS everywhere, then STT + tone/pronunciation scoring (advanced levels flexible).
7. **Polish** - progress dashboards, streaks, per-level completion vs HSK 3.0 targets.

---

## 8. Open questions
1. **Simplified only, or simplified + traditional?** (Affects dictionary + card schema.)
2. **Speech scoring**: on-device (limited tone accuracy) vs cloud speech API (better tones,
   needs network + cost).
3. **Handwriting**: full recognition (recognize any written character) vs stroke-match against
   a known target. Latter is far easier and covers ~90% of study value - recommended to start.

---

## Sources
- HSKStory - Complete HSK 3.0 Vocabulary List (2025 final): https://hskstory.com/guides/hsk-30-vocabulary-complete
- MandarinZone - New HSK 3.0 Changes Explained (2026): https://www.mandarinzone.com/new-hsk-test/
- Skritter Blog - The new HSK 3.0: https://blog.skritter.com/2021/06/the-new-hsk-3-0-what-you-need-to-know/
- GF0025-2021 (MoE): http://www.moe.gov.cn/jyb_xwfb/gzdt_gzdt/s5987/202103/t20210329_523304.html
- CTI official query system / registration: http://www.chinesetest.cn
