import { useState } from "react";
import { AppShell } from "./app/AppShell/AppShell";
import { type SectionId } from "./app/nav";
import { TodayScreen } from "./features/today/TodayScreen";
import { ReaderScreen } from "./features/reader/ReaderScreen";
import { WritingScreen } from "./features/writing/WritingScreen";
import { ReviewScreen } from "./features/review/ReviewScreen";
import { NotebookScreen } from "./features/notebook/NotebookScreen";
import { SpeakingScreen } from "./features/speaking/SpeakingScreen";

function App() {
  const [active, setActive] = useState<SectionId>("today");

  return (
    <AppShell active={active} onSelect={setActive}>
      {active === "today" && <TodayScreen onNavigate={setActive} />}
      {active === "read" && <ReaderScreen />}
      {active === "write" && <WritingScreen />}
      {active === "review" && <ReviewScreen />}
      {active === "notebook" && <NotebookScreen />}
      {active === "speak" && <SpeakingScreen />}
    </AppShell>
  );
}

export default App;
