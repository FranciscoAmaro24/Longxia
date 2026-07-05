import { useEffect, useRef, useState } from "react";
import { Button, Panel, Tag } from "../../components";
import { cn } from "../../lib/cn";
import { annotate, lookup, type Annotated, type DictEntry } from "../../lib/api";
import { PASSAGES } from "./passages";
import styles from "./ReaderScreen.module.css";

const HAN = /\p{Script=Han}/u;
const NBSP = " ";

interface Selection {
  key: string; // char + index, so repeats highlight independently
  char: string;
  left: number;
  top: number;
}

/**
 * Reader: every character shows its pinyin underneath (ambient, no click), and
 * tapping a character opens a popover with its full dictionary senses. Ambient
 * pinyin comes from `annotate`; the popover from `lookup`. Word segmentation
 * and tone coloring are later work.
 */
export function ReaderScreen() {
  const [pi, setPi] = useState(0);
  const [tokens, setTokens] = useState<Annotated[] | null>(null);
  const [sel, setSel] = useState<Selection | null>(null);
  const [entries, setEntries] = useState<DictEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);

  const passage = PASSAGES[pi];

  const close = () => {
    setSel(null);
    setEntries(null);
  };

  // Load ambient pinyin whenever the passage changes.
  useEffect(() => {
    let alive = true;
    setTokens(null);
    close();
    annotate(passage.text)
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

  const onCharClick = (
    e: React.MouseEvent<HTMLButtonElement>,
    char: string,
    index: number,
  ) => {
    e.stopPropagation();
    const wrap = wrapRef.current;
    if (!wrap) return;
    const wr = wrap.getBoundingClientRect();
    const br = e.currentTarget.getBoundingClientRect();
    const left = Math.max(0, Math.min(br.left - wr.left, wr.width - 220));
    const top = br.bottom - wr.top + 6;

    setSel({ key: `${char}-${index}`, char, left, top });
    setEntries(null);
    setLoading(true);
    lookup(char)
      .then(setEntries)
      .catch(() => setEntries([]))
      .finally(() => setLoading(false));
  };

  // Render from annotated tokens, or fall back to raw characters (no pinyin).
  const render: Annotated[] =
    tokens ?? Array.from(passage.text).map((text) => ({ text, pinyin: null }));

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
        actions={<Tag variant="ink">Level {passage.level}</Tag>}
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
            {render.map((tok, i) =>
              HAN.test(tok.text) ? (
                <button
                  key={i}
                  type="button"
                  className={cn(
                    styles.token,
                    styles.tokenBtn,
                    sel?.key === `${tok.text}-${i}` && styles.tokenActive,
                  )}
                  onClick={(e) => onCharClick(e, tok.text, i)}
                >
                  <span className={styles.char}>{tok.text}</span>
                  <span className={styles.py}>{tok.pinyin ?? NBSP}</span>
                </button>
              ) : (
                <span key={i} className={cn(styles.token, styles.tokenPunct)}>
                  <span className={styles.char}>{tok.text}</span>
                  <span className={styles.py}>{NBSP}</span>
                </span>
              ),
            )}
          </div>

          {sel && (
            <div
              className={styles.popover}
              style={{ left: sel.left, top: sel.top }}
              onClick={(e) => e.stopPropagation()}
              role="dialog"
              aria-label={`Lookup: ${sel.char}`}
            >
              <div className={styles.popHead}>
                <span className={styles.popHan} lang="zh">
                  {sel.char}
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

      <p className={styles.hint}>Pinyin shows under each character · tap for the full entry.</p>
    </main>
  );
}
