import { Button, Panel, ProgressRing, Tag, TianGrid } from "../../components";
import type { SectionId } from "../../app/nav";
import styles from "./TodayScreen.module.css";

export interface TodayScreenProps {
  onNavigate: (id: SectionId) => void;
}

/**
 * Today: the home dashboard. Data is static placeholder for Step 3 - it gets
 * wired to the SQLite core in Step 4. Structure follows the approved wireframe:
 * summary (HSK rings) before detail, one primary action (Review).
 */

// Placeholder figures. Cumulative HSK 3 targets are illustrative until the
// official CTI lists are imported (see PLAN.md section 3).
const HSK_RINGS = [
  { zh: "汉字", value: 75, count: "674 / 900" },
  { zh: "词语", value: 52, count: "512 / 988" },
  { zh: "语法", value: 42, count: "88 / 210" },
  { zh: "音节", value: 66, count: "402 / 608" },
] as const;

export function TodayScreen({ onNavigate }: TodayScreenProps) {
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

      <Panel label="HSK progress" actions={<Tag variant="ink">Level 3</Tag>}>
        <div className={styles.rings}>
          {HSK_RINGS.map((r) => (
            <ProgressRing
              key={r.zh}
              value={r.value}
              size={76}
              zh={r.zh}
              count={r.count}
            />
          ))}
        </div>
      </Panel>

      <div className={styles.duo}>
        <Panel label="Due today">
          <div className={styles.metric}>
            <span className={styles.metricNum}>18</span>
            <span className={styles.metricText}>cards due · 6 new</span>
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
