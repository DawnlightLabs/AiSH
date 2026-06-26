import type { ModelProfile } from '../../lib/api';

interface AppChromeProps {
  backendStatus: string;
  profiles: ModelProfile[];
  selectedProfileId: string;
  onSelectProfile: (id: string) => void;
}

export function AppChrome({ backendStatus, profiles, selectedProfileId, onSelectProfile }: AppChromeProps) {
  return (
    <header className="app-chrome">
      <div className="chrome-left">
        <div className="app-badge">Ai</div>
        <div className="app-title">
          <strong>AiSH</strong>
          <span>AI-native shell</span>
        </div>
        <div className="tab active-tab">PowerShell</div>
        <button className="tab add-tab" type="button">+</button>
      </div>

      <div className="chrome-right">
        <span className="chip">AI Run</span>
        <span className="chip muted">Context as needed</span>
        <select className="model-chip" value={selectedProfileId} onChange={(event) => onSelectProfile(event.target.value)}>
          {profiles.map((profile) => (
            <option key={String(profile.id)} value={String(profile.id)}>
              {String(profile.label ?? profile.id)}
            </option>
          ))}
        </select>
        <span className="chip muted">{backendStatus}</span>
      </div>
    </header>
  );
}
