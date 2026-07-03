import React, { useEffect, useMemo, useState } from 'react';
import Lenis from 'lenis';

const GITHUB_URL = 'https://github.com/amaansyed27/aish';
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
  ['01', 'Natural-language commands', 'Describe what you want to do and let AiSH translate intent into practical shell actions.'],
  ['02', 'Provider-shell first', 'Mainline AiSH is a native command-line provider, without the archived desktop wrapper.'],
  ['03', 'Risk-gated execution', 'Read-only commands can run quickly. Mutating or destructive commands wait for approval.'],
];

function useLenisScroll() {
  useEffect(() => {
    const lenis = new Lenis({
      duration: 1.12,
      easing: (t) => Math.min(1, 1.001 - Math.pow(2, -10 * t)),
      smoothWheel: true,
      wheelMultiplier: 0.82,
      touchMultiplier: 1.15,
    });

    window.__aishLenis = lenis;
    let frame = 0;
    function raf(time) {
      lenis.raf(time);
      frame = requestAnimationFrame(raf);
    }

    frame = requestAnimationFrame(raf);
    return () => {
      cancelAnimationFrame(frame);
      delete window.__aishLenis;
      lenis.destroy();
    };
  }, []);
}

function useRevealAnimations(page) {
  useEffect(() => {
    const nodes = Array.from(document.querySelectorAll('.reveal'));
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            entry.target.classList.add('is-visible');
            observer.unobserve(entry.target);
          }
        }
      },
      { threshold: 0.14, rootMargin: '0px 0px -10% 0px' },
    );

    nodes.forEach((node, index) => {
      node.style.setProperty('--reveal-delay', `${Math.min(index * 55, 330)}ms`);
      observer.observe(node);
    });
    return () => observer.disconnect();
  }, [page]);
}

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
          window.__aishLenis?.scrollTo(url.hash, { offset: -76 });
        } else {
          window.__aishLenis?.scrollTo(0, { immediate: false });
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
          <a className="nav-button" href="/#install">Get AiSH</a>
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
          <div className="eyebrow"><span className="eyebrow-dot" />AI-native shell for every platform</div>
          <h1>Think it.<br />Run it.</h1>
          <p className="hero-copy">AiSH turns natural-language intent into precise shell workflows. Built for developers, operators, and security professionals who want a faster, more intelligent command line.</p>
          <div className="hero-actions">
            <a className="button button-primary" href="/#install">Install AiSH</a>
            <a className="button button-secondary" href={GITHUB_URL}>View on GitHub</a>
          </div>
        </div>
        <div className="logo-card reveal" aria-label="AiSH logo showcase">
          <div className="logo-orbit" aria-hidden="true" />
          <LogoMark className="hero-logo-lockup" variant="full" />
        </div>
      </div>
    </section>
  );
}

function OsIcon({ os }) {
  if (os === 'windows') {
    return <svg className="os-icon" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M2 3.5 10.7 2v9.3H2V3.5Zm9.8-1.7L22 0v11.3H11.8V1.8ZM2 12.4h8.7V22L2 20.5v-8.1Zm9.8 0H22V24l-10.2-1.8v-9.8Z" /></svg>;
  }
  if (os === 'macos') {
    return <svg className="os-icon" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M17.1 12.7c0-2.9 2.4-4.3 2.5-4.4-1.4-2-3.5-2.3-4.2-2.3-1.8-.2-3.5 1.1-4.4 1.1-.9 0-2.3-1.1-3.8-1-1.9 0-3.7 1.1-4.7 2.8-2 3.5-.5 8.7 1.4 11.5.9 1.4 2 2.9 3.5 2.8 1.4-.1 1.9-.9 3.6-.9 1.7 0 2.1.9 3.6.9 1.5 0 2.4-1.4 3.4-2.8 1.1-1.6 1.5-3.1 1.6-3.2-.1 0-2.5-1-2.5-4.5ZM14.2 4.1c.8-1 1.4-2.4 1.2-3.8-1.2.1-2.7.8-3.6 1.8-.8.9-1.5 2.3-1.3 3.7 1.4.1 2.8-.7 3.7-1.7Z" /></svg>;
  }
  return <svg className="os-icon" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M12 2c-2.8 0-4.6 2.5-4.6 5.7 0 1.1.2 2.1.6 3-.8 1.2-1.4 2.8-1.4 4.4 0 2.1.9 3.8 2.2 4.8.7.6 1.7 1 3.2 1 1.5 0 2.5-.4 3.2-1 1.3-1 2.7-2.7 2.7-4.8 0-1.6-.6-3.2-1.4-4.4.4-.9.6-1.9.6-3C16.6 4.5 14.8 2 12 2Zm-1.8 5.1c-.5 0-.9-.5-.9-1.1s.4-1.1.9-1.1.9.5.9 1.1-.4 1.1-.9 1.1Zm3.6 0c-.5 0-.9-.5-.9-1.1s.4-1.1.9-1.1.9.5.9 1.1-.4 1.1-.9 1.1Z" /></svg>;
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
          <p className="section-kicker">Installation</p>
          <h2>One command. Ready to run.</h2>
          <p>Choose your operating system, copy the installer command, and launch AiSH from your terminal.</p>
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
                  <div className="os-label"><OsIcon os={os} />{item.label}</div>
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
      <div className="container">
        <div className="feature-grid">
          {FEATURES.map(([number, title, text]) => (
            <article className="feature-card reveal" key={title}>
              <div className="feature-number">{number}</div>
              <h3>{title}</h3>
              <p>{text}</p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

function Cta() {
  return (
    <section className="cta">
      <div className="container">
        <div className="cta-card reveal">
          <h2>Your shell should understand you.</h2>
          <p>Install AiSH and bring an intelligent command layer directly into your existing terminal workflow.</p>
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
          <div className="eyebrow"><span className="eyebrow-dot" />Downloads</div>
          <h1>Install AiSH from your shell.</h1>
          <p className="hero-copy">Use the one-command installer, or download release archives directly from GitHub.</p>
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
  return (
    <footer>
      <div className="container footer-inner">
        <span>© {new Date().getFullYear()} Dawnlight Labs</span>
        <span>AiSH — Artificially Intelligent Shell</span>
      </div>
    </footer>
  );
}

export default function App() {
  useLenisScroll();
  const page = useClientRoute();
  useRevealAnimations(page);

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
