import { AppShell } from "./components/layout/AppShell";

function App() {
  return (
    <AppShell>
      <section className="welcome" aria-labelledby="welcome-title">
        <div className="welcome__mark" aria-hidden="true">
          PC
        </div>
        <p className="welcome__eyebrow">Private by design</p>
        <h1 id="welcome-title">Your images stay on your device.</h1>
        <p className="welcome__description">
          Convert, compress, and resize images locally with PixChisel.
        </p>
        <div className="welcome__placeholder" aria-label="Image workspace coming soon">
          <span>Image workspace coming in Phase 1</span>
        </div>
      </section>
    </AppShell>
  );
}

export default App;
