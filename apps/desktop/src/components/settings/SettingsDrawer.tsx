import type { ModelProfile } from '../../lib/api';

interface SettingsDrawerProps {
  open: boolean;
  cwd: string;
  profiles: ModelProfile[];
  selectedProfileId: string;
  onSelectProfile: (id: string) => void;
  onClose: () => void;
}

export function SettingsDrawer({ open, cwd, profiles, selectedProfileId, onSelectProfile, onClose }: SettingsDrawerProps) {
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
        <div className="settings-field">
          <span>Directory</span>
          <code>{cwd || '~'}</code>
        </div>
        <div className="settings-field">
          <span>Keyboard</span>
          <code>Ctrl+Space focus prompt · Ctrl+, settings · Ctrl+Shift+T new tab · Esc close</code>
        </div>
        <div className="settings-note">
          Future modes stay hidden for now: Normal, cached history completion, and AI mode will later be cycled with Tab. Current build stays simple: one AI Run prompt with context gathered only when needed.
        </div>
      </aside>
    </div>
  );
}
