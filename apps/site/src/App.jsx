import React, { useEffect, useMemo, useState } from 'react';

const GITHUB_URL = 'https://github.com/DawnlightLabs/AiSH';
const RELEASE_URL = `${GITHUB_URL}/releases/latest`;
const WIN_INSTALL_COMMAND = ['irm', 'https://aish.dawnlightlabs.com/install.ps1', '|', ['i', 'e', 'x'].join('')].join(' ');
const UNIX_INSTALL_COMMAND = ['curl', '-fsSL', 'https://aish.dawnlightlabs.com/install', '|', 'bash'].join(' ');

const OS_COMMANDS = {
  windows: {
    label: 'Windows PowerShell',
    prompt: 'PS>',
    command: WIN_INSTALL_COMMAND,
    note: 'Open PowerShell and run the command. AiSH installs to your user profile and launches setup.',
  },
  macos: {
    label: 'macOS Terminal',
    prompt: '$',
    command: UNIX_INSTALL_COMMAND,
    note: 'Open Terminal and run the command. The installer downloads the native provider shell.',
  },
  linux: {
    label: 'Linux Shell',
    prompt: '$',
    command: UNIX_INSTALL_COMMAND,
    note: 'Run the command in Bash, Zsh, or another compatible shell.',
  },
};

const FEATURES = [
  ['Plain shell workflow', 'Type normally, ask when needed, and keep the terminal as the main interface.'],
  ['Provider shell builds', 'Install the native provider shell directly from GitHub release assets.'],
  ['Approval before change', 'Read-only commands can move quickly. Mutating commands require explicit approval.'],
];

function useClientRoute() {
  const getPage = () => (window.location.pathname.startsWith('/downloads') ? 'downloads' : 'home');
  const [page, setPage] = useState(getPage);

  useEffect(() => {
    const onPopState = () => setPage(getPage());
    const onClick = (event) => {
      const anchor = event.target.closest('a[href]');
      if (!anchor) return;

      const url = new URL(anchor.href, window.location.href);
      if (url.origin !== window.location.origin) return;
      if (url.pathname !== '/' && url.pathname !== '/downloads/') return;

      event.preventDefault();
      const next = url.pathname === '/downloads/' ? 'downloads' : 'home';
      if (window.location.pathname !== url.pathname || window.location.hash !== url.hash) {
        window.history.pushState({}, '', `${url.pathname}${url.hash}`);
      }
      setPage(next);

      requestAnimationFrame(() => {
        if (url.hash) {
          document.querySelector(url.hash)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
        } else {
          window.scrollTo({ top: 0, behavior: 'smooth' });
        }
      });
    };

    window.addEventListener('popstate', onPopState);
    document.addEventListener('click', onClick);
    return () => {
      window.removeEventListener('popstate', onPopState);
      document.removeEventListener('click', onClick);
    };
  }, []);

  return page;
}

function LogoMark({ className = 'brand-mark', variant = 'icon' }) {
  const src = variant === 'full'
    ? '/brand/aish-full-horizontal-graphite.svg'
    : '/brand/aish-icon-black.svg';
  const alt = variant === 'full' ? 'AiSH full lockup' : 'AiSH logo';
  return <img className={className} src={src} alt={alt} draggable="false" />;
}

function Nav() {
  return (
    <header className="nav">
      <div className="container nav-inner">
        <a className="brand" href="/#top" aria-label="AiSH home">
          <LogoMark />
          <span>AiSH</span>
        </a>
        <nav className="nav-links" aria-label="Primary navigation">
          <a href="/#install">Install</a>
          <a href="/#features">Features</a>
          <a href="/downloads/">Downloads</a>
          <a href={GITHUB_URL}>GitHub</a>
        </nav>
      </div>
    </header>
  );
}

function Hero() {
  return (
    <section className="hero" id="top">
      <div className="container hero-grid">
        <div className="hero-text reveal">
          <h1>AiSH for the terminal.</h1>
          <p className="hero-copy">A native provider shell that turns plain-language intent into command-line actions, with approval before anything changes your system.</p>
          <div className="hero-actions">
            <a className="button button-primary" href="/#install">Install AiSH</a>
            <a className="button button-secondary" href={GITHUB_URL}>View source</a>
          </div>
        </div>
        <div className="logo-panel reveal" aria-label="AiSH logo">
          <LogoMark className="hero-logo-lockup" variant="full" />
        </div>
      </div>
    </section>
  );
}

function detectPreferredOs() {
  const platform = navigator.platform.toLowerCase();
  if (platform.includes('win')) return 'windows';
  if (platform.includes('mac')) return 'macos';
  return 'linux';
}

function InstallTabs() {
  const [active, setActive] = useState('windows');
  const [copied, setCopied] = useState('');
  const osKeys = useMemo(() => Object.keys(OS_COMMANDS), []);

  useEffect(() => {
    setActive(detectPreferredOs());
  }, []);

  async function copyCommand(os) {
    const command = OS_COMMANDS[os].command;
    try {
      await navigator.clipboard.writeText(command);
      setCopied(os);
      window.setTimeout(() => setCopied(''), 1600);
    } catch {
      setCopied('');
    }
  }

  return (
    <section className="install" id="install">
      <div className="container">
        <div className="section-heading reveal">
          <h2>Install</h2>
          <p>Choose your operating system and run the installer from your shell.</p>
        </div>
        <div className="installer reveal">
          <div className="installer-tabs" role="tablist" aria-label="Operating system">
            {osKeys.map((os) => (
              <button key={os} className={`tab ${active === os ? 'active' : ''}`} onClick={() => setActive(os)} role="tab" aria-selected={active === os}>
                {os === 'macos' ? 'macOS' : os[0].toUpperCase() + os.slice(1)}
              </button>
            ))}
          </div>
          <div className="installer-content">
            {osKeys.map((os) => {
              const item = OS_COMMANDS[os];
              return (
                <div key={os} className={`os-panel ${active === os ? 'active' : ''}`} id={os}>
                  <div className="os-label">{item.label}</div>
                  <div className="command-box">
                    <span className="prompt">{item.prompt}</span>
                    <code>{item.command}</code>
                    <button className="copy-button" onClick={() => copyCommand(os)}>{copied === os ? 'Copied' : 'Copy'}</button>
                  </div>
                  <p className="install-note">{item.note}</p>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
}

function FeatureGrid() {
  return (
    <section className="features" id="features">
      <div className="container feature-grid">
        {FEATURES.map(([title, text]) => (
          <article className="feature-card reveal" key={title}>
            <h3>{title}</h3>
            <p>{text}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

function Cta() {
  return (
    <section className="cta">
      <div className="container">
        <div className="cta-card reveal">
          <h2>Use AiSH where you already work.</h2>
          <p>Install the provider shell, open a new terminal, and run AiSH from your normal command-line workflow.</p>
          <div className="hero-actions">
            <a className="button button-primary" href="/#install">Install now</a>
            <a className="button button-secondary" href={RELEASE_URL}>Manual downloads</a>
          </div>
        </div>
      </div>
    </section>
  );
}

function Downloads() {
  return (
    <main id="top">
      <section className="page-hero">
        <div className="container reveal">
          <h1>Downloads</h1>
          <p className="hero-copy">Use the installer command or download release archives directly from GitHub.</p>
          <div className="hero-actions"><a className="button button-primary" href="/#install">Recommended install</a><a className="button button-secondary" href={RELEASE_URL}>Latest release</a></div>
        </div>
      </section>
      <InstallTabs />
      <Cta />
    </main>
  );
}

function Home() {
  return (
    <main>
      <Hero />
      <InstallTabs />
      <FeatureGrid />
      <Cta />
    </main>
  );
}

function Footer() {
  const year = new Date().getFullYear();

  return (
    <footer className="site-footer">
      <div className="container footer-shell">
        <div className="footer-word" aria-label="AiSH"><span>AiSH</span></div>
        <div className="footer-bottom">
          <span>© {year} Dawnlight Labs</span>
          <span>Artificially Intelligent Shell</span>
        </div>
      </div>
    </footer>
  );
}

export default function App() {
  const page = useClientRoute();

  return (
    <>
      <Nav />
      <div className="page-shell" key={page}>
        {page === 'downloads' ? <Downloads /> : <Home />}
      </div>
      <Footer />
    </>
  );
}
