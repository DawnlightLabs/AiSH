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
  ['01', 'Plain shell workflow', 'Type normally, ask when needed, and keep the terminal as the main interface.'],
  ['02', 'Provider shell builds', 'Install the native provider shell directly from GitHub release assets.'],
  ['03', 'Approval before change', 'Read-only commands can move quickly. Mutating commands require explicit approval.'],
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

function useFeatureRail(page) {
  useEffect(() => {
    const sections = Array.from(document.querySelectorAll('.features-cinema'));
    if (!sections.length || prefersReducedMotion()) return undefined;

    let frameId = 0;

    function update() {
      sections.forEach((section) => {
        const track = section.querySelector('.feature-rail-inner');
        const windowEl = section.querySelector('.feature-rail-window');
        if (!track || !windowEl) return;

        const rect = section.getBoundingClientRect();
        const travel = Math.max(1, rect.height - window.innerHeight);
        const progress = Math.min(1, Math.max(0, -rect.top / travel));
        const maxShift = Math.max(0, track.scrollWidth - windowEl.clientWidth);

        section.style.setProperty('--feature-progress', progress.toFixed(4));
        section.style.setProperty('--feature-shift', `${Math.round(-maxShift * progress)}px`);
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
          <h1>AiSH for the terminal.</h1>
          <p className="hero-copy">A native provider shell that turns plain-language intent into command-line actions, with approval before anything changes your system.</p>
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
          <h2>Built like a shell, not a dashboard.</h2>
          <p>Scroll through the operating model: stay native, install the provider, approve anything that changes state.</p>
        </div>
        <div className="feature-rail-window">
          <div className="feature-rail-inner">
            {FEATURES.map(([number, title, text], index) => (
              <article className="feature-card feature-slide" style={{ '--card-index': index }} key={title}>
                <span className="feature-number">{number}</span>
                <h3>{title}</h3>
                <p>{text}</p>
              </article>
            ))}
          </div>
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
