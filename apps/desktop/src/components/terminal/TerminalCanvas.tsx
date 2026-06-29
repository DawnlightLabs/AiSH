import type { ModelRunResult } from '../../lib/api';
import { WelcomeMark } from './WelcomeMark';

interface TerminalCanvasProps {
  cwd: string;
  result: ModelRunResult | null;
  error: string;
  lastIntent: string;
  busy: boolean;
}

interface AiCommandCard {
  action_type?: string;
  command?: string;
  risk?: string;
  reason?: string;
  fallback_message?: string;
}

function shortPath(path: string) {
  return path.replace(/\\/g, '/').split('/').slice(-2).join('/');
}

function parseCard(text: string): AiCommandCard | null {
  const trimmed = text.trim().replace(/^```json/i, '').replace(/^```/, '').replace(/```$/g, '').trim();
  try {
    return JSON.parse(trimmed) as AiCommandCard;
  } catch {
    return null;
  }
}

function resultText(result: ModelRunResult | null) {
  if (!result) return '';
  const out = String(result.output ?? '').trim();
  const err = String(result.error ?? '').trim();
  return out || err || 'No response returned.';
}

function renderResponse(result: ModelRunResult | null) {
  const text = resultText(result);
  const card = parseCard(text);

  if (!card) return text;

  if (card.action_type === 'command' && card.command) {
    return [
      card.command,
      '',
      `risk: ${card.risk || 'unknown'}`,
      card.reason ? `reason: ${card.reason}` : '',
    ].filter(Boolean).join('\n');
  }

  if (card.action_type === 'fallback_message') {
    return card.fallback_message || card.reason || text;
  }

  return text;
}

export function TerminalCanvas(props: TerminalCanvasProps) {
  const hasContent = Boolean(props.result || props.error || props.busy);
  const prompt = `PS ${shortPath(props.cwd)}> ${props.lastIntent || 'ask AiSH'}`;

  return (
    <section className="terminal-canvas" onContextMenu={(event) => event.preventDefault()}>
      {!hasContent && <WelcomeMark cwd={props.cwd} />}
      {props.busy && (
        <article className="command-block">
          <pre><span className="prompt-line">{prompt}</span>{'\n'}AiSH is thinking...</pre>
        </article>
      )}
      {props.error && (
        <article className="command-block error-block">
          <pre><span className="prompt-line">{prompt}</span>{'\n'}{props.error}</pre>
        </article>
      )}
      {props.result && (
        <article className="command-block">
          <pre><span className="prompt-line">{prompt}</span>{'\n'}{renderResponse(props.result)}</pre>
        </article>
      )}
    </section>
  );
}
