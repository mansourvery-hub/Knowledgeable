#!/usr/bin/env node
/**
 * Knowledgeable UI definition-of-done checks (SPEC section 11).
 *
 * "Playwright or the existing CDP harness" — this is the CDP harness: it
 * drives headless Chrome over the DevTools protocol (no new dependencies;
 * uses the workspace `ws` package) and fails non-zero on any violation.
 *
 * Usage:
 *   # 1. Serve the fixture harness (temporary, deleted after proof):
 *   #      (cd apps/web/client && vite --port 5176)
 *   # 2. Run:
 *   #      node e2e/knowledgeable-ui.cjs --harness http://127.0.0.1:5176/beta-harness.html
 *   #      node e2e/knowledgeable-ui.cjs --harness <url> --shots-dir docs/ui-proof --shots-only
 *
 * Checks: forbidden text (11.6), min text size (11.3), arrowhead clearance
 * (11.4), focus visibility (11.5), pane independence (11.8), input contrast
 * (11.2). Palette literals (11.7) are covered by grep in CI; axe-core and
 * build/test gates (11.1, 11.10) run separately.
 */
const fs = require('fs');
const path = require('path');

function loadWs() {
  try {
    return require('ws');
  } catch {
    return require('../apps/web/node_modules/ws');
  }
}
const WebSocket = loadWs();

const args = process.argv.slice(2);
function flag(name, fallback) {
  const i = args.indexOf(name);
  return i === -1 ? fallback : (args[i + 1] ?? fallback);
}
const HARNESS = flag('--harness', 'http://127.0.0.1:5176/beta-harness.html');
const SHOTS_DIR = flag('--shots-dir', null);
const SHOTS_ONLY = args.includes('--shots-only');
const CHECKS_ONLY = args.includes('--checks-only');
const DEBUG_PORT = Number(flag('--debug-port', '9333'));

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Persistent session per scenario: blank target, then protocol navigation
// (the /json/new query form mangles our ?page=&theme= params).
async function openSession(url) {
  const res = await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/new?about:blank`, {
    method: 'PUT',
  });
  const target = await res.json();
  const ws = new WebSocket(target.webSocketDebuggerUrl, { maxPayload: 256 * 1024 * 1024 });
  await new Promise((r) => ws.on('open', r));
  let id = 0;
  const pending = new Map();
  ws.on('message', (data) => {
    const msg = JSON.parse(String(data));
    if (msg.id && pending.has(msg.id)) {
      pending.get(msg.id)(msg);
      pending.delete(msg.id);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      id += 1;
      pending.set(id, (msg) => (msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result)));
      ws.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) => {
    const { result } = await send('Runtime.evaluate', {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (result.subtype === 'error') {
      throw new Error(result.description);
    }
    return result.value;
  };
  await send('Page.enable');
  await send('Page.navigate', { url });
  return {
    send,
    evaluate,
    close: async () => {
      ws.close();
      await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/close/${target.id}`, { method: 'PUT' }).catch(() => {});
    },
  };
}

const CHECKS_JS = `
(() => {
  const out = { failures: [] };
  const panels = [...document.querySelectorAll('.k-panel, .k-reader')];
  const inDev = (el) => !!el.closest('.k-dev');
  const visible = (el) => {
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden';
  };

  // 11.6 forbidden text (visible text + placeholders + aria-labels, outside .k-dev).
  const FORBIDDEN = /\\b(UUID|Concept ID|Depth|Limit|inspection|neighborhood|wiki)\\b/i;
  const texts = [];
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const node = walker.currentNode;
    const el = node.parentElement;
    if (!el || ['SCRIPT', 'STYLE'].includes(el.tagName)) continue;
    if (!panels.some((p) => p.contains(el))) continue;
    if (inDev(el) || !visible(el)) continue;
    texts.push(node.textContent);
  }
  document.querySelectorAll('.k-panel input[placeholder]').forEach((el) => {
    if (!inDev(el) && visible(el)) texts.push(el.getAttribute('placeholder') ?? '');
  });
  document.querySelectorAll('.k-panel [aria-label], .k-reader [aria-label]').forEach((el) => {
    if (!inDev(el) && visible(el)) texts.push(el.getAttribute('aria-label') ?? '');
  });
  const joined = texts.join('\\n');
  const hit = joined.match(FORBIDDEN);
  if (hit) out.failures.push('forbidden-text: ' + JSON.stringify(hit[0]));

  // 11.3 minimum text size 12px.
  const small = [];
  const walker2 = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (walker2.nextNode()) {
    const node = walker2.currentNode;
    const el = node.parentElement;
    if (!el || ['SCRIPT', 'STYLE'].includes(el.tagName)) continue;
    if (!panels.some((p) => p.contains(el))) continue;
    if (inDev(el) || !visible(el)) continue;
    if (!node.textContent.trim()) continue;
    const size = parseFloat(getComputedStyle(el).fontSize);
    if (size < 12) small.push(el.tagName + '.' + el.className.toString().slice(0, 40) + ' ' + size + 'px');
  }
  if (small.length) out.failures.push('min-text-size: ' + JSON.stringify(small.slice(0, 5)));

  // 11.4 arrowhead clearance: dependency endpoints outside every node box.
  const nodes = [...document.querySelectorAll('.k-node')].map((el) => {
    const r = el.getBoundingClientRect();
    return { id: el.getAttribute('data-concept-id'), x: r.x, y: r.y, w: r.width, h: r.height };
  });
  const outside = (x, y, b) => x < b.x || x > b.x + b.w || y < b.y || y > b.y + b.h;
  for (const path of document.querySelectorAll('path.k-edge[marker-end]')) {
    const len = path.getTotalLength();
    const pt = path.getPointAtLength(len);
    // Endpoint in viewport coords: path lives in an untransformed svg, so
    // client point = svg rect origin + local point scaled by the viewBox.
    const svgRect = path.ownerSVGElement.getBoundingClientRect();
    const vb = path.ownerSVGElement.viewBox.baseVal;
    const sx = svgRect.width / vb.width, sy = svgRect.height / vb.height;
    const cx = svgRect.x + pt.x * sx, cy = svgRect.y + pt.y * sy;
    const inside = nodes.filter((b) => !outside(cx, cy, b)).map((b) => b.id);
    if (inside.length) out.failures.push('arrowhead-inside-node: ' + JSON.stringify(inside));
  }

  // 11.2 dark-theme inputs: non-white background with 4.5:1 text contrast.
  const lum = (rgb) => {
    const ch = (v) => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
    };
    const m = rgb.match(/\\d+(\\.\\d+)?/g).map(Number);
    const [r, g, b] = [ch(m[0]), ch(m[1]), ch(m[2])];
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  if (document.documentElement.classList.contains('dark')) {
    for (const input of document.querySelectorAll('.k-panel input')) {
      if (inDev(input) || !visible(input)) continue;
      const cs = getComputedStyle(input);
      const field = input.closest('.k-field');
      const bgEl = field ?? input;
      const bgc = getComputedStyle(bgEl).backgroundColor;
      if (bgc === 'rgb(255, 255, 255)') {
        out.failures.push('input-white-background');
        continue;
      }
      const L1 = lum(cs.color), L2 = lum(bgc);
      const ratio = (Math.max(L1, L2) + 0.05) / (Math.min(L1, L2) + 0.05);
      if (ratio < 4.5) out.failures.push('input-contrast: ' + ratio.toFixed(2));
    }
  }
  return out;
})()
`;

async function waitFor(session, expression, timeoutMs = 20000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      if (await session.evaluate(expression)) {
        return true;
      }
    } catch {
      // keep polling
    }
    await sleep(500);
  }
  return false;
}

async function focusWalk(session) {
  await session.evaluate('document.body.focus()');
  const stops = [];
  for (let i = 0; i < 30; i += 1) {
    await session.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
    await session.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
    await sleep(120);
    const stop = await session.evaluate(`(() => {
      const el = document.activeElement;
      if (!el || el === document.body) return null;
      if (!el.closest('.k-panel, .k-reader')) return { outside: true };
      const cs = getComputedStyle(el);
      // Search inputs suppress their own outline by design: the focus
      // indicator is the .k-field focus-within glow, so accept it instead.
      const field = el.tagName === 'INPUT' ? el.closest('.k-field') : null;
      return { tag: el.tagName, testid: el.getAttribute('data-testid'), label: el.getAttribute('aria-label'), outlineStyle: cs.outlineStyle, outlineWidth: cs.outlineWidth, fieldGlow: field ? getComputedStyle(field).boxShadow : 'none' };
    })()`);
    if (!stop || stop.outside) {
      continue;
    }
    stops.push(stop);
  }
  const bad = stops.filter(
    (s) =>
      s.outlineStyle === 'none' && parseFloat(s.outlineWidth) === 0 && s.fieldGlow === 'none',
  );
  return { stops: stops.length, bad };
}

const SCENARIOS = [
  { name: 'map-1280-light', page: 'map', theme: 'light', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="graph-canvas-node"]')`, checks: ['all', 'focus', 'arrows'] },
  { name: 'map-1280-dark', page: 'map', theme: 'dark', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="graph-canvas-node"]')`, checks: ['all', 'focus', 'arrows', 'inputs'] },
  { name: 'map-390-light', page: 'map', theme: 'light', width: 390, height: 844, ready: `!!document.querySelector('[data-testid="graph-canvas-node"]')`, checks: ['all', 'arrows'] },
  { name: 'map-390-dark', page: 'map', theme: 'dark', width: 390, height: 844, ready: `!!document.querySelector('[data-testid="graph-canvas-node"]')`, checks: ['all', 'arrows', 'inputs'] },
  { name: 'map-empty-1280-light', page: 'map-empty', theme: 'light', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="graph-empty"]')`, checks: ['all'] },
  { name: 'map-empty-1280-dark', page: 'map-empty', theme: 'dark', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="graph-empty"]')`, checks: ['all', 'inputs'] },
  { name: 'map-error-1280-light', page: 'map-error', theme: 'light', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="graph-error"]')`, checks: ['all'] },
  { name: 'map-error-1280-dark', page: 'map-error', theme: 'dark', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="graph-error"]')`, checks: ['all', 'inputs'] },
  { name: 'notebook-1280-light', page: 'notebook', theme: 'light', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="wiki-list"]')`, checks: ['all'] },
  { name: 'notebook-1280-dark', page: 'notebook', theme: 'dark', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="wiki-list"]')`, checks: ['all', 'inputs'] },
  { name: 'notebook-empty-1280-light', page: 'notebook-empty', theme: 'light', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="wiki-empty"]')`, checks: ['all'] },
  { name: 'reader-1280-light', page: 'reader', theme: 'light', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="wiki-title"]')`, checks: ['all', 'pane'] },
  { name: 'reader-1280-dark', page: 'reader', theme: 'dark', width: 1280, height: 800, ready: `!!document.querySelector('[data-testid="wiki-title"]')`, checks: ['all', 'pane'] },
  { name: 'reader-390-light', page: 'reader', theme: 'light', width: 390, height: 844, ready: `!!document.querySelector('[data-testid="wiki-title"]')`, checks: ['all', 'pane'] },
  { name: 'reader-390-dark', page: 'reader', theme: 'dark', width: 390, height: 844, ready: `!!document.querySelector('[data-testid="wiki-title"]')`, checks: ['all', 'pane'] },
];

(async () => {
  const failures = [];
  const shots = [];
  for (const s of SCENARIOS) {
    const url = `${HARNESS}?page=${s.page}${s.theme === 'dark' ? '&theme=dark' : ''}`;
    const session = await openSession(url);
    try {
      await session.send('Emulation.setDeviceMetricsOverride', {
        width: s.width,
        height: s.height,
        deviceScaleFactor: 1,
        mobile: false,
      });
      await session.send('Page.enable');
      const ready = await waitFor(session, s.ready);
      if (!ready) {
        failures.push(`${s.name}: page never settled`);
        continue;
      }
      await sleep(800);
      if (!CHECKS_ONLY && SHOTS_DIR) {
        const { data } = await session.send('Page.captureScreenshot', { format: 'png' });
        const file = path.join(SHOTS_DIR, `${s.name}.png`);
        fs.writeFileSync(file, Buffer.from(data, 'base64'));
        shots.push(file);
      }
      if (!SHOTS_ONLY && s.checks.includes('all')) {
        const result = await session.evaluate(CHECKS_JS);
        for (const f of result.failures) {
          failures.push(`${s.name}: ${f}`);
        }
      }
      if (!SHOTS_ONLY && s.checks.includes('focus')) {
        const walk = await focusWalk(session);
        if (walk.stops === 0) {
          failures.push(`${s.name}: no focusable panel controls found`);
        }
        for (const b of walk.bad) {
          failures.push(`${s.name}: focus-no-outline: ${b.tag} ${b.testid ?? b.label ?? ''}`);
        }
      }
      if (!SHOTS_ONLY && s.checks.includes('pane')) {
        const same = await session.evaluate(`(async () => {
          const chat = document.querySelector('[data-testid="harness-chat"]');
          if (!chat) return 'no-harness-chat';
          const before = JSON.stringify(chat.getBoundingClientRect().toJSON());
          const close = document.querySelector('[data-testid="wiki-close"]');
          if (!close) return 'no-close';
          close.click();
          await new Promise((r) => setTimeout(r, 400));
          const after = JSON.stringify(chat.getBoundingClientRect().toJSON());
          return before === after ? 'same' : ('moved ' + before + ' -> ' + after);
        })()`);
        if (same !== 'same') {
          failures.push(`${s.name}: pane-independence: ${same}`);
        }
      }
      console.log(`ok ${s.name}`);
    } catch (err) {
      failures.push(`${s.name}: harness error ${err.message}`);
    } finally {
      await session.close();
    }
  }
  if (shots.length) {
    console.log(`shots: ${shots.length} written`);
  }
  if (failures.length) {
    console.log('FAILURES:');
    for (const f of failures) {
      console.log(` - ${f}`);
    }
    process.exit(1);
  }
  console.log('all definition-of-done checks passed');
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
