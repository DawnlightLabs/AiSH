import { forwardRef, useEffect, useState } from 'react';

interface CommandComposerProps {
  value: string;
  cwd: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onSubmit: () => void;
}

const prompts = [
  'Run a command...',
  'Find a file...',
  'Inspect this project...',
  'Explain an error...',
  'Search this folder...',
];

export const CommandComposer = forwardRef<HTMLInputElement, CommandComposerProps>(function CommandComposer({ value, disabled, onChange, onSubmit }, ref) {
  const [index, setIndex] = useState(0);

  useEffect(() => {
    const timer = window.setInterval(() => setIndex((current) => (current + 1) % prompts.length), 2600);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <form className="composer" onSubmit={(event) => { event.preventDefault(); onSubmit(); }}>
      <span className="ken-mark" aria-hidden="true">AiSH</span>
      <span className="ken-label">Ask Ken</span>
      <input
        ref={ref}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        placeholder={prompts[index]}
        autoFocus
        spellCheck={false}
        autoCapitalize="off"
        autoComplete="off"
      />
      <span className="shortcut-hint">Ctrl+Space</span>
      <button type="submit" disabled={disabled}>{disabled ? 'Running' : 'Run'}</button>
    </form>
  );
});
