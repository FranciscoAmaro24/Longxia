import { useEffect, useRef, useState } from "react";
import HanziWriter from "hanzi-writer";
import { Button, Panel, TianGrid } from "../../components";
import { cn } from "../../lib/cn";
import { CHAR_DATA, PRACTICE, PRACTICE_BANDS } from "./characters";
import styles from "./WritingScreen.module.css";

const SIZE = 240;

interface Feedback {
  text: string;
  tone?: "correction" | "jade";
}

/** Read theme colors from CSS tokens so the strokes match light/dark. */
function readColors() {
  const s = getComputedStyle(document.documentElement);
  const v = (name: string, fallback: string) =>
    s.getPropertyValue(name).trim() || fallback;
  return {
    strokeColor: v("--ink", "#1b1e24"),
    outlineColor: v("--ink-faint", "#8a8d93"),
    drawingColor: v("--correction", "#bb3b2e"),
    highlightColor: v("--jade", "#4e7c6b"),
  };
}

/**
 * Writing: stroke-order animation and guided tracing (quiz) inside the 田字格
 * cell, powered by Hanzi Writer with locally bundled stroke data. The user's
 * strokes draw in the correction (red-pen) color; completion feedback is jade.
 */
export function WritingScreen() {
  const [band, setBand] = useState<number>(PRACTICE_BANDS[0]);
  const [char, setChar] = useState(PRACTICE[0].char);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const targetRef = useRef<HTMLDivElement>(null);
  const writerRef = useRef<HanziWriter | null>(null);

  // (Re)create the writer whenever the character changes.
  useEffect(() => {
    const target = targetRef.current;
    if (!target) return;
    target.innerHTML = "";
    setFeedback(null);

    writerRef.current = HanziWriter.create(target, char, {
      width: SIZE,
      height: SIZE,
      padding: 10,
      showCharacter: false,
      showOutline: true,
      ...readColors(),
      charDataLoader: (c) => CHAR_DATA[c],
    });

    return () => {
      writerRef.current = null;
      target.innerHTML = "";
    };
  }, [char]);

  const animate = () => {
    setFeedback({ text: "Watch the stroke order" });
    writerRef.current?.animateCharacter();
  };

  const trace = () => {
    const writer = writerRef.current;
    if (!writer) return;
    setFeedback({ text: "Trace each stroke in order" });
    writer.quiz({
      onMistake: (stroke) =>
        setFeedback({
          text: `Stroke ${stroke.strokeNum + 1}: follow the highlight`,
          tone: "correction",
        }),
      onComplete: (summary) =>
        setFeedback({
          text: `Complete · ${summary.totalMistakes} mistake${
            summary.totalMistakes === 1 ? "" : "s"
          }`,
          tone: "jade",
        }),
    });
  };

  const reveal = () => {
    setFeedback(null);
    writerRef.current?.showCharacter();
  };

  const reset = () => {
    setFeedback(null);
    writerRef.current?.hideCharacter();
  };

  const current = PRACTICE.find((p) => p.char === char);
  const chars = PRACTICE.filter((p) => p.level === band);

  // Switch bands and jump to that band's first character.
  const selectBand = (b: number) => {
    setBand(b);
    const first = PRACTICE.find((p) => p.level === b);
    if (first) setChar(first.char);
  };

  return (
    <main className={styles.screen}>
      <header className={styles.header}>
        <span className={styles.zh} lang="zh">
          书写
        </span>
        <span className={styles.en}>Writing</span>
      </header>

      <Panel
        label="Character"
        actions={
          <div className={styles.bands}>
            {PRACTICE_BANDS.map((b) => (
              <Button
                key={b}
                size="sm"
                variant={b === band ? "secondary" : "ghost"}
                aria-pressed={b === band}
                onClick={() => selectBand(b)}
              >
                HSK {b}
              </Button>
            ))}
          </div>
        }
      >
        <div className={styles.selector}>
          {chars.map((p) => (
            <Button
              key={p.char}
              size="sm"
              variant={p.char === char ? "secondary" : "ghost"}
              aria-pressed={p.char === char}
              onClick={() => setChar(p.char)}
            >
              <span className={styles.selChar} lang="zh">
                {p.char}
              </span>
            </Button>
          ))}
        </div>

        <div className={styles.stage}>
          <TianGrid size={SIZE + 8}>
            <div ref={targetRef} className={styles.target} />
          </TianGrid>

          <div className={styles.controls}>
            <Button variant="primary" onClick={animate}>
              Animate
            </Button>
            <Button variant="accent" onClick={trace}>
              Trace
            </Button>
            <Button variant="ghost" onClick={reveal}>
              Show
            </Button>
            <Button variant="quiet" onClick={reset}>
              Reset
            </Button>
          </div>

          <p
            className={cn(
              styles.feedback,
              feedback?.tone === "correction" && styles.feedbackCorrection,
              feedback?.tone === "jade" && styles.feedbackJade,
            )}
          >
            {feedback?.text ?? ""}
          </p>

          {current && (
            <div className={styles.meta}>
              <span className={styles.metaChar} lang="zh">
                {current.char}
              </span>
              <span className={styles.metaPy}>{current.pinyin}</span>
              <span className={styles.metaGloss}>{current.gloss}</span>
            </div>
          )}
        </div>
      </Panel>

      <p className={styles.hint}>
        Animate to watch the strokes, then Trace to write it yourself.
      </p>
    </main>
  );
}
