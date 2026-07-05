import { useCallback, useEffect, useState } from "react";
import { Button, Panel, Tag } from "../../components";
import {
  getReviewQueue,
  reviewCard,
  type Rating,
  type ReviewCard,
} from "../../lib/api";
import styles from "./ReviewScreen.module.css";

/** Seconds until due -> a short human label for the grade buttons. */
function fmtInterval(secs: number): string {
  if (secs < 60) return "<1m";
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  if (secs < 86400) return `${Math.round(secs / 3600)}h`;
  return `${Math.round(secs / 86400)}d`;
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
 * Review: an FSRS study loop. Reads the due queue, shows one card at a time,
 * reveals the answer, and reschedules on a rating via the Rust core. Keyboard:
 * Space reveals, 1-4 rate.
 */
export function ReviewScreen() {
  const [queue, setQueue] = useState<ReviewCard[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [idx, setIdx] = useState(0);
  const [revealed, setRevealed] = useState(false);
  const [reviewed, setReviewed] = useState(0);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    getReviewQueue()
      .then(setQueue)
      .catch((e) => setError(String(e)));
  }, []);

  const current = queue && idx < queue.length ? queue[idx] : null;
  const done = queue != null && idx >= queue.length && queue.length > 0;
  const remaining = queue ? queue.length - idx : 0;

  const grade = useCallback(
    (rating: Rating) => {
      if (!current || busy) return;
      setBusy(true);
      reviewCard(current.id, rating)
        .catch((e) => setError(String(e)))
        .finally(() => {
          setReviewed((n) => n + 1);
          setRevealed(false);
          setIdx((i) => i + 1);
          setBusy(false);
        });
    },
    [current, busy],
  );

  // Keyboard shortcuts.
  useEffect(() => {
    if (!current) return;
    const onKey = (e: KeyboardEvent) => {
      if (!revealed && (e.code === "Space" || e.key === "Enter")) {
        e.preventDefault();
        setRevealed(true);
      } else if (revealed && ["1", "2", "3", "4"].includes(e.key)) {
        grade(Number(e.key) as Rating);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current, revealed, grade]);

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
          <Panel label="Recall">
            <div className={styles.card}>
              <div className={styles.headword} lang="zh">
                {current.headword}
              </div>

              {!revealed ? (
                <Button variant="primary" onClick={() => setRevealed(true)}>
                  Show answer
                </Button>
              ) : (
                <>
                  <div className={styles.answer}>
                    <span className={styles.answerPy}>
                      {current.pinyin ?? "—"}
                    </span>
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
          {revealed ? "Rate 1-4 · how well did you recall it?" : "Space to reveal"}
        </p>
      )}
    </main>
  );
}
