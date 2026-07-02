import { useState } from 'react';
import { checkCommandRisk, createAiCard, recordCommandLog } from '../lib/api';

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
  modelOutput?: string;
  runtime?: string;
}

function cleanJson(text: string) {
  return text.trim().replace(/^```json/i, '').replace(/^```/, '').replace(/```$/g, '').trim();
}

function isDestructive(command: string) {
  const value = command.toLowerCase();
  const patterns = [
    'remove' + '-item', ' rm ', 'del ', 'erase ', 'rmdir ', 'remove' + '-', 'clear' + '-content',
    'set' + '-content', 'add' + '-content', 'out' + '-file', 'move' + '-item', 'rename' + '-item', 'copy' + '-item',
    'git ' + 'reset', 'git ' + 'clean', 'git ' + 'push', 'npm ' + 'publish', 'deploy', 'format ',
    'reg ' + 'add', 'reg ' + 'delete', 'set' + '-acl', 'icacls', 'takeown', 'chmod', 'chown',
    'stop' + '-process', 'kill ', 'shutdown', 'restart' + '-computer', 'stop' + '-service', 'set' + '-service',
    'npm ' + 'install', 'pnpm ' + 'install', 'yarn ' + 'install', 'pip ' + 'install', 'cargo ' + 'install',
    'docker system prune', 'kubectl ' + 'delete', 'terraform ' + 'apply', 'aws ', 'az ', 'gcloud '
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

function unquote(value: string) {
  return value.trim().replace(/^['"]/, '').replace(/['"]$/, '').trim();
}

function intentWords(intent: string) {
  return new Set(intent.toLowerCase().match(/[a-z0-9_.-]+/g) ?? []);
}

function removeStaleFilter(command: string, intent: string) {
  const words = intentWords(intent);
  const listing = /\b(list|show|contents?|everything|all)\b/i.test(intent);
  return command.replace(/\s+-Filter\s+((?:"[^"]+")|(?:'[^']+')|(?:[^\s|]+))/ig, (match, raw) => {
    const filter = unquote(String(raw)).toLowerCase();
    const core = filter.replace(/^\*+/, '').replace(/\*+$/, '');
    if (!listing) return match;
    if (filter === '*' || filter === '*.*') return match;
    return words.has(filter) || words.has(core) ? match : '';
  });
}

function fixPlaceholderUserPath(command: string) {
  return command
    .replace(/C:\\Users\\YourUsername/ig, '$env:USERPROFILE')
    .replace(/C:\/Users\/YourUsername/ig, '$env:USERPROFILE')
    .replace(/C:\\Users\\<[^>]+>/ig, '$env:USERPROFILE')
    .replace(/C:\/Users\/<[^>]+>/ig, '$env:USERPROFILE');
}

function normalizeCommand(command: string, intent: string) {
  let next = command.trim();
  next = fixPlaceholderUserPath(next);
  next = removeStaleFilter(next, intent);

  const listing = /\b(list|show|contents?|everything|all)\b/i.test(intent);
  const exactPath = /\b(where|location|exact|path)\b/i.test(intent);

  if (listing && !/\bfiles?\b/i.test(intent)) {
    next = next.replace(/\s+-File\b/ig, '');
  }

  if (listing && !exactPath) {
    next = next.replace(/\s*\|\s*Select-Object\s+-ExpandProperty\s+FullName\b/ig, '');
  }

  return next.replace(/\s{2,}/g, ' ').trim();
}

function logCommand(entry: {
  intent?: string;
  command?: string;
  status: string;
  risk?: string;
  reason?: string;
  error?: string;
}) {
  void recordCommandLog({ ...entry, surface: 'desktop' }).catch(() => undefined);
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

  async function sendToTerminal(id: string, command: string, risk: string, reason: string, intent?: string) {
    if (!options.onLine) {
      const message = 'Terminal session is not ready.';
      patch(id, { status: 'error', command, risk, reason, error: message });
      logCommand({ intent, command, status: 'error', risk, reason, error: message });
      return;
    }
    patch(id, { status: 'running', command, risk, reason, output: 'Running in terminal...' });
    await options.onLine(command);
    patch(id, { status: 'done', command, risk, reason, output: 'Sent to terminal.' });
    logCommand({ intent, command, status: 'sent_to_terminal', risk, reason });
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
      patch(id, { modelOutput: body, runtime: String(raw?.command_line ?? '') });
      let card: any = null;
      try { card = JSON.parse(body); } catch { card = null; }
      if (!card) {
        const message = body || 'No valid card returned.';
        patch(id, { status: 'error', error: message });
        logCommand({ intent: text, status: 'error', error: message });
        return;
      }

      const command = normalizeCommand(String(card['com' + 'mand'] ?? ''), text);
      const modelRisk = String(card.risk ?? 'medium').toLowerCase();
      const reason = String(card.reason ?? '');

      if (!command) {
        const message = String(card.fallback_message ?? (reason || 'No action available.'));
        patch(id, { status: 'blocked', output: message, reason });
        logCommand({ intent: text, status: 'blocked', reason, error: message });
        return;
      }

      const localDecision = await checkCommandRisk(command);
      const destructive = isDestructive(command);
      const readOnly = isReadOnlyInspection(command) && !localDecision.needs_confirmation;
      const localRisk = String(localDecision.risk ?? 'medium').toLowerCase();
      const risk = localRisk === 'high' || modelRisk === 'high'
        ? 'high'
        : (localDecision.needs_confirmation || destructive || (!readOnly && modelRisk !== 'low') ? 'medium' : 'low');
      const safetyReason = localDecision.needs_confirmation ? localDecision.reason : reason;

      if (localDecision.needs_confirmation || destructive || (risk !== 'low' && !readOnly)) {
        patch(id, { status: 'approval', command, risk, reason: safetyReason, needsApproval: true, output: 'Approval required. Expand Working to approve or cancel.' });
        logCommand({ intent: text, command, status: 'approval_required', risk, reason: safetyReason });
        return;
      }

      await sendToTerminal(id, command, risk, safetyReason, text);
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      setError(message);
      patch(id, { status: 'error', error: message });
      logCommand({ intent: text, status: 'error', error: message });
    } finally {
      setIsRunning(false);
    }
  }

  async function approveEntry(id: string) {
    const entry = entries.find((item) => item.id === id);
    if (!entry?.command) return;
    await sendToTerminal(id, entry.command, entry.risk || 'medium', entry.reason || 'Approved by user.', entry.intent);
  }

  function cancelEntry(id: string) {
    const entry = entries.find((item) => item.id === id);
    patch(id, { status: 'blocked', needsApproval: false, output: 'Cancelled.' });
    logCommand({ intent: entry?.intent, command: entry?.command, status: 'cancelled', risk: entry?.risk, reason: entry?.reason });
  }

  function reset() { setEntries([]); setResult(null); setError(''); setLastIntent(''); }

  return { entries, result, isRunning, error, lastIntent, runIntent, approveEntry, cancelEntry, reset };
}
