import { useState } from 'react';
import { providerPlan, recordCommandLog, type ProviderContextMode, type ProviderPlan } from '../lib/api';

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

export function useAiRun(profileId: string, contextMode: ProviderContextMode = 'auto', options: { onLine?: (line: string) => Promise<void> } = {}) {
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
      patch(id, { status: 'error', command, risk, reason, needsApproval: false, error: message });
      logCommand({ intent, command, status: 'error', risk, reason, error: message });
      return;
    }
    patch(id, { status: 'running', command, risk, reason, needsApproval: false, output: 'Running in terminal...' });
    await options.onLine(command);
    patch(id, { status: 'done', command, risk, reason, needsApproval: false, output: 'Sent to terminal.' });
    logCommand({ intent, command, status: 'sent_to_terminal', risk, reason });
  }

  function patchPlanTrace(id: string, plan: ProviderPlan) {
    patch(id, {
      command: plan.command ?? undefined,
      risk: plan.risk,
      reason: plan.reason,
      modelOutput: plan.model_output ?? undefined,
      runtime: plan.runtime ?? undefined,
    });
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
      const plan = await providerPlan(profileId, text, 'ai_run', contextMode);
      setResult(plan);
      patchPlanTrace(id, plan);

      if (plan.action === 'noop') {
        patch(id, { status: 'blocked', output: plan.reason });
        return;
      }

      if (plan.action === 'error') {
        const message = plan.error || plan.reason || 'Provider planning failed.';
        patch(id, { status: 'error', error: message });
        logCommand({ intent: text, command: plan.command ?? undefined, status: 'error', risk: plan.risk, reason: plan.reason, error: message });
        return;
      }

      if (plan.action === 'fallback') {
        const message = plan.fallback_message || plan.reason || 'No action available.';
        patch(id, { status: 'blocked', output: message, reason: plan.reason });
        logCommand({ intent: text, status: 'blocked', risk: plan.risk, reason: plan.reason, error: message });
        return;
      }

      if (!plan.command) {
        const message = 'Provider returned no command.';
        patch(id, { status: 'error', error: message });
        logCommand({ intent: text, status: 'error', risk: plan.risk, reason: plan.reason, error: message });
        return;
      }

      if (plan.action === 'approval_required' || plan.needs_approval) {
        patch(id, { status: 'approval', command: plan.command, risk: plan.risk, reason: plan.reason, needsApproval: true, output: 'Approval required. Expand Working to approve or cancel.' });
        logCommand({ intent: text, command: plan.command, status: 'approval_required', risk: plan.risk, reason: plan.reason });
        return;
      }

      await sendToTerminal(id, plan.command, plan.risk, plan.reason, text);
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
