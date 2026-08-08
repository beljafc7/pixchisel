import { useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import { ImageImportQueue } from "./features/image-import/ImageImportQueue";
import { WorkflowSelector } from "./features/workflows/WorkflowSelector";
import type { WorkflowMode } from "./features/workflows/workflow";

function App() {
  const [workflow, setWorkflow] = useState<WorkflowMode | null>(null);
  return (
    <AppShell>
      {workflow ? (
        <ImageImportQueue workflow={workflow} onChangeWorkflow={() => setWorkflow(null)} />
      ) : (
        <WorkflowSelector onSelect={setWorkflow} />
      )}
    </AppShell>
  );
}

export default App;
