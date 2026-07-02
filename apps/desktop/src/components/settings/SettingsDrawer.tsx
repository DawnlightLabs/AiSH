import type { CommandLogPolicy, LogSettings, ModelProfile } from '../../lib/api';

interface SettingsDrawerProps {
  open: boolean;
  cwd: string;
  profiles: ModelProfile[];
  selectedProfileId: string;
  showFullReasoning: boolean;
  logSettings: LogSettings;
  onSelectProfile: (id: string) => void;
  onToggleFullReasoning: (value: boolean) => void;
  onChangeLogSettings: (settings: LogSettings) => void;
  onClose: () => void;
}

const logPolicyLabels: Record<CommandLogPolicy, string> = {
  off: 'Do not record commands',
  failed_only: 'Record failed commands only',
  all: 'Record all AiSH commands locally',
};

export function SettingsDrawer({ open, cwd, profiles, selectedProfileId, showFullReasoning, logSettings, onSelectProfile, onToggleFullReasoning, onChangeLogSettings, onClose }: SettingsDrawerProps) {
  if (!open) return null;

  function changeLogPolicy(command_log_policy: CommandLogPolicy) {
    onChangeLogSettings({ ...logSettings, command_log_policy });
  }

  function changeCrashSharing(crash_log_sharing_opt_in: boolean) {
    onChangeLogSettings({ ...logSettings, crash_log_sharing_opt_in });
  }

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
        <label className="settings-field">
          <span>Local command logs</span>
          <select value={logSettings.command_log_policy} onChange={(event) => changeLogPolicy(event.target.value as CommandLogPolicy)}>
            {(Object.keys(logPolicyLabels) as CommandLogPolicy[]).map((policy) => (
              <option key={policy} value={policy}>{logPolicyLabels[policy]}</option>
            ))}
          </select>
        </label>
        <label className="settings-check">
          <input type="checkbox" checked={logSettings.crash_log_sharing_opt_in} onChange={(event) => changeCrashSharing(event.target.checked)} />
          <span>Allow crash-log sharing prompts for Dawnlight Labs</span>
        </label>
        <div className="settings-note">
          Command logs are stored locally on this machine. AiSH does not upload logs in this build; Dawnlight Labs sharing is a saved preference for a later explicit opt-in flow.
        </div>
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
