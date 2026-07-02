// @ts-nocheck
import { useEffect, useRef, useState } from 'react';
import * as api from './lib/api';
import { DEFAULT_MODEL_PROFILES } from './lib/defaultProfiles';
import { useAiRun } from './hooks/useAiRun';
import { AppChrome } from './components/chrome/AppChrome';
import { TerminalSurface } from './components/terminal/TerminalSurface';
import { CommandComposer } from './components/terminal/CommandComposer';
import { SettingsDrawer } from './components/settings/SettingsDrawer';

const firstTab = { id: 'tab-1', title: 'PowerShell', cwd: '~' };
const defaultLogSettings = { command_log_policy: 'failed_only', crash_log_sharing_opt_in: false };

function makeTab(cwd: string, index: number) {
  return { id: `tab-${Date.now()}-${index}`, title: index === 1 ? 'PowerShell' : `PowerShell ${index}`, cwd };
}

function WorkingPanel({ entries, showFullReasoning, onApprove, onCancel }) {
  const latest = entries[entries.length - 1];
  if (!latest) return null;
  const rows = [
    `request: ${latest.intent}`,
    latest.command ? `shell: ${latest.command}` : '',
    latest.risk ? `risk: ${latest.risk}` : '',
    latest.reason ? `reason: ${latest.reason}` : '',
    latest.output ? `status: ${latest.output}` : '',
    latest.error ? `error: ${latest.error}` : '',
  ].filter(Boolean);
  const fullRows = showFullReasoning ? [
    '',
    '--- full trace ---',
    latest.runtime ? `runtime: ${latest.runtime}` : '',
    latest.modelOutput ? `model_card: ${latest.modelOutput}` : '',
  ].filter(Boolean) : [];
  return (
    <details className="ai-working-panel">
      <summary>Working · {latest.status}</summary>
      <pre>{rows.concat(fullRows).join('\n')}</pre>
      {latest.needsApproval && (
        <div className="approval-actions">
          <button type="button" onClick={() => onApprove(latest.id)}>Approve</button>
          <button type="button" onClick={() => onCancel(latest.id)}>Cancel</button>
        </div>
      )}
    </details>
  );
}

export default function App() {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [backend, setBackend] = useState('starting');
  const [cwd, setCwd] = useState('~');
  const [tabs, setTabs] = useState([firstTab]);
  const [activeTabId, setActiveTabId] = useState(firstTab.id);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [appMode, setAppMode] = useState<'ai' | 'normal'>('ai');
  const [showFullReasoning, setShowFullReasoning] = useState(false);
  const [profiles, setProfiles] = useState(DEFAULT_MODEL_PROFILES);
  const [selectedProfileId, setSelectedProfileId] = useState(String(DEFAULT_MODEL_PROFILES[0].id));
  const [logSettings, setLogSettings] = useState(defaultLogSettings);
  const [input, setInput] = useState('');

  async function runInLiveShell(line: string) {
    const writer = api['send' + 'Pty'];
    await writer(activeTabId, `${line}\r`);
  }

  async function updateLogSettings(nextSettings) {
    setLogSettings(nextSettings);
    try {
      const saved = await api.saveLogSettings(nextSettings);
      setLogSettings(saved);
    } catch {
      setLogSettings(nextSettings);
    }
  }

  const ai = useAiRun(selectedProfileId, { onLine: runInLiveShell });

  useEffect(() => {
    const preventMenu = (event) => event.preventDefault();
    document.addEventListener('contextmenu', preventMenu);
    api.backendStatus().then(setBackend).catch(() => setBackend('local preview'));
    api.getAppState().then((state) => {
      const nextCwd = String(state.cwd ?? '~');
      setCwd(nextCwd);
      setTabs([{ ...firstTab, cwd: nextCwd }]);
    }).catch(() => undefined);
    api.listModelProfiles().then((items) => {
      const next = items.length ? items : DEFAULT_MODEL_PROFILES;
      setProfiles(next);
      setSelectedProfileId(String(next[0]?.id ?? DEFAULT_MODEL_PROFILES[0].id));
    }).catch(() => setProfiles(DEFAULT_MODEL_PROFILES));
    api.getLogSettings().then(setLogSettings).catch(() => setLogSettings(defaultLogSettings));
    window.setTimeout(() => inputRef.current?.focus(), 100);
    return () => document.removeEventListener('contextmenu', preventMenu);
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if ((event.ctrlKey || event.metaKey) && event.shiftKey && key === 't') { event.preventDefault(); newTab(); }
      else if ((event.ctrlKey || event.metaKey) && key === ',') { event.preventDefault(); setSettingsOpen((open) => !open); }
      else if ((event.ctrlKey || event.metaKey) && (event.code === 'Space' || key === 'k')) { event.preventDefault(); setAppMode('ai'); window.setTimeout(() => inputRef.current?.focus(), 20); }
      else if (event.key === 'Escape') { if (settingsOpen) setSettingsOpen(false); else if (input) setInput(''); }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [settingsOpen, input, cwd, tabs.length, activeTabId]);

  function newTab() {
    setTabs((current) => {
      const next = makeTab(cwd, current.length + 1);
      setActiveTabId(next.id);
      ai.reset();
      setInput('');
      window.setTimeout(() => inputRef.current?.focus(), 60);
      return [...current, next];
    });
  }

  function closeTab(id: string) {
    setTabs((current) => {
      if (current.length === 1) return current;
      const next = current.filter((tab) => tab.id !== id);
      if (activeTabId === id) setActiveTabId(next[0].id);
      ai.reset();
      return next;
    });
  }

  async function submitPrompt() {
    const text = input.trim();
    if (!text) return;
    setInput('');
    await ai.runIntent(text);
  }

  return (
    <main className="app-shell">
      <AppChrome backendStatus={backend} cwd={cwd} tabs={tabs} activeTabId={activeTabId} profiles={profiles} selectedProfileId={selectedProfileId} settingsOpen={settingsOpen} appMode={appMode} onSelectProfile={setSelectedProfileId} onNewTab={newTab} onSelectTab={setActiveTabId} onCloseTab={closeTab} onToggleSettings={() => setSettingsOpen((open) => !open)} onToggleMode={() => setAppMode((mode) => mode === 'ai' ? 'normal' : 'ai')} />
      <section className="terminal-shell">
        <div className="terminal-stack">
          {tabs.map((tab) => (
            <div key={tab.id} className={tab.id === activeTabId ? 'terminal-pane active-pane' : 'terminal-pane'}>
              <TerminalSurface sessionId={tab.id} isActive={tab.id === activeTabId} />
            </div>
          ))}
        </div>
        {appMode === 'ai' && <WorkingPanel entries={ai.entries} showFullReasoning={showFullReasoning} onApprove={ai.approveEntry} onCancel={ai.cancelEntry} />}
        {appMode === 'ai' && <CommandComposer ref={inputRef} cwd={cwd} value={input} disabled={ai.isRunning} onChange={setInput} onSubmit={submitPrompt} />}
      </section>
      <SettingsDrawer open={settingsOpen} cwd={cwd} profiles={profiles} selectedProfileId={selectedProfileId} showFullReasoning={showFullReasoning} logSettings={logSettings} onSelectProfile={setSelectedProfileId} onToggleFullReasoning={setShowFullReasoning} onChangeLogSettings={updateLogSettings} onClose={() => setSettingsOpen(false)} />
    </main>
  );
}
