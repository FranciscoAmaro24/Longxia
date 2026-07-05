import { useState } from "react";
import { AppShell } from "./app/AppShell/AppShell";
import { NAV_ITEMS, type SectionId } from "./app/nav";
import { TodayScreen } from "./features/today/TodayScreen";
import { ReaderScreen } from "./features/reader/ReaderScreen";
import { PlaceholderScreen } from "./features/PlaceholderScreen";

/** One-line notes for the not-yet-built sections. */
const SECTION_NOTES: Partial<Record<SectionId, string>> = {
  write: "田字格 canvas: stroke-order animation, guided tracing, AI structure feedback.",
  notebook: "Freeform notes; highlight a span for red-pen AI insight; send to SRS.",
  speak: "Text-to-speech, recording, and per-syllable tone scoring.",
  review: "The FSRS review queue over your due cards.",
};

function App() {
  const [active, setActive] = useState<SectionId>("today");

  const item = NAV_ITEMS.find((n) => n.id === active)!;

  return (
    <AppShell active={active} onSelect={setActive}>
      {active === "today" ? (
        <TodayScreen onNavigate={setActive} />
      ) : active === "read" ? (
        <ReaderScreen />
      ) : (
        <PlaceholderScreen zh={item.zh} en={item.en} note={SECTION_NOTES[active]} />
      )}
    </AppShell>
  );
}

export default App;
