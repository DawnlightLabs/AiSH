import type { CommandTrace } from '../types';
import type { ModelRunResult } from '../lib/api';

export function TracePanel({ trace, modelOutput, project }: { trace: CommandTrace | null; modelOutput: ModelRunResult | null; project: Record<string, unknown> }) {
  return (
    <section className="trace-card">
      <div className="trace-toggle">
        <span>Working</span>
        <small>Command Trace</small>
      </div>
      <div className="trace-body">
        {trace && <pre>{JSON.stringify(trace, null, 2)}</pre>}
        {modelOutput && <pre>{JSON.stringify(modelOutput, null, 2)}</pre>}
        <details>
          <summary>Project context</summary>
          <pre>{JSON.stringify(project, null, 2)}</pre>
        </details>
      </div>
    </section>
  );
}
