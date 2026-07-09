import { useState } from "react";
import { AppShell } from "./app/AppShell/AppShell";
import { type SectionId } from "./app/nav";
import { hasApiToken, isTauri } from "./lib/api";
import { AuthGate } from "./features/auth/AuthGate";
import { TodayScreen } from "./features/today/TodayScreen";
import { ReaderScreen } from "./features/reader/ReaderScreen";
import { WritingScreen } from "./features/writing/WritingScreen";
import { ReviewScreen } from "./features/review/ReviewScreen";
import { NotebookScreen } from "./features/notebook/NotebookScreen";
import { SpeakingScreen } from "./features/speaking/SpeakingScreen";

function App() {
  const [active, setActive] = useState<SectionId>("today");
  // In the browser, a session is needed to reach the server. The Tauri app talks
  // to the local core and never needs one, so it is always authed.
  const [authed, setAuthed] = useState(() => isTauri() || hasApiToken());

  if (!authed) {
    return <AuthGate onAuthed={() => setAuthed(true)} />;
  }

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
