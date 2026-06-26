import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { openPty, sendPty } from '../../lib/api';
import type { ModelRunResult } from '../../lib/api';

const AISH_MARK = String.raw` █████╗ ██╗███████╗██╗  ██╗
██╔══██╗██║██╔════╝██║  ██║
███████║██║███████╗███████║
██╔══██║██║╚════██║██╔══██║
██║  ██║██║███████║██║  ██║
╚═╝  ╚═╝╚═╝╚══════╝╚═╝  ╚═╝`;

interface TerminalOutputEvent {
  session_id: string;
  data: string;
}

export function TerminalSurface({ sessionId, modelOutput, error }: { sessionId: string; modelOutput: ModelRunResult | null; error: string }) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);

  useEffect(() => {
    if (!hostRef.current) return;

    const terminal = new Terminal({
      cursorBlink: true,
      convertEol: true,
      fontFamily: 'Cascadia Mono, CaskaydiaCove Nerd Font, Consolas, monospace',
      fontSize: 13,
      theme: {
        background: '#03060a',
        foreground: '#d7dce8',
        cursor: '#d7dce8',
        selectionBackground: '#243047',
      },
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(hostRef.current);
    fit.fit();
    termRef.current = terminal;

    terminal.writeln(AISH_MARK);
    terminal.writeln('');

    const cols = terminal.cols || 120;
    const rows = terminal.rows || 30;
    openPty(sessionId, cols, rows).catch((err) => terminal.writeln(String(err)));

    const dataDisposable = terminal.onData((data) => {
      void sendPty(sessionId, data);
    });

    let unlisten: null | (() => void) = null;
    listen<TerminalOutputEvent>('terminal-output', (event) => {
      if (event.payload.session_id === sessionId) {
        terminal.write(event.payload.data);
      }
    }).then((fn) => { unlisten = fn; }).catch(() => undefined);

    const resize = () => fit.fit();
    window.addEventListener('resize', resize);

    return () => {
      window.removeEventListener('resize', resize);
      if (unlisten) unlisten();
      dataDisposable.dispose();
      terminal.dispose();
      termRef.current = null;
    };
  }, [sessionId]);

  useEffect(() => {
    if (!modelOutput || !termRef.current) return;
    const output = String(modelOutput.output ?? '').trim();
    const stderr = String(modelOutput.error ?? '').trim();
    termRef.current.writeln('');
    termRef.current.writeln('[AiSH model]');
    if (output) termRef.current.writeln(output);
    if (stderr) termRef.current.writeln(stderr);
  }, [modelOutput]);

  useEffect(() => {
    if (!error || !termRef.current) return;
    termRef.current.writeln('');
    termRef.current.writeln(`[AiSH error] ${error}`);
  }, [error]);

  return <div className="xterm-host" ref={hostRef} onContextMenu={(event) => event.preventDefault()} />;
}
