import { useEffect, useRef, useState } from "react";
import { Button, Panel, Tag } from "../../components";
import { PHRASES } from "./phrases";
import { speak, tonePath, toneOf, ttsAvailable } from "./audio";
import styles from "./SpeakingScreen.module.css";

/**
 * Speaking: shadow a phrase with the system Chinese voice, see each syllable's
 * tone contour, and record yourself to compare by ear. Automated tone scoring
 * is deferred (needs reliable in-app speech recognition).
 */
export function SpeakingScreen() {
  const [pi, setPi] = useState(0);
  const [slow, setSlow] = useState(false);
  const [recording, setRecording] = useState(false);
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [recError, setRecError] = useState<string | null>(null);
  const mediaRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);

  const phrase = PHRASES[pi];
  const rate = slow ? 0.6 : 1;

  // Warm the voice list; reset audio when the phrase changes.
  useEffect(() => {
    if (ttsAvailable()) window.speechSynthesis.getVoices();
  }, []);

  useEffect(() => {
    setRecError(null);
    setAudioUrl((prev) => {
      if (prev) URL.revokeObjectURL(prev);
      return null;
    });
  }, [pi]);

  const startRec = async () => {
    setRecError(null);
    const md = navigator.mediaDevices;
    if (!md?.getUserMedia || typeof MediaRecorder === "undefined") {
      setRecError("Recording isn't available in this environment.");
      return;
    }
    try {
      const stream = await md.getUserMedia({ audio: true });
      const mr = new MediaRecorder(stream);
      chunksRef.current = [];
      mr.ondataavailable = (e) => {
        if (e.data.size) chunksRef.current.push(e.data);
      };
      mr.onstop = () => {
        const blob = new Blob(chunksRef.current, {
          type: mr.mimeType || "audio/webm",
        });
        setAudioUrl((prev) => {
          if (prev) URL.revokeObjectURL(prev);
          return URL.createObjectURL(blob);
        });
        stream.getTracks().forEach((t) => t.stop());
      };
      mr.start();
      mediaRef.current = mr;
      setRecording(true);
    } catch {
      setRecError("Microphone permission was denied or is unavailable.");
    }
  };

  const stopRec = () => {
    mediaRef.current?.stop();
    setRecording(false);
  };

  return (
    <main className={styles.screen}>
      <header className={styles.header}>
        <span className={styles.zh} lang="zh">
          口语
        </span>
        <span className={styles.en}>Speaking</span>
      </header>

      <Panel label="Shadow" actions={<Tag>Level 1-3</Tag>}>
        <div className={styles.switcher}>
          {PHRASES.map((p, i) => (
            <Button
              key={p.id}
              size="sm"
              variant={i === pi ? "secondary" : "ghost"}
              aria-pressed={i === pi}
              onClick={() => setPi(i)}
            >
              {p.tokens
                .slice(0, 3)
                .map((t) => t.char)
                .join("")}
              …
            </Button>
          ))}
        </div>

        <div className={styles.phrase} lang="zh">
          {phrase.tokens.map((t, i) => (
            <button
              key={i}
              type="button"
              className={styles.token}
              onClick={() => speak(t.char, rate)}
              aria-label={`Play ${t.pinyin}`}
            >
              <svg
                className={styles.contour}
                width="24"
                height="16"
                viewBox="0 0 24 16"
                aria-hidden="true"
              >
                <path
                  d={tonePath(toneOf(t.pinyin))}
                  stroke="currentColor"
                  strokeWidth="2"
                  fill="none"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
              <span className={styles.tokenChar}>{t.char}</span>
              <span className={styles.tokenPy}>{t.pinyin}</span>
            </button>
          ))}
        </div>

        <p className={styles.translation}>{phrase.translation}</p>

        <div className={styles.controls}>
          <Button variant="primary" onClick={() => speak(phrase.text, rate)}>
            Play
          </Button>
          <Button
            variant={slow ? "secondary" : "ghost"}
            aria-pressed={slow}
            onClick={() => setSlow((s) => !s)}
          >
            Slow
          </Button>
        </div>
      </Panel>

      <Panel label="Record yourself">
        <div className={styles.recRow}>
          {recording ? (
            <Button variant="accent" onClick={stopRec}>
              Stop
            </Button>
          ) : (
            <Button variant="secondary" onClick={startRec}>
              Record
            </Button>
          )}
          {recording && <span className={styles.recDot}>● recording</span>}
          {audioUrl && (
            <audio className={styles.player} src={audioUrl} controls />
          )}
        </div>
        {recError && <p className={styles.error}>{recError}</p>}
        <p className={styles.note}>
          Uses your system Chinese voice. Play the phrase, then record and
          compare by ear.
        </p>
      </Panel>
    </main>
  );
}
