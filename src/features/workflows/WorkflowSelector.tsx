import { workflowCopy, type WorkflowMode } from "./workflow";

export function WorkflowSelector({ onSelect }: { onSelect: (mode: WorkflowMode) => void }) {
  return (
    <section className="workflow-selector" aria-labelledby="workflow-title">
      <p className="welcome__eyebrow">PixChisel</p>
      <h1 id="workflow-title">What do you want to do?</h1>
      <div className="workflow-selector__choices">
        {(["compress", "convert", "resize"] as const).map((mode) => (
          <button key={mode} className="workflow-choice" type="button" onClick={() => onSelect(mode)}>
            <strong>{mode[0].toUpperCase() + mode.slice(1)}</strong>
            <span>{workflowCopy[mode].description}</span>
          </button>
        ))}
      </div>
      <p className="local-note">Processed locally. Nothing is uploaded.</p>
    </section>
  );
}
