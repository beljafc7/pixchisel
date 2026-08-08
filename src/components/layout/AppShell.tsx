import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState, type PropsWithChildren } from "react";

export function AppShell({ children }: PropsWithChildren) {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    void getVersion()
      .then(setVersion)
      .catch(() => undefined);
  }, []);

  return (
    <div className="app-shell">
      <header className="app-header">
        <span className="app-header__name">PixChisel</span>
        <span className="app-header__tagline">Convert. Compress. Resize. Locally.</span>
      </header>
      <main className="app-content">{children}</main>
      <footer className="app-footer">
        <span className="app-footer__status">
          <span className="status-dot" aria-hidden="true" />
          Offline and ready
        </span>
        {version && <span className="app-footer__version">v{version}</span>}
      </footer>
    </div>
  );
}
