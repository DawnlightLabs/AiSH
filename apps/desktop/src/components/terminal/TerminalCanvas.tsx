import { WelcomeMark } from './WelcomeMark';

export function TerminalCanvas(props: any) {
  const entries = props.entries || [];
  const cwd = String(props.cwd || '~').replace(/\\/g, '/').split('/').slice(-2).join('/');
  const hasContent = entries.length > 0;

  return (
    <section className="terminal-canvas" onContextMenu={(event) => event.preventDefault()}>
      {!hasContent && <WelcomeMark cwd={props.cwd} />}
      {entries.map((entry: any) => {
        const rows = [];
        if (entry.command) rows.push(entry.command);
        if (entry.status === 'running') rows.push('AiSH is thinking...');
        if (entry.output) rows.push(entry.output);
        if (entry.error) rows.push(entry.error);
        if (entry.status === 'blocked' && entry.reason) rows.push(`held: ${entry.reason}`);
        return (
          <div key={entry.id} className="terminal-entry">
            <div className="terminal-prompt"><span>PS</span> {cwd} {'>'} <strong>{entry.intent}</strong></div>
            <pre>{rows.join('\n').trimEnd()}</pre>
          </div>
        );
      })}
    </section>
  );
}
