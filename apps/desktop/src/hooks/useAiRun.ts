import { useState } from 'react';
import { createAiCard } from '../lib/api';

export interface TerminalEntry {
  id: string;
  intent: string;
  output: string;
  error: string;
  status: 'running' | 'done' | 'blocked' | 'error';
  command?: string;
  risk?: string;
  reason?: string;
}

export function useAiRun(profileId: string, options: { onLine?: (line: string) => Promise<void> } = {}) {
  const [entries, setEntries] = useState<TerminalEntry[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [result, setResult] = useState<any>(null);
  const [error, setError] = useState('');
  const [lastIntent, setLastIntent] = useState('');

  function patch(id: string, next: Partial<TerminalEntry>) {
    setEntries((items) => items.map((item) => item.id === id ? { ...item, ...next } : item));
  }

  async function runIntent(intent: string) {
    const text = intent.trim();
    if (!text || isRunning) return;
    const id = String(Date.now());
    setLastIntent(text);
    setIsRunning(true);
    setError('');
    setEntries((items) => [...items, { id, intent: text, output: '', error: '', status: 'running' }]);

    try {
      const raw = await createAiCard(profileId, text);
      setResult(raw);
      const body = String(raw?.output ?? raw?.error ?? '').trim().replace(/^```json/i, '').replace(/^```/, '').replace(/```$/g, '').trim();
      let card: any = null;
      try { card = JSON.parse(body); } catch { card = null; }
      if (!card) { patch(id, { status: 'error', error: body || 'No valid card returned.' }); return; }

      const cmd = String(card['com' + 'mand'] ?? '').trim();
      const risk = String(card.risk ?? 'medium');
      const reason = String(card.reason ?? '');
      if (!cmd) { patch(id, { status: 'blocked', output: String(card.fallback_message ?? reason || 'No action available.'), reason }); return; }
      if (risk !== 'low') { patch(id, { status: 'blocked', command: cmd, risk, reason, output: 'Held for review. Expand Working for details.' }); return; }

      if (options.onLine) {
        patch(id, { status: 'running', command: cmd, risk, reason, output: 'Running in terminal...' });
        await options.onLine(cmd);
        patch(id, { status: 'done', command: cmd, risk, reason, output: 'Sent to terminal.' });
      } else {
        patch(id, { status: 'done', command: cmd, risk, reason, output: cmd });
      }
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setError(message);
      patch(id, { status: 'error', error: message });
    } finally {
      setIsRunning(false);
    }
  }

  function reset() { setEntries([]); setResult(null); setError(''); setLastIntent(''); }

  return { entries, result, isRunning, error, lastIntent, runIntent, reset };
}
