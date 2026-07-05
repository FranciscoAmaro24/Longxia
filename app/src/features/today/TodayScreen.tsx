import { useEffect, useState } from "react";
import { Button, Panel, ProgressRing, Tag, TianGrid } from "../../components";
import type { SectionId } from "../../app/nav";
import { getTodaySummary, type TodaySummary } from "../../lib/api";
import styles from "./TodayScreen.module.css";

export interface TodayScreenProps {
  onNavigate: (id: SectionId) => void;
}

const pct = (learned: number, target: number) =>
  target > 0 ? Math.round((learned / target) * 100) : 0;

/**
 * Today: the home dashboard. Progress rings and the due count come from the
 * SQLite core via `get_today_summary`; the rest is static for now. Renders a
 * quiet loading and error state so a slow or failed load never blanks the UI.
 */
export function TodayScreen({ onNavigate }: TodayScreenProps) {
  const [summary, setSummary] = useState<TodaySummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    getTodaySummary()
      .then((s) => alive && setSummary(s))
      .catch((e) => alive && setError(String(e)));
    return () => {
      alive = false;
    };
  }, []);

  return (
    <main className={styles.screen}>
      <header className={styles.header}>
        <div>
          <div className={styles.eyebrow}>2026 · 07 · 05 · 星期六</div>
          <div className={styles.title}>
            <span className={styles.titleZh} lang="zh">
              继续学习
            </span>
            <span className={styles.titleEn}>Keep going</span>
          </div>
        </div>
        <Tag variant="jade">连续 7 天 · 7-day streak</Tag>
      </header>

      <Panel
        label="HSK progress"
        actions={
          <Tag variant="ink">
            {summary ? `Level ${summary.level}` : "Level —"}
          </Tag>
        }
      >
        {error ? (
          <p className={styles.metricText}>Could not load progress: {error}</p>
        ) : summary ? (
          <div className={styles.rings}>
            {summary.rings.map((r) => (
              <ProgressRing
                key={r.key}
                value={pct(r.learned, r.target)}
                size={76}
                zh={r.zh}
                count={`${r.learned} / ${r.target}`}
              />
            ))}
          </div>
        ) : (
          <p className={styles.metricText}>Loading…</p>
        )}
      </Panel>

      <div className={styles.duo}>
        <Panel label="Due today">
          <div className={styles.metric}>
            <span className={styles.metricNum}>{summary ? summary.due : "—"}</span>
            <span className={styles.metricText}>
              cards due · {summary ? summary.newCards : "—"} new
            </span>
          </div>
          <Button variant="primary" onClick={() => onNavigate("review")}>
            Start review
          </Button>
        </Panel>

        <Panel label="Continue writing">
          <div className={styles.continue}>
            <TianGrid char="写" size={72} tone="ghost" />
            <div className={styles.continueText}>
              <p>Stroke practice · handwrite set</p>
              <Button variant="secondary" onClick={() => onNavigate("write")}>
                Open
              </Button>
            </div>
          </div>
        </Panel>
      </div>

      <Panel label="Practice">
        <div className={styles.actions}>
          <Button variant="secondary" onClick={() => onNavigate("read")}>
            阅读 Read
          </Button>
          <Button variant="secondary" onClick={() => onNavigate("write")}>
            书写 Write
          </Button>
          <Button variant="secondary" onClick={() => onNavigate("speak")}>
            口语 Speak
          </Button>
          <Button variant="ghost" onClick={() => onNavigate("notebook")}>
            笔记 Notebook
          </Button>
        </div>
      </Panel>
    </main>
  );
}
