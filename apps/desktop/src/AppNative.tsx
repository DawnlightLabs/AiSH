// @ts-nocheck
import { useState } from 'react';
import * as ShellView from './components/terminal/TerminalSurface';

export default function AppNative() {
  const [sessionId] = useState('main');
  const Surface = ShellView.TerminalSurface;
  return <main className="app-shell"><section className="terminal-shell"><Surface sessionId={sessionId} modelOutput={null} error="" /></section></main>;
}
