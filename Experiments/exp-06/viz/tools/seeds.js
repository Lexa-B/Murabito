// Render the page for seeds 0…N−1 at the defaults and average the step-6 and step-5d readings.
const fs = require("fs");
const html = fs.readFileSync(process.argv[2], "utf8"), N = +(process.argv[3] || 30);
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(m => m[1]);
function node(tag) { const n = { tag, children: [], attrs: {}, listeners: [], _text: "", className: "", style: {} }; n.appendChild = c => { n.children.push(c); return c; }; n.append = (...cs) => cs.forEach(c => n.children.push(c)); n.replaceChildren = () => { n.children.length = 0; }; n.setAttribute = (k, v) => { n.attrs[k] = v; }; n.addEventListener = (t, f) => n.listeners.push(f); Object.defineProperty(n, "textContent", { get: () => n._text, set: v => { n._text = String(v); } }); return n; }
const byId = {}; for (const m of html.matchAll(/id="([^"]+)"/g)) byId[m[1]] = node("byid");
const shared = [], structure = [];
for (const m of html.matchAll(/class="controls controls-(shared|structure)"/g)) (m[1] === "shared" ? shared : structure).push(node("div"));
global.document = { getElementById: id => byId[id] || node("missing:" + id), createElement: t => node(t), createElementNS: (ns, t) => node(t), querySelectorAll: sel => sel === ".controls-shared" ? shared : sel === ".controls-structure" ? structure : [] };
const wait = ms => new Promise(r => setTimeout(r, ms));
const flush = async () => { await wait(200); await wait(0); await wait(0); await wait(0); };
const rows = id => Object.fromEntries(byId[id].children.map(d => [d.children[0]._text, d.children[1]._text]));
const num = (s, re) => { const m = s.match(re); return m ? +m[1] : NaN; };
(async () => {
  new Function(scripts[0])(); await flush();
  const seedIn = structure[0].children[0].children[1];
  const acc = {};
  const add = (k, v) => { if (!isNaN(v)) (acc[k] = acc[k] || []).push(v); };
  for (let seed = 0; seed < N; seed++) {
    seedIn.value = String(seed); seedIn.listeners[0](); await flush();
    const f6 = rows("facts6"), f5d = rows("facts5d"), f4 = rows("facts4"), fs_ = rows("factsSettle");
    const share = f6[Object.keys(f6).find(k => k.startsWith("land share"))];
    add("land % after step 2", num(share, /^(\d+)%/)); add("land % after step 4", num(share, /→ (\d+)% →/)); add("land % after 5d", num(share, /→ \d+% → (\d+)%/));
    add("erosion's cost, points", num(share, /costs ([\d.]+) points/));
    add("inset for 70%, ri", num(f6["inset for 70% land after erosion"], /d ≈ \+?(-?[\d.]+) ri/));
    add("coast ri", num(f6["coast at ri scale"], /^(\d+) coast ri/)); add("coast D at ri scale", num(f6["coast at ri scale"], /D ≈ ([\d.]+)/));
    const er = f5d[Object.keys(f5d).find(k => k.startsWith("erosion over"))]; add("mean |Δ| on land after 5d, m", num(er, /→ (\d+) m/));
    add("window relief after 5d, m", num(f5d[Object.keys(f5d).find(k => k.startsWith("window round"))], /→ (\d+) m over/));
    add("noise range low, m", num(f4["noise over the world's ri"], /^(-?\d+) to/)); add("noise range high, m", num(f4["noise over the world's ri"], /to (-?\d+) m/));
    add("lattice turned (1 = yes)", f4["heavy side"].includes("turned") ? 1 : 0);
    const rain = fs_[Object.keys(fs_).find(k => k.startsWith("a year's rain"))]; add("rain held in lakes, %", 100 * num(rain, /held in lakes (\d+)/) / num(rain, /^(\d+) M/));
    add("pits with fresh capacity", num(fs_["pits found"], /· (\d+) with fresh capacity/));
    const fc = rows("factsCarve"), hc = fc[Object.keys(fc).find(k => k.startsWith("a year's rain held"))] || ""; add("rain held after 7d, %", num(hc, /→ (\d+)%\)/)); add("lakes kept after 7d", num(fc[Object.keys(fc).find(k => k.startsWith("lakes, 7c"))] || "", /→ (\d+)/)); add("ri carved", num(fc[Object.keys(fc).find(k => k.startsWith("lakes deeper"))] || "", /· (\d+) ri carved/));
    process.stderr.write("seed " + seed + " done\n");
  }
  for (const [k, v] of Object.entries(acc)) { const mean = v.reduce((a, b) => a + b, 0) / v.length, sd = Math.sqrt(v.reduce((a, b) => a + (b - mean) ** 2, 0) / v.length); console.log(k.padEnd(32), "mean " + mean.toFixed(2).padStart(9), "sd " + sd.toFixed(2).padStart(8), "min " + Math.min(...v).toFixed(2).padStart(8), "max " + Math.max(...v).toFixed(2).padStart(8), "n " + v.length); }
})();
