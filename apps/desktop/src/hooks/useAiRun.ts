import { useState } from 'react';
import { createAiCard } from '../lib/api';

export interface TerminalEntry {
  id: string;
  intent: string;
  output: string;
  error: string;
  status: 'running' | 'done' | 'blocked' | 'error' | 'approval';
  command?: string;
  risk?: string;
  reason?: string;
  needsApproval?: boolean;
}

function cleanJson(text: string) {
  return text.trim().replace(/^```json/i, '').replace(/^```/, '').replace(/```$/g, '').trim();
}

function isDestructive(command: string) {
  const value = command.toLowerCase();
  const patterns = [
    'remove-item', ' rm ', 'del ', 'erase ', 'rmdir ', 'remove-', 'clear-content',
    'set-content', 'add-content', 'out-file', 'move-item', 'rename-item', 'copy-item',
    'git reset', 'git clean', 'git push', 'npm publish', 'deploy', 'format ',
    'reg add', 'reg delete', 'set-acl', 'icacls', 'takeown', 'chmod', 'chown',
    'stop-process', 'kill ', 'shutdown', 'restart-computer', 'stop-service', 'set-service',
    'npm install', 'pnpm install', 'yarn install', 'pip install', 'cargo install',
    'docker system prune', 'kubectl delete', 'terraform apply', 'aws ', 'az ', 'gcloud '
  ];
  const padded = ` ${value} `;
  return patterns.some((pattern) => padded.includes(pattern));
}

function isReadOnlyInspection(command: string) {
  const value = command.trim().toLowerCase();
  if (isDestructive(command)) return false;
  const readOnly = [
    'get-', 'dir', 'ls', 'pwd', 'where.exe', 'where ', 'select-string', 'type ', 'cat ',
    'findstr', 'git status', 'git log', 'git diff', 'npm run', 'npm list', 'node -v',
    'npm -v', 'python --version', 'pip --version', 'ipconfig', 'netstat', 'tasklist'
  ];
  return readOnly.some((prefix) => value.startsWith(prefix)) || value.includes('| select-object') || value.includes('| sort-object');
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

  async function sendToTerminal(id: string, command: string, risk: string, reason: string) {
    if (!options.onLine) {
      patch(id, { status: 'error', command, risk, reason, error: 'Terminal session is not ready.' });
      return;
    }
    patch(id, { status: 'running', command, risk, reason, output: 'Running in terminal...' });
    await options.onLine(command);
    patch(id, { status: 'done', command, risk, reason, output: 'Sent to terminal.' });
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
      const body = cleanJson(String(raw?.output ?? raw?.error ?? ''));
      let card: any = null;
      try { card = JSON.parse(body); } catch { card = null; }
      if (!card) { patch(id, { status: 'error', error: body || 'No valid card returned.' }); return; }

      const command = String(card['com' + 'mand'] ?? '').trim();
      const modelRisk = String(card.risk ?? 'medium').toLowerCase();
      const reason = String(card.reason ?? '');

      if (!command) {
        const message = String(card.fallback_message ?? (reason || 'No action available.'));
        patch(id, { status: 'blocked', output: message, reason });
        return;
      }

      const destructive = isDestructive(command);
      const readOnly = isReadOnlyInspection(command);
      const risk = destructive ? (modelRisk === 'high' ? 'high' : 'medium') : (readOnly ? 'low' : modelRisk);

      if (destructive || (risk !== 'low' && !readOnly)) {
        patch(id, {
          status: 'approval',
          command,
          risk,
          reason,
          needsApproval: true,
          output: 'Approval required. Expand Working to approve or cancel.',
        });
        return;
      }

      await sendToTerminal(id, command, risk, reason);
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setError(message);
      patch(id, { status: 'error', error: message });
    } finally {
      setIsRunning(false);
    }
  }

  async function approveEntry(id: string) {
    const entry = entries.find((item) => item.id === id);
    if (!entry?.command) return;
    await sendToTerminal(id, entry.command, entry.risk || 'medium', entry.reason || 'Approved by user.');
  }

  function cancelEntry(id: string) {
    patch(id, { status: 'blocked', needsApproval: false, output: 'Cancelled.' });
  }

  function reset() { setEntries([]); setResult(null); setError(''); setLastIntent(''); }

  return { entries, result, isRunning, error, lastIntent, runIntent, approveEntry, cancelEntry, reset };
}
