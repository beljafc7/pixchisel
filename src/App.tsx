import { AppShell } from "./components/layout/AppShell";
import { ImageInspector } from "./features/image-inspection/ImageInspector";

function App() {
  return (
    <AppShell>
      <ImageInspector />
    </AppShell>
  );
}

export default App;
