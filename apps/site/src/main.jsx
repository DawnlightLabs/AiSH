import React from 'react';
import { createRoot } from 'react-dom/client';
import './styles.css';

const releaseBase = 'https://github.com/amaansyed27/aish/releases/latest';

function Mascot() {
  return (
    <div className="mascot" aria-hidden="true">
      <div className="prompt-mark">{'>'}_</div>
      <span className="eye left" />
      <span className="eye right" />
      <span className="smile" />
    </div>
  );
}

function TerminalPanel() {
  return (
    <div className="terminal-panel">
      <div className="terminal-top"><span /><span /><span /><b>aish</b></div>
      <pre>{`$ aish
AiSH provider shell
Copyright (c) 2026 Dawnlight Labs

ask: clean my repo safely
plan: inspect first, approve before mutations`}</pre>
    </div>
  );
}

function CodeBlock({ children }) {
  return <pre className="install-code"><code>{children}</code></pre>;
}

function Hero() {
  return (
    <section className="hero">
      <div className="hero-copy">
        <p className="eyebrow">Dawnlight Labs pilot project</p>
        <h1>An AI-native shell for real terminal workflows.</h1>
        <p className="lead">AiSH is now provider-shell first: local command planning, shell-aware syntax, approval gates, and setup flows without a heavyweight desktop wrapper.</p>
        <div className="hero-actions">
          <a className="primary" href="/downloads/">Install AiSH</a>
          <a className="secondary" href="#product">Explore product</a>
        </div>
        <div className="status-row"><span>Provider shell</span><span>Local-first model path</span><span>Risk-gated execution</span></div>
      </div>
      <div className="hero-visual"><Mascot /><TerminalPanel /></div>
    </section>
  );
}

function Card({ tag, title, children }) {
  return <article className="card"><span className="pill">{tag}</span><h2>{title}</h2><p>{children}</p></article>;
}

function Home() {
  return (
    <>
      <Hero />
      <section id="install" className="split section">
        <div>
          <p className="eyebrow">Install</p>
          <h2>One command, then AiSH handles setup.</h2>
          <p>Install scripts download the correct provider-shell binary, then launch AiSH setup for PATH, model path, shell profiles, Windows Terminal, and editor integrations.</p>
        </div>
        <div className="stack">
          <CodeBlock>{'irm https://aish.dawnlightlabs.com/install.ps1 | iex'}</CodeBlock>
          <CodeBlock>{'curl -fsSL https://aish.dawnlightlabs.com/install | bash'}</CodeBlock>
        </div>
      </section>
      <section id="product" className="grid section">
        <Card tag="01" title="AI Run mode">Type intent, get shell actions. AiSH keeps final terminal output clean and avoids raw model trace spam.</Card>
        <Card tag="02" title="Local-first Ken">The pilot build targets a local Qwen2.5 Coder GGUF profile with a compact command-card interface.</Card>
        <Card tag="03" title="Approval gates">Read-only commands can run quickly. Mutating, destructive, or system-impacting actions wait for explicit approval.</Card>
      </section>
      <section id="architecture" className="split section">
        <div><p className="eyebrow">Architecture</p><h2>Provider shell first.</h2><p>The desktop wrapper is archived. Mainline AiSH ships as a native provider shell that can run in Windows Terminal, PowerShell, VS Code-compatible terminals, Cursor, Windsurf, and standard macOS/Linux shells.</p></div>
        <div className="stack"><div>Windows PowerShell install script</div><div>macOS/Linux curl install script</div><div>GitHub Release binary archives</div><div>Static landing site on Vercel</div></div>
      </section>
    </>
  );
}

function Downloads() {
  return (
    <>
      <section className="page-hero section">
        <p className="eyebrow">Downloads</p>
        <h1>Install AiSH from your shell.</h1>
        <p className="lead">The recommended path is a one-command install. Backup archives are available from GitHub Releases.</p>
        <div className="hero-actions"><a className="primary" href={releaseBase}>Open latest release</a><a className="secondary" href="https://github.com/amaansyed27/aish/releases">All releases</a></div>
      </section>
      <section className="grid section">
        <article className="card"><span className="pill">WIN</span><h2>Windows</h2><p>PowerShell installer. Downloads aish.exe, adds PATH, and launches setup.</p><CodeBlock>{'irm https://aish.dawnlightlabs.com/install.ps1 | iex'}</CodeBlock></article>
        <article className="card"><span className="pill">MAC</span><h2>macOS</h2><p>Shell installer. Downloads the native provider shell and launches setup.</p><CodeBlock>{'curl -fsSL https://aish.dawnlightlabs.com/install | bash'}</CodeBlock></article>
        <article className="card"><span className="pill">LIN</span><h2>Linux</h2><p>Shell installer. Installs to your local bin path and launches setup.</p><CodeBlock>{'curl -fsSL https://aish.dawnlightlabs.com/install | bash'}</CodeBlock></article>
      </section>
    </>
  );
}

function Layout() {
  const page = window.location.pathname.startsWith('/downloads') ? 'downloads' : 'home';
  return (
    <div>
      <div className="bg-grid" aria-hidden="true" />
      <header className="nav"><a className="brand" href="/"><span className="brand-mark">Ai</span><span>AiSH</span></a><nav><a href="/#install">Install</a><a href="/#product">Product</a><a href="/downloads/">Downloads</a><a href="https://github.com/amaansyed27/aish">GitHub</a></nav></header>
      <main>{page === 'downloads' ? <Downloads /> : <Home />}</main>
      <footer><span>AiSH by Dawnlight Labs</span><span>2026 Dawnlight Labs</span></footer>
    </div>
  );
}

createRoot(document.getElementById('root')).render(<Layout />);
