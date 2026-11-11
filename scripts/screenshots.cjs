// Regenerates the README screenshots in docs/screenshots from src/index.html.
// Run before each release:  npm run screenshots
// Needs Playwright with Chromium (npm i -g playwright && npx playwright install chromium).
const http = require('http'), fs = require('fs'), path = require('path');
const { chromium } = require('playwright');

const ROOT = path.join(__dirname, '..'), OUT = path.join(ROOT, 'docs/screenshots');
const MIN = 60e3, now = Date.now(), H = Math.floor(now / 3600e3) * 3600e3; // deadlines on the hour

// Demo boards. Times are relative to now so the chips always read the same.
const short = [
  { name: 'To do', cards: [{ t: 'Plan the week' }, { t: 'Water plants' }, { t: 'Pay rent', id: 'd1', due: H + 2 * 24 * 60 * MIN, dl: 1 }] },
  { name: 'Doing', track: true, cards: [{ t: 'Write README', id: 't1', due: now + 18 * MIN, ms: 42 * MIN, since: now }, { t: 'Fix the bug', ms: 75 * MIN, since: now }] },
  { name: 'Done', cards: [{ t: 'Ship v0.3', ms: 3 * 60 * MIN }, { t: 'Book flights', ms: 20 * MIN }] },
];
const tiny = [
  { name: 'To do', cards: [{ t: 'Plan the week' }, { t: 'Water plants' }, { t: 'Pay rent' }] },
  { name: 'Doing', track: true, cards: [{ t: 'Write README', ms: 12 * MIN, since: now }] },
  { name: 'Done', cards: [{ t: 'Ship v0.3' }, { t: 'Book flights' }] },
];
const long = [
  { name: 'To do', cards: [{ t: 'Plan the week' }, { t: 'Water plants' }, { t: 'Call the bank', id: 'd1', due: H - 3 * 60 * MIN, dl: 1, late: 1 }, { t: 'Pay rent', id: 'd2', due: H + 2 * 24 * 60 * MIN, dl: 1 }] },
  { name: 'Doing', track: true, cards: [{ t: 'Write README', id: 't1', due: now + 25 * MIN, ms: 12 * MIN, since: now }, { t: 'Fix the bug', ms: 50 * MIN, since: now }] },
  { name: 'Done', cards: [{ t: 'Ship v0.3', ms: 3 * 60 * MIN }, { t: 'Book flights', ms: 40 * MIN }, { t: 'Tidy desk', ms: 5 * MIN }] },
];

// Every card: 2 to 4 words.
for (const c of [...short, ...long, ...tiny].flatMap(c => c.cards)) { const n = c.t.split(/\s+/).length; if (n < 2 || n > 4) throw new Error('2-4 words per card: ' + c.t) }

const frameCss = `html,body{margin:0;background:#f4f4f5}
.f{display:inline-block;margin:14px;border-radius:12px;overflow:hidden;box-shadow:0 0 0 1px #0002,0 4px 14px #0003}
iframe{display:block;border:0}`;
const desk = (w, h) => `<!doctype html><meta charset=utf-8><style>${frameCss}
html,body{background:linear-gradient(135deg,#c7d2fe,#fbcfe8)}.f{margin:44px 56px}</style><div class="f"><iframe src="/index.html" width="${w}" height="${h}"></iframe></div>`;
const page = (w, h) => `<!doctype html><meta charset=utf-8><style>${frameCss}</style><div class="f"><iframe src="/index.html" width="${w}" height="${h}"></iframe></div>`;

const editor = `<!doctype html><meta charset=utf-8><style>${frameCss}
:root{color-scheme:dark}html,body{background:#f4f4f5}
.e{position:relative;display:flex;width:1000px;height:550px;background:#1e1f24;font:15px/1.65 ui-monospace,Menlo,Consolas,monospace;color:#d4d4d8}
.side{width:200px;background:#18191d;padding:14px 18px;font:15px/2.1 system-ui,sans-serif;color:#a1a1aa}.side b{color:#e4e4e7;font-weight:500}
pre{margin:0;padding:18px 24px;white-space:pre}.c{color:#6b7280;font-style:italic}.k{color:#c084fc}.f2{color:#60a5fa}.s{color:#a3e635}.n{color:#fb923c}
iframe{position:absolute;right:16px;bottom:16px}</style>
<div class="f"><div class="e"><div class="side"><b>tiny-kanban</b><br>▸ src<br>&nbsp; index.html<br>▸ src-tauri<br>&nbsp; main.rs<br>README.md</div>
<pre><span class="c">// Make the lock button the only clickable part of the window.</span>
<span class="k">fn</span> <span class="f2">passthrough</span>(window: WebviewWindow, keep: Option&lt;[f64; <span class="n">4</span>]&gt;) -&gt; bool {
    <span class="k">let</span> wayland = std::env::<span class="f2">var_os</span>(<span class="s">"WAYLAND_DISPLAY"</span>).<span class="f2">is_some</span>();
    <span class="k">if</span> keep.<span class="f2">is_none</span>() || wayland {
        <span class="k">let</span> _ = window.<span class="f2">set_ignore_cursor_events</span>(<span class="k">false</span>);
        <span class="k">return</span> keep.<span class="f2">is_none</span>();
    }
    std::thread::<span class="f2">spawn</span>(<span class="k">move</span> || <span class="k">loop</span> {
        <span class="c">// 4x a second when the mouse is far away, 30x when close</span>
        <span class="k">let</span> near = <span class="f2">cursor_over</span>(&amp;window);
        std::thread::<span class="f2">park_timeout</span>(<span class="f2">ms</span>(<span class="k">if</span> near { <span class="n">30</span> } <span class="k">else</span> { <span class="n">250</span> }));
    });
    <span class="k">true</span>
}

<span class="c">// TODO: stop refactoring and ship it</span>
<span class="k">fn</span> <span class="f2">main</span>() {
    tauri::Builder::<span class="f2">default</span>()
        .<span class="f2">run</span>(tauri::<span class="f2">generate_context!</span>())
        .<span class="f2">expect</span>(<span class="s">"failed to run tiny-kanban"</span>);
}</pre><iframe src="/index.html" width="480" height="220"></iframe></div></div>`;

const shots = [
  { file: 'board-dark.png', w: 560, h: 290, data: { theme: 'dark', cols: short } },
  { file: 'board-light.png', w: 760, h: 420, data: { theme: 'light', cols: long } },
  { file: 'mini-mode.png', w: 230, h: 140, pad: 44, desk: true, data: { theme: 'light', mini: true, cols: tiny }, hover: true },
  { file: 'settings.png', w: 560, h: 600, data: { theme: 'light', cols: long }, click: '#cg' },
  { file: 'overlay.png', html: editor, data: { theme: 'dark', mini: true, locked: true, opacity: 80, accent: '#4f6ef7', cols: long } },
];

const server = http.createServer((req, res) => {
  const shot = shots[+new URL(req.url, 'http://x').searchParams.get('s')];
  if (req.url.startsWith('/index.html')) return res.end(fs.readFileSync(path.join(ROOT, 'src/index.html')));
  res.setHeader('content-type', 'text/html');
  res.end(shot ? shot.html || (shot.desk ? desk : page)(shot.w, shot.h) : '');
});

(async () => {
  await new Promise(r => server.listen(0, r));
  const base = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch();
  for (const [i, s] of shots.entries()) {
    const ctx = await browser.newContext({ locale: 'en-GB', deviceScaleFactor: 2, viewport: { width: 1400, height: 1000 } });
    await ctx.addInitScript(d => { if (location.pathname === '/index.html') localStorage.setItem('kanban', d) }, JSON.stringify(s.data));
    const p = await ctx.newPage();
    p.on('pageerror', e => { console.error(s.file, e.message); process.exitCode = 1 });
    await p.goto(`${base}/?s=${i}`);
    const app = p.frameLocator('iframe');
    await app.locator('.card').first().waitFor();
    if (s.click) await app.locator(s.click).click();
    if (s.hover) await app.locator('.col .list > .card').nth(1).hover({ force: true });
    await p.mouse.move(1390, 990);
    if (s.hover) await app.locator('.col .list > .card').nth(1).hover({ force: true });
    await p.waitForTimeout(400);
    // keep the outer margin so the shadow isn't cut off
    const box = await p.locator('.f').boundingBox();
    await p.screenshot({ path: path.join(OUT, s.file), clip: { x: box.x - (s.pad || 14), y: box.y - (s.pad || 14), width: box.width + 2 * (s.pad || 14), height: box.height + 2 * (s.pad || 14) } });
    console.log('wrote', s.file);
    await ctx.close();
  }
  await browser.close(); server.close();
})();
