import { useCallback, useEffect, useRef, useState } from "react";
import { Button, Panel, Tag } from "../../components";
import { cn } from "../../lib/cn";
import {
  getReviewQueue,
  reviewCard,
  type Rating,
  type ReviewCard,
} from "../../lib/api";
import styles from "./ReviewScreen.module.css";

type Mode = "pinyin" | "chars";

/** Seconds until due -> a short human label for the grade buttons. */
function fmtInterval(secs: number): string {
  if (secs < 60) return "<1m";
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  if (secs < 86400) return `${Math.round(secs / 3600)}h`;
  return `${Math.round(secs / 86400)}d`;
}

/** Normalize pinyin for comparison: drop tones, spacing, and u/ü/v differences. */
function normPinyin(s: string): string {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "") // strip combining tone marks
    .replace(/v/g, "u")
    .replace(/[^a-z]/g, "");
}

const GRADES: {
  rating: Rating;
  label: string;
  variant: "accent" | "secondary" | "primary" | "ghost";
  key: keyof Pick<ReviewCard, "again" | "hard" | "good" | "easy">;
}[] = [
  { rating: 1, label: "Again", variant: "accent", key: "again" },
  { rating: 2, label: "Hard", variant: "secondary", key: "hard" },
  { rating: 3, label: "Good", variant: "primary", key: "good" },
  { rating: 4, label: "Easy", variant: "ghost", key: "easy" },
];

/**
 * Review: an FSRS study loop with typed recall. Toggle between typing the
 * pinyin (card shows the characters) or typing the characters via IME (card
 * shows pinyin + meaning). Type, check, then rate. Reschedules via the Rust
 * core. Keyboard: Enter checks, 1-4 rate.
 */
export function ReviewScreen() {
  const [queue, setQueue] = useState<ReviewCard[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<Mode>("pinyin");
  const [idx, setIdx] = useState(0);
  const [input, setInput] = useState("");
  const [checked, setChecked] = useState(false);
  const [correct, setCorrect] = useState<boolean | null>(null);
  const [reviewed, setReviewed] = useState(0);
  const [busy, setBusy] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    getReviewQueue()
      .then(setQueue)
      .catch((e) => setError(String(e)));
  }, []);

  const current = queue && idx < queue.length ? queue[idx] : null;
  const done = queue != null && idx >= queue.length && queue.length > 0;
  const remaining = queue ? queue.length - idx : 0;

  // Reset the answer state on card change or mode switch, and focus the input.
  useEffect(() => {
    setInput("");
    setChecked(false);
    setCorrect(null);
  }, [idx, mode]);

  useEffect(() => {
    if (current && !checked) inputRef.current?.focus();
  }, [current, checked, mode]);

  const check = useCallback(
    (skip = false) => {
      if (!current || checked) return;
      if (skip) {
        setCorrect(null);
        setChecked(true);
        return;
      }
      const typed = input.trim();
      if (typed === "") return;
      const ok =
        mode === "pinyin"
          ? current.pinyin != null &&
            normPinyin(typed) === normPinyin(current.pinyin)
          : typed === current.headword;
      setCorrect(ok);
      setChecked(true);
    },
    [current, checked, input, mode],
  );

  const grade = useCallback(
    (rating: Rating) => {
      if (!current || busy) return;
      setBusy(true);
      reviewCard(current.id, rating)
        .catch((e) => setError(String(e)))
        .finally(() => {
          setReviewed((n) => n + 1);
          setIdx((i) => i + 1);
          setBusy(false);
        });
    },
    [current, busy],
  );

  // Number keys rate once the answer is checked.
  useEffect(() => {
    if (!current || !checked) return;
    const onKey = (e: KeyboardEvent) => {
      if (["1", "2", "3", "4"].includes(e.key)) grade(Number(e.key) as Rating);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current, checked, grade]);

  const modeToggle = (
    <div className={styles.toggle}>
      <Button
        size="sm"
        variant={mode === "pinyin" ? "secondary" : "ghost"}
        aria-pressed={mode === "pinyin"}
        onClick={() => setMode("pinyin")}
      >
        Pinyin
      </Button>
      <Button
        size="sm"
        variant={mode === "chars" ? "secondary" : "ghost"}
        aria-pressed={mode === "chars"}
        onClick={() => setMode("chars")}
      >
        字
      </Button>
    </div>
  );

  return (
    <main className={styles.screen}>
      <header className={styles.header}>
        <div className={styles.title}>
          <span className={styles.zh} lang="zh">
            复习
          </span>
          <span className={styles.en}>Review</span>
        </div>
        {current && <span className={styles.progress}>{remaining} left</span>}
      </header>

      {error ? (
        <Panel label="Review">
          <p className={styles.message}>Could not load the queue: {error}</p>
        </Panel>
      ) : !queue ? (
        <Panel label="Review">
          <p className={styles.message}>Loading…</p>
        </Panel>
      ) : queue.length === 0 ? (
        <Panel label="Review" actions={<Tag variant="jade">clear</Tag>}>
          <p className={styles.message}>Nothing due right now. Well done.</p>
        </Panel>
      ) : done ? (
        <Panel label="Session complete" actions={<Tag variant="jade">done</Tag>}>
          <p className={styles.message}>Reviewed {reviewed} cards.</p>
        </Panel>
      ) : (
        current && (
          <Panel label="Recall" actions={modeToggle}>
            <div className={styles.card}>
              {/* Prompt */}
              {mode === "pinyin" ? (
                <div className={styles.headword} lang="zh">
                  {current.headword}
                </div>
              ) : (
                <>
                  {current.pinyin && (
                    <div className={styles.promptPy}>{current.pinyin}</div>
                  )}
                  {current.gloss && (
                    <div className={styles.promptGloss}>{current.gloss}</div>
                  )}
                </>
              )}

              {!checked ? (
                <div className={styles.inputRow}>
                  <input
                    ref={inputRef}
                    className={cn(
                      styles.input,
                      mode === "chars" && styles.inputChars,
                    )}
                    value={input}
                    lang={mode === "chars" ? "zh" : undefined}
                    autoComplete="off"
                    autoCapitalize="off"
                    spellCheck={false}
                    placeholder={mode === "pinyin" ? "type the pinyin" : "type the characters"}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        check();
                      }
                    }}
                  />
                  <div className={styles.inputButtons}>
                    <Button variant="primary" onClick={() => check()}>
                      Check
                    </Button>
                    <Button variant="quiet" onClick={() => check(true)}>
                      Reveal
                    </Button>
                  </div>
                </div>
              ) : (
                <>
                  <div className={styles.answer}>
                    {correct != null && (
                      <Tag variant={correct ? "jade" : "correction"}>
                        {correct ? "correct" : "not quite"}
                      </Tag>
                    )}
                    {mode === "chars" && (
                      <div className={styles.headword} lang="zh">
                        {current.headword}
                      </div>
                    )}
                    {current.pinyin && (
                      <span className={styles.answerPy}>{current.pinyin}</span>
                    )}
                    {current.gloss && (
                      <span className={styles.answerGloss}>{current.gloss}</span>
                    )}
                  </div>

                  <div className={styles.grades}>
                    {GRADES.map((g) => (
                      <Button
                        key={g.rating}
                        variant={g.variant}
                        size="sm"
                        disabled={busy}
                        onClick={() => grade(g.rating)}
                      >
                        <span className={styles.gradeInner}>
                          <span>{g.label}</span>
                          <span className={styles.gradeInt}>
                            {fmtInterval(current[g.key])}
                          </span>
                        </span>
                      </Button>
                    ))}
                  </div>
                </>
              )}
            </div>
          </Panel>
        )
      )}

      {current && (
        <p className={styles.hint}>
          {checked ? "Rate 1-4 · how well did you recall it?" : "Enter to check"}
        </p>
      )}
    </main>
  );
}
