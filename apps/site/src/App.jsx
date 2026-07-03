import React, { useEffect, useMemo, useState } from 'react';
import Lenis from 'lenis';

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
  {
    number: '01',
    title: 'Ask in plain language',
    text: 'Describe the intent. AiSH keeps you inside the shell and turns the request into a command plan.',
    tag: 'intent',
    prompt: 'find the largest log files',
    lines: ['intent received', 'reading current directory context', 'building a safe command plan'],
  },
  {
    number: '02',
    title: 'Review the exact command',
    text: 'The generated command is shown before it runs, so the workflow stays inspectable and native.',
    tag: 'plan',
    prompt: 'show the command first',
    lines: ['Get-ChildItem -Recurse *.log', 'Sort-Object Length -Descending', 'Select-Object -First 10'],
  },
  {
    number: '03',
    title: 'Approve system changes',
    text: 'Read-only actions can move fast. Anything that changes files or state waits for explicit approval.',
    tag: 'approve',
    prompt: 'clean temp files after review',
    lines: ['12 files matched', 'mutation detected', 'approval required before execution'],
  },
];

const DEMO_LINES = [
  ['PS>', 'aish "find large files in this repo"', 38],
  ['AiSH', 'Intent understood. Planning a read-only command.', 48],
  ['AiSH', 'Get-ChildItem -Recurse | Sort-Object Length -Descending | Select-Object -First 10', 86],
  ['OK', 'No approval needed for read-only inspection.', 44],
  ['AiSH', 'For cleanup or file changes, AiSH will ask first.', 51],
];

function prefersReducedMotion() {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

function getScrollDriver() {
  return window.__aishLenis || null;
}

function useLenisScroll() {
  useEffect(() => {
    if (prefersReducedMotion()) return undefined;

    const lenis = new Lenis({
      duration: 1.16,
      easing: (t) => Math.min(1, 1.001 - 2 ** (-10 * t)),
      orientation: 'vertical',
      gestureOrientation: 'vertical',
      smoothWheel: true,
      wheelMultiplier: 0.86,
      touchMultiplier: 1.15,
    });

    window.__aishLenis = lenis;
    let frameId = 0;

    function raf(time) {
      lenis.raf(time);
      frameId = requestAnimationFrame(raf);
    }

    frameId = requestAnimationFrame(raf);

    return () => {
      cancelAnimationFrame(frameId);
      lenis.destroy();
      if (window.__aishLenis === lenis) delete window.__aishLenis;
    };
  }, []);
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
        const lenis = getScrollDriver();
        const target = url.hash ? document.querySelector(url.hash) : 0;
        if (lenis) {
          lenis.scrollTo(target || 0, { offset: -18, duration: 1.05 });
        } else if (target) {
          target.scrollIntoView({ behavior: 'smooth', block: 'start' });
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

function useRevealOnScroll(page) {
  useEffect(() => {
    const elements = Array.from(document.querySelectorAll('.reveal'));
    if (!elements.length) return undefined;

    if (prefersReducedMotion() || !('IntersectionObserver' in window)) {
      elements.forEach((element) => element.classList.add('is-visible'));
      return undefined;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add('is-visible');
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.16, rootMargin: '0px 0px -8% 0px' },
    );

    elements.forEach((element, index) => {
      element.style.setProperty('--reveal-delay', `${Math.min(index * 70, 260)}ms`);
      observer.observe(element);
    });

    return () => observer.disconnect();
  }, [page]);
}

function clamp(value, min = 0, max = 1) {
  return Math.min(max, Math.max(min, value));
}

function useFeatureRail(page) {
  useEffect(() => {
    const sections = Array.from(document.querySelectorAll('.features-cinema'));
    if (!sections.length || prefersReducedMotion()) return undefined;

    let frameId = 0;

    function update() {
      sections.forEach((section) => {
        const rect = section.getBoundingClientRect();
        const scrollable = Math.max(1, rect.height - window.innerHeight);
        const progress = clamp(-rect.top / scrollable);
        const activeProgress = Math.min(0.999, progress);
        const active = Math.min(FEATURES.length - 1, Math.floor(activeProgress * FEATURES.length));

        section.style.setProperty('--feature-progress', progress.toFixed(4));
        section.style.setProperty('--feature-scene', active.toString());
        section.setAttribute('data-active', `${active}`);

        section.querySelectorAll('.feature-step-card').forEach((card, index) => {
          const local = clamp((progress * FEATURES.length) - index);
          card.style.setProperty('--step-progress', local.toFixed(4));
          card.classList.toggle('is-active', index === active);
          card.classList.toggle('is-before', index < active);
          card.classList.toggle('is-after', index > active);
        });

        section.querySelectorAll('.feature-terminal-step').forEach((step, index) => {
          const local = clamp((progress * FEATURES.length) - index);
          step.style.setProperty('--step-progress', local.toFixed(4));
          step.classList.toggle('is-active', index === active);
          step.classList.toggle('is-before', index < active);
          step.classList.toggle('is-after', index > active);
        });
      });

      frameId = requestAnimationFrame(update);
    }

    update();
    return () => cancelAnimationFrame(frameId);
  }, [page]);
}

function useDemoScroll(page) {
  useEffect(() => {
    const panels = Array.from(document.querySelectorAll('.demo-panel'));
    if (!panels.length || prefersReducedMotion()) {
      panels.forEach((panel) => panel.classList.add('is-playing'));
      return undefined;
    }

    let frameId = 0;

    function update() {
      panels.forEach((panel) => {
        const rect = panel.getBoundingClientRect();
        const start = window.innerHeight * 0.94;
        const end = window.innerHeight * 0.24;
        const visibleProgress = Math.min(1, Math.max(0, (start - rect.top) / Math.max(1, start - end)));
        const progress = window.scrollY > 8 ? visibleProgress : 0;

        panel.style.setProperty('--demo-progress', progress.toFixed(4));
        panel.classList.toggle('is-playing', progress > 0.08);
      });

      frameId = requestAnimationFrame(update);
    }

    update();
    return () => cancelAnimationFrame(frameId);
  }, [page]);
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
          <h1>Think it.<br />Run it.</h1>
          <p className="hero-copy">AiSH turns natural-language intent into precise shell workflows. Built for developers, operators, and security professionals who want a faster, more intelligent command line.</p>
          <div className="hero-actions">
            <a className="button button-primary" href="/#install">Install AiSH</a>
            <a className="button button-secondary" href={GITHUB_URL}>View source</a>
          </div>
        </div>
        <div className="logo-panel reveal" aria-label="AiSH logo">
          <span className="logo-orbit" aria-hidden="true" />
          <LogoMark className="hero-logo-lockup" variant="full" />
        </div>
      </div>
    </section>
  );
}

function DemoSection() {
  return (
    <section className="demo-section" aria-label="AiSH simulated terminal demo">
      <div className="container demo-panel reveal">
        <div className="demo-terminal" aria-label="Simulated AiSH terminal session">
          <div className="terminal-window-bar"><span /><span /><span /></div>
          <div className="demo-terminal-body">
            {DEMO_LINES.map(([prompt, text, chars], index) => (
              <p className="demo-line" style={{ '--line': index, '--chars': chars }} key={`${prompt}-${text}`}>
                <span className="demo-prompt">{prompt}</span>
                <span className="demo-type">{text}</span>
              </p>
            ))}
            <span className="demo-cursor" aria-hidden="true" />
          </div>
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

function FeatureCarousel() {
  return (
    <section className="features-cinema" id="features" aria-label="AiSH workflow features">
      <div className="container features-pin reveal">
        <div className="features-copy">
          <p className="section-kicker">Workflow</p>
          <h2>How AiSH works.</h2>
          <p>A pinned shell sequence takes over this section. Each scroll beat advances the command from intent to review to approval.</p>
        </div>
        <div className="feature-stage" aria-label="AiSH workflow animation">
          <div className="feature-terminal" aria-hidden="true">
            <div className="terminal-window-bar"><span /><span /><span /></div>
            <div className="feature-terminal-body">
              {FEATURES.map((feature) => (
                <div className="feature-terminal-step" key={feature.title}>
                  <p><span>$</span>aish "{feature.prompt}"</p>
                  {feature.lines.map((line, index) => (
                    <p key={line}><span>{index === feature.lines.length - 1 ? '✓' : '→'}</span>{line}</p>
                  ))}
                </div>
              ))}
            </div>
          </div>
          <div className="feature-deck">
            {FEATURES.map(({ number, title, text, tag }, index) => (
              <article className="feature-card feature-step-card" style={{ '--step': index }} key={title}>
                <div className="feature-card-top">
                  <span className="feature-number">{number}</span>
                  <span className="feature-tag">{tag}</span>
                </div>
                <h3>{title}</h3>
                <p>{text}</p>
              </article>
            ))}
          </div>
          <div className="feature-progress-track" aria-hidden="true"><span /></div>
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
      <DemoSection />
      <InstallTabs />
      <FeatureCarousel />
      <Cta />
    </main>
  );
}

function Footer() {
  const year = new Date().getFullYear();

  return (
    <footer className="site-footer">
      <div className="container footer-shell reveal">
        <div className="footer-word" aria-label="AiSH"><span>AiSH</span></div>
        <div className="footer-bottom">
          <span>© {year} <span className="footer-dawnlight">Dawnlight Labs</span></span>
          <span>AiSH — Artificially Intelligent Shell</span>
        </div>
      </div>
    </footer>
  );
}

export default function App() {
  const page = useClientRoute();
  useLenisScroll();
  useRevealOnScroll(page);
  useDemoScroll(page);
  useFeatureRail(page);

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
