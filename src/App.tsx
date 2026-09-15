import { useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import { ImageImportQueue } from "./features/image-import/ImageImportQueue";
import type { WorkflowMode } from "./features/workflows/workflow";

function App() {
  const [workflow, setWorkflow] = useState<WorkflowMode>("compress");
  return (
    <AppShell>
      <ImageImportQueue workflow={workflow} onChangeWorkflow={setWorkflow} />
    </AppShell>
  );
}

export default App;
