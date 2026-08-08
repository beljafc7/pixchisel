import { Component, type ErrorInfo, type PropsWithChildren, type ReactNode } from "react";

interface State {
  failed: boolean;
}

export class AppErrorBoundary extends Component<PropsWithChildren, State> {
  state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  componentDidCatch(_error: Error, _info: ErrorInfo): void {
    if (import.meta.env.DEV) {
      console.error("PixChisel interface error", {
        name: _error.name,
        message: _error.message,
        stack: _error.stack,
        componentStack: _info.componentStack,
      });
    }
  }

  render(): ReactNode {
    if (!this.state.failed) return this.props.children;
    return (
      <main className="interface-error" role="alert">
        <h1>PixChisel hit an unexpected interface error.</h1>
        <p>Your original images were not changed by this interface error.</p>
        <button className="primary-button" type="button" onClick={() => window.location.reload()}>
          Return to Start
        </button>
      </main>
    );
  }
}
