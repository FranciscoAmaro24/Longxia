import { Button, Panel, Tag, TianGrid } from "./components";
import styles from "./App.module.css";

/**
 * Step 2 preview: a gallery of the design tokens + primitives so the look and
 * feel can be reviewed in the real app window. Replaced by the app shell and
 * the Today screen in Step 3. Follows OS light/dark automatically.
 */
function App() {
  return (
    <main className={styles.page}>
      <header className={styles.masthead}>
        <span className={styles.eyebrow}>Design system · Step 2</span>
        <div className={styles.wordmark}>
          <span className={styles.zh} lang="zh">
            龙虾
          </span>
          <span className={styles.py}>Lóngxiā · primitives</span>
        </div>
      </header>

      <section className={styles.section}>
        <h2 className={styles.sectionLabel}>Buttons</h2>
        <div className={styles.row}>
          <Button variant="primary">Review</Button>
          <Button variant="secondary">Read</Button>
          <Button variant="ghost">Add card</Button>
          <Button variant="accent">Explain</Button>
          <Button variant="quiet">Skip</Button>
          <Button variant="secondary" disabled>
            Disabled
          </Button>
        </div>
        <div className={styles.row}>
          <Button size="sm" variant="primary">
            Small
          </Button>
          <Button size="sm" variant="secondary">
            Small
          </Button>
          <Button size="sm" variant="ghost">
            Small
          </Button>
        </div>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionLabel}>Tags</h2>
        <div className={styles.row}>
          <Tag>Graded · L3</Tag>
          <Tag variant="ink">Level 3</Tag>
          <Tag variant="correction">AI insight</Tag>
          <Tag variant="jade">7-day streak</Tag>
        </div>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionLabel}>Type specimen</h2>
        <div className={styles.specimen}>
          <TianGrid char="学" size={120} />
          <div>
            <div className={styles.wordmark}>
              <span className={styles.zh} lang="zh">
                学习
              </span>
            </div>
            <div className={styles.py}>xuéxí · to study</div>
          </div>
        </div>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionLabel}>Panels &amp; the 田字格 cell</h2>
        <div className={styles.grid}>
          <Panel label="HSK progress" actions={<Tag variant="ink">L3</Tag>}>
            <div className={styles.row}>
              <TianGrid char="写" size={72} tone="ghost" />
              <TianGrid char="字" size={72} />
            </div>
          </Panel>

          <Panel
            label="Review"
            actions={<Button size="sm" variant="primary">Start</Button>}
          >
            <p>18 cards due · 6 new</p>
          </Panel>

          <Panel label="Notebook" actions={<Tag variant="correction">AI</Tag>}>
            <p>
              Highlight a span and the red pen annotates exactly what you point
              at.
            </p>
          </Panel>
        </div>
      </section>
    </main>
  );
}

export default App;
