import type { ModelRunResult } from '../../lib/api';
import { TerminalSurface } from './TerminalSurface';

interface TerminalCanvasProps {
  sessionId: string;
  result: ModelRunResult | null;
  error: string;
}

export function TerminalCanvas({ sessionId, result, error }: TerminalCanvasProps) {
  return (
    <section className="terminal-canvas">
      <TerminalSurface sessionId={sessionId} modelOutput={result} error={error} />
    </section>
  );
}
