interface CommandComposerProps {
  value: string;
  cwd: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onSubmit: () => void;
}

function shortPath(path: string) {
  if (!path) return '~';
  const normalized = path.replaceAll('\\', '/');
  const parts = normalized.split('/').filter(Boolean);
  return parts.slice(-2).join('/') || normalized;
}

export function CommandComposer({ value, cwd, disabled, onChange, onSubmit }: CommandComposerProps) {
  return (
    <form className="composer" onSubmit={(event) => { event.preventDefault(); onSubmit(); }}>
      <span className="prompt">PS {shortPath(cwd)}&gt;</span>
      <input
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        placeholder="Ask AiSH to run something..."
        autoFocus
      />
      <button type="submit" disabled={disabled}>{disabled ? 'Running' : 'Run'}</button>
    </form>
  );
}
