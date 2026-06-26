interface CommandComposerProps {
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onSubmit: () => void;
}

export function CommandComposer({ value, disabled, onChange, onSubmit }: CommandComposerProps) {
  return (
    <form className="composer" onSubmit={(event) => { event.preventDefault(); onSubmit(); }}>
      <span className="prompt">PS</span>
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
