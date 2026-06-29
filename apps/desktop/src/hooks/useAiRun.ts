import { useState } from 'react';
import { createAiCard, executeShellCommand, type ModelRunResult } from '../lib/api';

type EntryStatus = 'running' | 'done' | 'blocked' | 'error';

interface CommandCard {
  action_type?: string;
  command?: string;
  risk?: string;
  reason?: string;
  fallback_message?: string;
}

export interface TerminalEntry {
  id: string;
  intent: string;
  command?: string;
  output: string;
  error: string;
  reason?: string;
  risk?: string;
  status: EntryStatus;
}

function entryId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function parseCard(result: ModelRunResult | null): CommandCard | null {
  const raw = String(result?.output ?? '').trim();
  if (!raw) return null;
  const cleaned = raw.replace(/^```json/i, '').replace(/^```/, '').replace(/```$/g, '').trim();
  try {
    return JSON.parse(cleaned) as CommandCard;
  } catch {
    return null;
  }
}

function deterministicCommand(intent: string): CommandCard | null {
  const text = intent.trim().toLowerCase();

  if (/^(pwd|where am i|show current directory|current directory|show current folder|current folder)$/.test(text)) {
    return { action_type: 'command', command: 'Get-Location', risk: 'low', reason: 'Prints the current working directory.' };
  }

  if (/^(dir|ls|list files|show files|list current directory|show current directory files)$/.test(text)) {
    return { action_type: 'command', command: 'Get-ChildItem', risk: 'low', reason: 'Lists files in the current directory.' };
  }

  if (/^(git status|show git status|check git status)$/.test(text)) {
    return { action_type: 'command', command: 'git status', risk: 'low', reason: 'Shows repository status without changing files.' };
  }

  if (/^(show npm scripts|list npm scripts|npm scripts)$/.test(text)) {
    return { action_type: 'command', command: 'npm run', risk: 'low', reason: 'Lists npm scripts from package.json.' };
  }

  const port = text.match(/(?:port|using port)\s+(\d{2,5})/);
  if (port) {
    return {
      action_type: 'command',
      command: `Get-NetTCPConnection -LocalPort ${port[1]} -ErrorAction SilentlyContinue | Select-Object LocalAddress,LocalPort,State,OwningProcess`,
      risk: 'low',
      reason: 'Reads local TCP connection information for the requested port.',
    };
  }

  return null;
}

function traceOutput(trace: any) {
  const stdout = String(trace?.output ?? '').trimEnd();
  const stderr = String(trace?.error ?? '').trimEnd();
  return { stdout, stderr };
}

export function useAiRun(selectedProfileId: string) {
  const [entries, setEntries] = useState<TerminalEntry[]>([]);
  const [result, setResult] = useState<ModelRunResult | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [error, setError] = useState('');
  const [lastIntent, setLastIntent] = useState('');

  function updateEntry(id: string, patch: Partial<TerminalEntry>) {
    setEntries((current) => current.map((entry) => entry.id === id ? { ...entry, ...patch } : entry));
  }

  async function runCard(id: string, card: CommandCard) {
    const command = String(card.command ?? '').trim();
    if (!command) {
      updateEntry(id, { status: 'error', error: card.fallback_message || card.reason || 'No command returned.', reason: card.reason });
      return;
    }

    if (card.risk && card.risk !== 'low') {
      updateEntry(id, {
        status: 'blocked',
        command,
        risk: card.risk,
        reason: card.reason,
        output: `Proposed command not run automatically:\n${command}`,
      });
      return;
    }

    updateEntry(id, { command, risk: card.risk || 'low', reason: card.reason });
    const trace = await executeShellCommand(command, false);
    const { stdout, stderr } = traceOutput(trace);
    updateEntry(id, {
      status: stderr ? 'error' : 'done',
      output: stdout || '(no output)',
      error: stderr,
    });
  }

  async function runIntent(intent: string) {
    const trimmed = intent.trim();
    if (!trimmed || isRunning) return;

    const id = entryId();
    setLastIntent(trimmed);
    setIsRunning(true);
    setError('');
    setResult(null);
    setEntries((current) => [...current, { id, intent: trimmed, output: '', error: '', status: 'running' }]);

    try {
      const local = deterministicCommand(trimmed);
      if (local) {
        await runCard(id, local);
        return;
      }

      if (!selectedProfileId) {
        updateEntry(id, { status: 'error', error: 'No model profile selected. Open Settings and choose a model.' });
        return;
      }

      const next = await createAiCard(selectedProfileId, trimmed);
      setResult(next);
      const card = parseCard(next);

      if (!card) {
        updateEntry(id, { status: 'error', error: String(next.output ?? next.error ?? 'Model did not return a command card.') });
        return;
      }

      if (card.action_type === 'fallback_message') {
        updateEntry(id, { status: 'blocked', output: card.fallback_message || card.reason || 'No command available.', reason: card.reason });
        return;
      }

      await runCard(id, card);
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setError(message);
      updateEntry(id, { status: 'error', error: message });
    } finally {
      setIsRunning(false);
    }
  }

  function reset() {
    setEntries([]);
    setResult(null);
    setError('');
    setLastIntent('');
  }

  return { entries, result, isRunning, error, lastIntent, runIntent, reset };
}
