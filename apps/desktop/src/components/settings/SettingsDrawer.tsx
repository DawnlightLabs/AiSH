import type { ModelProfile } from '../../lib/api';

interface SettingsDrawerProps {
  open: boolean;
  cwd: string;
  profiles: ModelProfile[];
  selectedProfileId: string;
  showFullReasoning: boolean;
  onSelectProfile: (id: string) => void;
  onToggleFullReasoning: (value: boolean) => void;
  onClose: () => void;
}

export function SettingsDrawer({ open, cwd, profiles, selectedProfileId, showFullReasoning, onSelectProfile, onToggleFullReasoning, onClose }: SettingsDrawerProps) {
  if (!open) return null;

  return (
    <div className="settings-layer">
      <button className="settings-backdrop" type="button" aria-label="Close settings" onClick={onClose} />
      <aside className="settings-drawer" role="dialog" aria-label="AiSH settings">
        <div className="drawer-header">
          <div>
            <strong>Settings</strong>
            <span>AI Run mode</span>
          </div>
          <button type="button" onClick={onClose}>×</button>
        </div>
        <label className="settings-field">
          <span>Model</span>
          <select value={selectedProfileId} onChange={(event) => onSelectProfile(event.target.value)}>
            {profiles.map((profile) => (
              <option key={String(profile.id)} value={String(profile.id)}>{String(profile.label ?? profile.id)}</option>
            ))}
          </select>
        </label>
        <label className="settings-check">
          <input type="checkbox" checked={showFullReasoning} onChange={(event) => onToggleFullReasoning(event.target.checked)} />
          <span>Show full AI Run trace in Working</span>
        </label>
        <div className="settings-field">
          <span>Shell</span>
          <code>Live PowerShell session</code>
        </div>
        <div className="settings-field">
          <span>Keyboard</span>
          <code>Ctrl+Space focus Ken · Ctrl+, settings · Ctrl+Shift+T new tab · Esc close</code>
        </div>
        <div className="settings-note">
          Normal mode hides the Ken composer. AI Run mode sends approved low-risk commands into the live shell and keeps details in Working.
        </div>
      </aside>
    </div>
  );
}
