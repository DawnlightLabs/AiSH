import fs from 'node:fs';
import path from 'node:path';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

const root = process.cwd();
const dist = path.join(root, 'dist');
const styles = fs.readFileSync(path.join(root, 'src', 'styles.css'), 'utf8');

const releaseBase = 'https://github.com/amaansyed27/aish/releases/latest';
const downloadBase = `${releaseBase}/download`;

function A(props) {
  return React.createElement('a', props, props.children);
}

function Mascot() {
  return React.createElement('div', { className: 'mascot', 'aria-hidden': 'true' },
    React.createElement('div', { className: 'prompt-mark' }, '>_'),
    React.createElement('span', { className: 'eye left' }),
    React.createElement('span', { className: 'eye right' }),
    React.createElement('span', { className: 'smile' })
  );
}

function TerminalPanel() {
  return React.createElement('div', { className: 'terminal-panel' },
    React.createElement('div', { className: 'terminal-top' },
      React.createElement('span'), React.createElement('span'), React.createElement('span'),
      React.createElement('b', null, 'aish')
    ),
    React.createElement('pre', null, '$ aish\nAiSH provider shell\nCopyright (c) 2026 Dawnlight Labs\n\nask: clean my repo safely\nplan: inspect first, approve before mutations')
  );
}

function Hero() {
  return React.createElement('section', { className: 'hero' },
    React.createElement('div', { className: 'hero-copy' },
      React.createElement('p', { className: 'eyebrow' }, 'Dawnlight Labs pilot project'),
      React.createElement('h1', null, 'An AI-native shell that still feels like a real terminal.'),
      React.createElement('p', { className: 'lead' }, 'AiSH brings local AI command generation, approval gates, provider-shell workflows, and a cinematic desktop terminal into one developer surface.'),
      React.createElement('div', { className: 'hero-actions' },
        A({ className: 'primary', href: '/downloads/' , children: 'Download AiSH' }),
        A({ className: 'secondary', href: '#product', children: 'Explore product' })
      ),
      React.createElement('div', { className: 'status-row' },
        React.createElement('span', null, 'Local-first model path'),
        React.createElement('span', null, 'Risk-gated execution'),
        React.createElement('span', null, 'Desktop + provider shell')
      )
    ),
    React.createElement('div', { className: 'hero-visual' },
      React.createElement(Mascot),
      React.createElement(TerminalPanel)
    )
  );
}

function Card({ tag, title, children }) {
  return React.createElement('article', { className: 'card' },
    React.createElement('span', { className: 'pill' }, tag),
    React.createElement('h2', null, title),
    React.createElement('p', null, children)
  );
}

function Home() {
  return React.createElement(React.Fragment, null,
    React.createElement(Hero),
    React.createElement('section', { id: 'product', className: 'grid section' },
      React.createElement(Card, { tag: '01', title: 'AI Run mode' }, 'Type intent, get shell actions. AiSH keeps the final terminal output clean and avoids dumping raw model traces into the shell.'),
      React.createElement(Card, { tag: '02', title: 'Local-first Ken' }, 'The pilot build targets a local Qwen2.5 Coder GGUF profile with a small command-card interface and explicit model status.'),
      React.createElement(Card, { tag: '03', title: 'Approval gates' }, 'Read-only commands can run quickly. Mutating, destructive, or system-impacting actions wait for explicit approval.')
    ),
    React.createElement('section', { id: 'architecture', className: 'split section' },
      React.createElement('div', null,
        React.createElement('p', { className: 'eyebrow' }, 'Architecture'),
        React.createElement('h2', null, 'A provider shell and desktop terminal built as a launch product.'),
        React.createElement('p', null, 'AiSH ships as a desktop terminal for controlled AI workflows and as a provider shell for terminal-native usage. Windows, macOS, and Linux bundles are produced through GitHub Actions.')
      ),
      React.createElement('div', { className: 'stack' },
        ['Windows MSI + provider shell', 'macOS DMG + provider shell', 'Linux DEB, RPM, AppImage', 'Static landing site on Vercel'].map((item) => React.createElement('div', { key: item }, item))
      )
    )
  );
}

const downloads = [
  ['WIN', 'Windows', 'MSI desktop installer plus provider shell executable.', `${downloadBase}/aish-windows.zip`],
  ['MAC', 'macOS', 'DMG desktop installer plus provider shell binary. Public launch builds should be signed and notarized.', `${downloadBase}/aish-macos.zip`],
  ['LIN', 'Linux', 'DEB, RPM, AppImage, and provider shell binary.', `${downloadBase}/aish-linux.zip`]
];

function Downloads() {
  return React.createElement(React.Fragment, null,
    React.createElement('section', { className: 'page-hero section' },
      React.createElement('p', { className: 'eyebrow' }, 'Downloads'),
      React.createElement('h1', null, 'Install AiSH on Windows, macOS, and Linux.'),
      React.createElement('p', { className: 'lead' }, 'Release builds are published from GitHub Releases. Pick your platform archive, then use the installer or provider shell binary inside it.'),
      React.createElement('div', { className: 'hero-actions' },
        A({ className: 'primary', href: releaseBase, children: 'Open latest release' }),
        A({ className: 'secondary', href: 'https://github.com/amaansyed27/aish/releases', children: 'All releases' })
      )
    ),
    React.createElement('section', { className: 'grid section' },
      downloads.map(([tag, title, text, href]) => React.createElement('article', { className: 'card', key: title },
        React.createElement('span', { className: 'pill' }, tag),
        React.createElement('h2', null, title),
        React.createElement('p', null, text),
        A({ className: 'secondary download-link', href, children: `Download ${title}` })
      ))
    )
  );
}

function Layout({ page }) {
  return React.createElement('div', null,
    React.createElement('div', { className: 'bg-grid', 'aria-hidden': 'true' }),
    React.createElement('header', { className: 'nav' },
      A({ className: 'brand', href: '/', children: [React.createElement('span', { className: 'brand-mark', key: 'm' }, 'Ai'), React.createElement('span', { key: 't' }, 'AiSH')] }),
      React.createElement('nav', null,
        A({ href: '/#product', children: 'Product' }),
        A({ href: '/#architecture', children: 'Architecture' }),
        A({ href: '/downloads/', children: 'Downloads' }),
        A({ href: 'https://github.com/amaansyed27/aish', children: 'GitHub' })
      )
    ),
    React.createElement('main', null, page === 'downloads' ? React.createElement(Downloads) : React.createElement(Home)),
    React.createElement('footer', null,
      React.createElement('span', null, 'AiSH by Dawnlight Labs'),
      React.createElement('span', null, '2026 Dawnlight Labs')
    )
  );
}

function html(page, title) {
  const body = renderToStaticMarkup(React.createElement(Layout, { page }));
  return `<!doctype html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><meta name="theme-color" content="#090807"><meta name="description" content="AiSH is an AI-native shell and desktop terminal from Dawnlight Labs."><title>${title}</title><link rel="stylesheet" href="/styles.css"></head><body>${body}</body></html>`;
}

fs.rmSync(dist, { recursive: true, force: true });
fs.mkdirSync(path.join(dist, 'downloads'), { recursive: true });
fs.writeFileSync(path.join(dist, 'styles.css'), styles);
fs.writeFileSync(path.join(dist, 'index.html'), html('home', 'AiSH by Dawnlight Labs'));
fs.writeFileSync(path.join(dist, 'downloads', 'index.html'), html('downloads', 'AiSH Downloads'));
console.log('Built AiSH React static site into dist');
