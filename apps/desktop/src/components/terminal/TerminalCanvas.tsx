import type { ModelRunResult } from '../../lib/api';
import { WelcomeMark } from './WelcomeMark';

interface TerminalCanvasProps {
  cwd: string;
  result: ModelRunResult | null;
  error: string;
  lastIntent: string;
  busy: boolean;
}

export function TerminalCanvas(props: TerminalCanvasProps) {
  const hasContent = Boolean(props.result || props.error || props.busy);
  const stdout = String(props.result?.output ?? '').trim();
  const stderr = String(props.result?.error ?? '').trim();
  const body = stdout || stderr || 'No response returned.';

  return (
    <section className="terminal-canvas" onContextMenu={(event) => event.preventDefault()}>
      {!hasContent && <WelcomeMark cwd={props.cwd} />}
      {props.busy && (
        <article className="command-block">
          <div className="block-meta">AiSH</div>
          <pre>{props.lastIntent ? `Request: ${props.lastIntent}` : 'Processing request...'}</pre>
        </article>
      )}
      {props.error && (
        <article className="command-block error-block">
          <div className="block-meta">Error</div>
          <pre>{props.error}</pre>
        </article>
      )}
      {props.result && (
        <article className="command-block">
          <div className="block-meta">Result</div>
          <pre>{body}</pre>
        </article>
      )}
    </section>
  );
}
