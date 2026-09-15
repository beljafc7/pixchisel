import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState, type PropsWithChildren } from "react";
import pixChiselIcon from "../../../src-tauri/icons/64x64.png";

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
        <div className="app-brand">
          <span className="app-brand__mark"><img src={pixChiselIcon} alt="" /></span>
          <span className="app-brand__copy">
            <strong>PixChisel</strong>
            <span>Image optimizer</span>
          </span>
        </div>
        <div className="app-header__meta">
          <span className="privacy-badge"><span aria-hidden="true" />Fast, private, local</span>
          {version && <span className="app-header__version">v{version}</span>}
        </div>
      </header>
      <main className="app-content">{children}</main>
      <footer className="app-footer">
        <span className="app-footer__status">
          <span className="status-dot" aria-hidden="true" />
          Offline and ready
        </span>
        <span className="app-footer__tagline">Convert. Compress. Resize. Locally.</span>
      </footer>
    </div>
  );
}
