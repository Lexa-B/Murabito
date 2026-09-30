const fs = require("fs");
const html = fs.readFileSync(process.argv[2], "utf8");
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(m => m[1]);
for (const s of scripts) new Function(s);          // syntax
function node(tag) {
  const n = { tag, children: [], attrs: {}, listeners: [], _text: "", className: "", style: {} };
  n.appendChild = c => { n.children.push(c); return c; };
  n.append = (...cs) => cs.forEach(c => n.children.push(c));
  n.replaceChildren = () => { n.children.length = 0; };
  n.setAttribute = (k, v) => { n.attrs[k] = v; };
  n.addEventListener = (t, f) => n.listeners.push(f);
  Object.defineProperty(n, "textContent", { get: () => n._text, set: v => { n._text = String(v); } });
  return n;
}
const byId = {};
for (const m of html.matchAll(/id="([^"]+)"/g)) byId[m[1]] = node("byid");
const shared = [], structure = [], gradientC = [], seedC = [];
for (const m of html.matchAll(/class="controls controls-(shared|structure|gradient|seed)"/g)) (m[1] === "shared" ? shared : m[1] === "structure" ? structure : m[1] === "gradient" ? gradientC : seedC).push(node("div"));
global.document = {
  getElementById: id => byId[id] || node("missing:" + id),
  createElement: t => node(t), createElementNS: (ns, t) => node(t),
  querySelectorAll: sel => sel === ".controls-shared" ? shared : sel === ".controls-structure" ? structure : sel === ".controls-gradient" ? gradientC : sel === ".controls-seed" ? seedC : [],
};
// let the debounced render (150 ms), then the deferred windows, then the deferred drainage run
const wait = ms => new Promise(r => setTimeout(r, ms));
const flush = async () => { await wait(200); await wait(0); await wait(0); await wait(0); };
(async () => {
const t0 = Date.now();
new Function(scripts[0])();
const t1 = Date.now(); await flush();
console.log("render ms", t1 - t0, "with drainage", Date.now() - t0);
const dump = (id) => { const n = byId[id]; if (!n) return; console.log("--", id); for (const d of n.children) console.log(d.children.map(c => c._text).join(" | ")); };
dump("facts"); dump("facts2"); dump("facts3"); dump("facts4"); dump("facts5"); dump("facts5b"); dump("facts5c"); dump("facts5d"); dump("facts6"); dump("factsGradient"); dump("factsRain"); dump("factsSettle"); dump("factsCarve"); dump("facts8");
{ for (const id of ["lakesScene", "flowScene"]) { const n = byId[id]; console.log("--", id, "children", n.children.length, n.children.filter(c => c.tag === "text").map(c => c._text).slice(-1)[0]); } }
{ const n = byId.rainScene; console.log("-- rainScene children", n.children.length, n.children.filter(c => c.tag === "text").map(c => c._text).slice(-1)[0]); }
{ const n = byId.gradientsScene; const path = n.children.find(c => c.tag === "path"); console.log("-- gradientsScene children", n.children.length, "path segments", path ? (path.attrs.d.match(/M/g) || []).length : 0, "gradient sliders", gradientC.map(c => c.children.length).join(",")); }
const foot = (id) => { const n = byId[id]; const texts = n.children.filter(c => c.tag === "text").map(c => c._text); console.log("--", id, "texts:", texts.slice(-3).join(" || "), "| children", n.children.length); };
foot("noiseScene"); foot("heights4Scene"); foot("section4Scene"); foot("gulliesScene"); foot("maskScene"); foot("heights5Scene"); foot("windowBeforeScene"); foot("windowAfterScene"); foot("delta5bScene"); foot("heights5bScene"); foot("window5aScene"); foot("window5bScene"); foot("delta5cScene"); foot("heights5cScene"); foot("window5bAgainScene"); foot("window5cScene"); foot("delta5dScene"); foot("heights5dScene"); foot("window5cAgainScene"); foot("window5dScene"); foot("shareChartScene"); foot("coastMapScene"); foot("boxChartScene"); foot("coastClusterScene"); foot("drillDeciScene"); foot("drillKenScene"); foot("drillShakuScene");
console.log("-- octaves rows", byId.octaves.children.length);
for (const tr of byId.octaves.children.slice(0, 12)) console.log(tr.className, tr.children.map(c => c._text).join(" | "));
console.log("-- sliders: shared containers", shared.length, "labels each", shared.map(c => c.children.length).join(","), "structure containers", structure.length, "labels", structure.map(c => c.children.length).join(","));
// nudge a structure slider and re-render through its listener
const sl = structure[0].children[1].children[1];  // amp input
sl.value = "0"; sl.listeners[0](); await flush();
console.log("-- with A0 = 0:"); dump("facts4");
sl.value = "100"; sl.listeners[0](); await flush();
const seedIn = structure[0].children[0].children[1];
for (const s of ["17", "3", "42"]) { seedIn.value = s; seedIn.listeners[0](); await flush(); console.log("-- seed", s); dump("facts4"); }
seedIn.value = "5"; seedIn.listeners[0](); await flush(); console.log("-- seed 5"); dump("factsSettle");
if (process.env.STRIP) { seedIn.value = "1"; seedIn.listeners[0](); await flush(); byId.stripButton.onclick(); await wait(100); await wait(0); console.log("-- strip button:", byId.stripButton._text); dump("stripFacts"); }
})();
