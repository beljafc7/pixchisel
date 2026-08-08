import type { PropsWithChildren } from "react";

export function AppShell({ children }: PropsWithChildren) {
  return (
    <div className="app-shell">
      <header className="app-header">
        <span className="app-header__name">PixChisel</span>
        <span className="app-header__tagline">Convert. Compress. Resize. Locally.</span>
      </header>
      <main className="app-content">{children}</main>
      <footer className="app-footer">
        <span className="status-dot" aria-hidden="true" />
        Offline and ready
      </footer>
    </div>
  );
}
