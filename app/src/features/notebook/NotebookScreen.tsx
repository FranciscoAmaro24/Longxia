import { useEffect, useRef, useState } from "react";
import { Button, Tag } from "../../components";
import {
  addInsight,
  deleteInsight,
  explain,
  getNote,
  saveNote,
  type Insight,
} from "../../lib/api";
import styles from "./NotebookScreen.module.css";

interface Selection {
  start: number;
  end: number;
  text: string;
}

/**
 * Notebook: a freeform note that autosaves, plus red-pen AI insights. Select
 * any span and "Explain" sends it to Claude (via the Rust core); the insight is
 * saved and shown in the margin, bound to that span. Persistence and the AI
 * call both go through typed commands.
 */
export function NotebookScreen() {
  const [text, setText] = useState("");
  const [insights, setInsights] = useState<Insight[]>([]);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [explaining, setExplaining] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(true);
  const saveTimer = useRef<number | undefined>(undefined);

  useEffect(() => {
    getNote()
      .then((n) => {
        setText(n.text);
        setInsights(n.insights);
      })
      .catch((e) => setError(String(e)));
    return () => window.clearTimeout(saveTimer.current);
  }, []);

  const onChange = (value: string) => {
    setText(value);
    setSaved(false);
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveNote(value)
        .then(() => setSaved(true))
        .catch((e) => setError(String(e)));
    }, 700);
  };

  const onSelect = (el: HTMLTextAreaElement) => {
    const { selectionStart: s, selectionEnd: e } = el;
    if (e > s) {
      setSelection({ start: s, end: e, text: text.slice(s, e) });
    } else {
      setSelection(null);
    }
  };

  const runExplain = () => {
    if (!selection || explaining) return;
    setExplaining(true);
    setError(null);
    const { start, end, text: snippet } = selection;
    explain(snippet)
      .then((explanation) => addInsight(snippet, explanation, start, end))
      .then((insight) => {
        setInsights((prev) => [insight, ...prev]);
        setSelection(null);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setExplaining(false));
  };

  const remove = (id: number) => {
    deleteInsight(id)
      .then(() => setInsights((prev) => prev.filter((i) => i.id !== id)))
      .catch((e) => setError(String(e)));
  };

  return (
    <main className={styles.screen}>
      <header className={styles.header}>
        <div className={styles.title}>
          <span className={styles.zh} lang="zh">
            笔记
          </span>
          <span className={styles.en}>Notebook</span>
        </div>
        <span className={styles.saved}>{saved ? "saved" : "saving…"}</span>
      </header>

      <div className={styles.layout}>
        <div className={styles.editorCol}>
          <div className={styles.toolbar}>
            <Button
              variant="accent"
              size="sm"
              disabled={!selection || explaining}
              onClick={runExplain}
            >
              {explaining ? "Explaining…" : "Explain selection"}
            </Button>
            {selection && (
              <span className={styles.selInfo} lang="zh">
                {selection.text.length > 24
                  ? `${selection.text.slice(0, 24)}…`
                  : selection.text}
              </span>
            )}
          </div>

          <textarea
            className={styles.editor}
            lang="zh"
            value={text}
            placeholder="Write notes here. Select any Chinese text and press Explain."
            onChange={(e) => onChange(e.target.value)}
            onSelect={(e) => onSelect(e.currentTarget)}
          />

          {error && <p className={styles.error}>{error}</p>}
          <p className={styles.hint}>
            AI insights need ANTHROPIC_API_KEY set when the app launches.
          </p>
        </div>

        <aside className={styles.margin}>
          <span className={styles.marginLabel}>AI insights</span>
          {insights.length === 0 ? (
            <p className={styles.empty}>
              Highlight a word or sentence and press Explain to add a note here.
            </p>
          ) : (
            insights.map((i) => (
              <div key={i.id} className={styles.insight}>
                <div className={styles.insightHead}>
                  <span className={styles.snippet} lang="zh">
                    {i.snippet}
                  </span>
                  <Button variant="quiet" size="sm" onClick={() => remove(i.id)}>
                    Remove
                  </Button>
                </div>
                <p className={styles.explanation}>{i.explanation}</p>
              </div>
            ))
          )}
          {insights.length > 0 && <Tag variant="correction">red pen</Tag>}
        </aside>
      </div>
    </main>
  );
}
