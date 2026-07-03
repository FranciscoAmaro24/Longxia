# 龙虾 Lóngxiā - a Chinese study app

A desktop + mobile app for learning Chinese, organized around the **HSK 3.0**
(three-level / nine-band) exam. It combines a real **notebook**, **interactive 田字格
character drawing**, and an **AI that annotates specific parts** of the language and your
writing - like a teacher's red pen.

Three learning sections: **Reading**, **Writing**, and **Speaking**.

## Status

**Planning / pre-scaffold.** Design direction and wireframes done; app not yet scaffolded.

## Key decisions so far

| Area | Decision |
|---|---|
| Stack | Tauri 2.0 + React + TypeScript (UI) + Rust (core) |
| Targets | macOS/Windows/Linux + iOS/Android, offline-first |
| Characters | **Simplified only** for now (traditional later) |
| Pronunciation | **Pinyin** shown throughout |
| Curriculum | HSK 3.0 exam syllabus (CLEC/CTI 2025); see `PLAN.md` |
| AI model | Default **`claude-haiku-4-5`** (cheapest); opt-in bump for writing critique |
| Spaced repetition | FSRS (Rust core) |

## Design language

Paper + ink + a single red "correction" pen (AI/annotations) and jade (progress).
Chinese typography (Songti SC / PingFang SC), with the **田字格 practice grid** as the
structural motif. Deliberately *not* the generic gradient-and-cards look.

## Repository layout

```text
chinese-learning-app/
├── README.md          ← you are here
├── CHANGELOG.md       ← running log of progress (manual + automated notes)
├── PLAN.md            ← full technical + curriculum plan
├── design/
│   └── wireframes.html  ← design direction + six core screens (open in a browser)
└── (app scaffold - added in Phase 1)
```

## Prerequisites (already installed on this machine)

- Rust + cargo (`rustc --version`)
- Node + npm
- Xcode Command Line Tools (macOS)

## Roadmap

See `PLAN.md` §7. Currently entering **Phase 1 - Foundation** (scaffold, SQLite schema,
dictionary + stroke-data import, HSK level structure).

## Notes for the maintainer

This README and `CHANGELOG.md` are kept up to date as the project progresses - edit them
freely to add your own notes.
