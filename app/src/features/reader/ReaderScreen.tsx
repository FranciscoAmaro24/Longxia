import { useEffect, useRef, useState } from "react";
import { Button, Panel, Tag } from "../../components";
import { cn } from "../../lib/cn";
import { toneOf } from "../../lib/pinyin";
import { lookup, segment, type DictEntry, type SegToken } from "../../lib/api";
import { PASSAGES } from "./passages";
import styles from "./ReaderScreen.module.css";

const HAN = /\p{Script=Han}/u;
const NBSP = " ";

interface Selection {
  key: string; // word + index, so repeats highlight independently
  word: string;
  left: number;
  top: number;
}

/**
 * Reader: the passage is segmented into words (multi-character words share one
 * pinyin, via `segment`), each shown with ambient pinyin and tappable for its
 * full dictionary entry. An optional tone-coloring mode colors each syllable by
 * its tone; it is off by default so the default reading surface stays neutral.
 */
export function ReaderScreen() {
  const [pi, setPi] = useState(0);
  const [tokens, setTokens] = useState<SegToken[] | null>(null);
  const [toneColor, setToneColor] = useState(false);
  const [sel, setSel] = useState<Selection | null>(null);
  const [entries, setEntries] = useState<DictEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);

  const passage = PASSAGES[pi];

  const close = () => {
    setSel(null);
    setEntries(null);
  };

  // Segment the passage whenever it changes.
  useEffect(() => {
    let alive = true;
    setTokens(null);
    close();
    segment(passage.text)
      .then((t) => alive && setTokens(t))
      .catch(() => alive && setTokens(null));
    return () => {
      alive = false;
    };
  }, [passage.text]);

  // Escape closes the popover.
  useEffect(() => {
    if (!sel) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [sel]);

  const onWordClick = (
    e: React.MouseEvent<HTMLButtonElement>,
    word: string,
    index: number,
  ) => {
    e.stopPropagation();
    const wrap = wrapRef.current;
    if (!wrap) return;
    const wr = wrap.getBoundingClientRect();
    const br = e.currentTarget.getBoundingClientRect();
    const left = Math.max(0, Math.min(br.left - wr.left, wr.width - 220));
    const top = br.bottom - wr.top + 6;

    setSel({ key: `${word}-${index}`, word, left, top });
    setEntries(null);
    setLoading(true);
    lookup(word)
      .then(setEntries)
      .catch(() => setEntries([]))
      .finally(() => setLoading(false));
  };

  // Render from segmented tokens, or fall back to raw characters (no pinyin).
  const render: SegToken[] =
    tokens ??
    Array.from(passage.text).map((text) => ({
      text,
      pinyin: null,
      word: HAN.test(text),
    }));

  return (
    <main className={styles.screen}>
      <header className={styles.header}>
        <span className={styles.zh} lang="zh">
          阅读
        </span>
        <span className={styles.en}>Reading</span>
      </header>

      <Panel
        label="Passage"
        actions={
          <div className={styles.tools}>
            <Button
              size="sm"
              variant={toneColor ? "secondary" : "ghost"}
              aria-pressed={toneColor}
              onClick={() => setToneColor((v) => !v)}
            >
              Tones
            </Button>
            <Tag variant="ink">Level {passage.level}</Tag>
          </div>
        }
      >
        <div className={styles.switcher}>
          {PASSAGES.map((p, i) => (
            <Button
              key={p.id}
              size="sm"
              variant={i === pi ? "secondary" : "ghost"}
              aria-pressed={i === pi}
              onClick={() => setPi(i)}
            >
              {Array.from(p.text).slice(0, 4).join("")}…
            </Button>
          ))}
        </div>

        <div className={styles.reader} ref={wrapRef} onClick={close}>
          <div className={styles.passage} lang="zh">
            {render.map((seg, si) => {
              if (!seg.word) {
                return (
                  <span key={si} className={cn(styles.cell, styles.punct)}>
                    <span className={styles.char}>{seg.text}</span>
                    <span className={styles.py}>{NBSP}</span>
                  </span>
                );
              }
              const chars = Array.from(seg.text);
              const sylls = seg.pinyin ? seg.pinyin.trim().split(/\s+/) : [];
              const aligned = sylls.length === chars.length;
              return (
                <button
                  key={si}
                  type="button"
                  className={cn(
                    styles.word,
                    sel?.key === `${seg.text}-${si}` && styles.wordActive,
                  )}
                  onClick={(e) => onWordClick(e, seg.text, si)}
                >
                  {chars.map((c, ci) => {
                    const syl = aligned ? sylls[ci] : ci === 0 ? seg.pinyin ?? "" : "";
                    const tone = syl ? toneOf(syl) : 0;
                    return (
                      <span key={ci} className={styles.cell}>
                        <span className={styles.char}>{c}</span>
                        <span
                          className={cn(styles.py, toneColor && styles[`tone${tone}`])}
                        >
                          {syl || NBSP}
                        </span>
                      </span>
                    );
                  })}
                </button>
              );
            })}
          </div>

          {sel && (
            <div
              className={styles.popover}
              style={{ left: sel.left, top: sel.top }}
              onClick={(e) => e.stopPropagation()}
              role="dialog"
              aria-label={`Lookup: ${sel.word}`}
            >
              <div className={styles.popHead}>
                <span className={styles.popHan} lang="zh">
                  {sel.word}
                </span>
                <Button size="sm" variant="quiet" onClick={close}>
                  Close
                </Button>
              </div>

              {loading ? (
                <p className={styles.popMuted}>Looking up…</p>
              ) : entries && entries.length > 0 ? (
                entries.map((en, i) => (
                  <div key={i} className={styles.entry}>
                    {en.pinyin && <div className={styles.entryPy}>{en.pinyin}</div>}
                    {en.gloss && (
                      <div className={styles.entryGloss}>{en.gloss}</div>
                    )}
                  </div>
                ))
              ) : (
                <p className={styles.popMuted}>No dictionary entry yet.</p>
              )}
            </div>
          )}
        </div>

        <p className={styles.translation}>{passage.translation}</p>
      </Panel>

      <p className={styles.hint}>
        Words are grouped and share one pinyin · tap a word for its full entry · Tones colors by
        tone.
      </p>
    </main>
  );
}
