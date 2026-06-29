// @ts-nocheck
import { useEffect, useRef, useState } from 'react';
import { backendStatus, getAppState, listModelProfiles } from './lib/api';
import { DEFAULT_MODEL_PROFILES } from './lib/defaultProfiles';
import { useAiRun } from './hooks/useAiRun';
import { AppChrome } from './components/chrome/AppChrome';
import { TerminalCanvas } from './components/terminal/TerminalCanvas';
import { CommandComposer } from './components/terminal/CommandComposer';
import { SettingsDrawer } from './components/settings/SettingsDrawer';

const firstTab = { id: 'tab-1', title: 'PowerShell', cwd: '~' };
function makeTab(cwd: string, index: number) { return { id: `tab-${Date.now()}-${index}`, title: index === 1 ? 'PowerShell' : `PowerShell ${index}`, cwd }; }

export default function App() {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [backend, setBackend] = useState('starting');
  const [cwd, setCwd] = useState('~');
  const [tabs, setTabs] = useState([firstTab]);
  const [activeTabId, setActiveTabId] = useState(firstTab.id);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [profiles, setProfiles] = useState(DEFAULT_MODEL_PROFILES);
  const [selectedProfileId, setSelectedProfileId] = useState(String(DEFAULT_MODEL_PROFILES[0].id));
  const [input, setInput] = useState('');
  const ai = useAiRun(selectedProfileId);

  useEffect(() => {
    const preventMenu = (event) => event.preventDefault();
    document.addEventListener('contextmenu', preventMenu);
    backendStatus().then(setBackend).catch(() => setBackend('local preview'));
    getAppState().then((state) => { const nextCwd = String(state.cwd ?? '~'); setCwd(nextCwd); setTabs([{ ...firstTab, cwd: nextCwd }]); }).catch(() => undefined);
    listModelProfiles().then((items) => { const next = items.length ? items : DEFAULT_MODEL_PROFILES; setProfiles(next); setSelectedProfileId(String(next[0]?.id ?? DEFAULT_MODEL_PROFILES[0].id)); }).catch(() => setProfiles(DEFAULT_MODEL_PROFILES));
    window.setTimeout(() => inputRef.current?.focus(), 100);
    return () => document.removeEventListener('contextmenu', preventMenu);
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if ((event.ctrlKey || event.metaKey) && event.shiftKey && key === 't') {
        event.preventDefault();
        newTab();
      } else if ((event.ctrlKey || event.metaKey) && key === ',') {
        event.preventDefault();
        setSettingsOpen((open) => !open);
      } else if ((event.ctrlKey || event.metaKey) && (event.code === 'Space' || key === 'k')) {
        event.preventDefault();
        inputRef.current?.focus();
      } else if (event.key === 'Escape') {
        if (settingsOpen) setSettingsOpen(false);
        else if (input) setInput('');
      }
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
      <AppChrome backendStatus={backend} cwd={cwd} tabs={tabs} activeTabId={activeTabId} profiles={profiles} selectedProfileId={selectedProfileId} settingsOpen={settingsOpen} onSelectProfile={setSelectedProfileId} onNewTab={newTab} onSelectTab={setActiveTabId} onCloseTab={closeTab} onToggleSettings={() => setSettingsOpen((open) => !open)} />
      <section className="terminal-shell">
        <TerminalCanvas cwd={cwd} entries={ai.entries} />
        <CommandComposer ref={inputRef} cwd={cwd} value={input} disabled={ai.isRunning} onChange={setInput} onSubmit={submitPrompt} />
      </section>
      <SettingsDrawer open={settingsOpen} cwd={cwd} profiles={profiles} selectedProfileId={selectedProfileId} onSelectProfile={setSelectedProfileId} onClose={() => setSettingsOpen(false)} />
    </main>
  );
}
