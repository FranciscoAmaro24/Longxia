import { useState } from "react";
import { AppShell } from "./app/AppShell/AppShell";
import { NAV_ITEMS, type SectionId } from "./app/nav";
import { TodayScreen } from "./features/today/TodayScreen";
import { ReaderScreen } from "./features/reader/ReaderScreen";
import { WritingScreen } from "./features/writing/WritingScreen";
import { ReviewScreen } from "./features/review/ReviewScreen";
import { PlaceholderScreen } from "./features/PlaceholderScreen";

/** One-line notes for the not-yet-built sections. */
const SECTION_NOTES: Partial<Record<SectionId, string>> = {
  notebook: "Freeform notes; highlight a span for red-pen AI insight; send to SRS.",
  speak: "Text-to-speech, recording, and per-syllable tone scoring.",
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
      ) : active === "write" ? (
        <WritingScreen />
      ) : active === "review" ? (
        <ReviewScreen />
      ) : (
        <PlaceholderScreen zh={item.zh} en={item.en} note={SECTION_NOTES[active]} />
      )}
    </AppShell>
  );
}

export default App;
