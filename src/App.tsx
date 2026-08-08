import { AppShell } from "./components/layout/AppShell";
import { ImageImportQueue } from "./features/image-import/ImageImportQueue";

function App() {
  return (
    <AppShell>
      <ImageImportQueue />
    </AppShell>
  );
}

export default App;
