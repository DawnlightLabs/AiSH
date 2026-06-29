import { useEffect, useRef } from 'react';
import { WelcomeMark } from './WelcomeMark';

function compact(value: unknown) {
  return String(value ?? '')
    .replace(/\r\n/g, '\n')
    .replace(/\n[ \t]*\n[ \t]+Directory:/g, '\nDirectory:')
    .replace(/\n[ \t]+Directory:/g, '\nDirectory:')
    .trimEnd();
}

export function AiRunTerminal(props: any) {
  const items = props.entries || [];
  const cwd = String(props.cwd || '~').replace(/\\/g, '/').split('/').slice(-2).join('/');
  const ref = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [items.length, items[items.length - 1]?.output, items[items.length - 1]?.error]);

  return (
    <section ref={ref} className="terminal-canvas" onContextMenu={(event) => event.preventDefault()}>
      {items.length === 0 && <WelcomeMark cwd={props.cwd} />}
      {items.map((item: any) => {
        const shown = [];
        if (item.status === 'running') shown.push('AiSH is working...');
        if (item.output) shown.push(compact(item.output));
        if (item.error) shown.push(compact(item.error));
        if (item.status === 'blocked' && item.reason) shown.push(`held: ${item.reason}`);
        return (
          <div key={item.id} className={`terminal-entry status-${item.status}`}>
            <div className="terminal-prompt"><span>PS</span> {cwd} {'>'} <strong>{item.intent}</strong></div>
            <pre>{shown.filter(Boolean).join('\n').trimEnd()}</pre>
          </div>
        );
      })}
    </section>
  );
}
