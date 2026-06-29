import type { ModelProfile } from '../../lib/api';

interface AppChromeProps {
  backendStatus: string;
  cwd: string;
  tabs: Array<{ id: string; title: string; cwd: string }>;
  activeTabId: string;
  profiles: ModelProfile[];
  selectedProfileId: string;
  settingsOpen: boolean;
  appMode: 'ai' | 'normal';
  onSelectProfile: (id: string) => void;
  onNewTab: () => void;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  onToggleSettings: () => void;
  onToggleMode: () => void;
}

export function AppChrome({ backendStatus, tabs, activeTabId, profiles, selectedProfileId, settingsOpen, appMode, onSelectProfile, onNewTab, onSelectTab, onCloseTab, onToggleSettings, onToggleMode }: AppChromeProps) {
  return (
    <header className="app-chrome" data-tauri-drag-region>
      <div className="chrome-left" data-tauri-drag-region>
        <div className="app-badge" title="AiSH by Dawnlight Labs">Ai</div>
        <div className="brand-stack" data-tauri-drag-region>
          <strong>AiSH</strong>
          <small>Dawnlight Labs</small>
        </div>
        <div className="tab-row">
          {tabs.map((tab) => (
            <button key={tab.id} type="button" className={tab.id === activeTabId ? 'tab active-tab' : 'tab'} onClick={() => onSelectTab(tab.id)}>
              <span>{tab.title}</span>
              {tabs.length > 1 && <b className="tab-close" onClick={(event) => { event.stopPropagation(); onCloseTab(tab.id); }}>×</b>}
            </button>
          ))}
          <button className="new-tab-button" type="button" title="New tab" onClick={onNewTab}>+</button>
        </div>
      </div>

      <div className="chrome-right">
        <button className="mode-chip mode-button" type="button" onClick={onToggleMode}>{appMode === 'ai' ? 'AI Run' : 'Normal'}</button>
        <select className="model-chip" value={selectedProfileId} onChange={(event) => onSelectProfile(event.target.value)} disabled={appMode === 'normal'}>
          {profiles.map((profile) => (
            <option key={String(profile.id)} value={String(profile.id)}>{String(profile.label ?? profile.id)}</option>
          ))}
        </select>
        <button className={settingsOpen ? 'icon-button active-icon' : 'icon-button'} type="button" title="Settings" onClick={onToggleSettings}>⚙</button>
        <span className="status-dot" title={backendStatus}>{backendStatus === 'starting' ? 'starting' : 'ready'}</span>
      </div>
    </header>
  );
}
