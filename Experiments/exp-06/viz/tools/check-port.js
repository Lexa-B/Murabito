// Ask the running exp-06 server for a render and print the facts rows the way check.js prints banana.html's,
// so the two outputs diff. Usage: node viz/tools/check-port.js [key=value ...]   e.g. theta=200 inset=-4 path=f32
// FLAGS=1 appends " [bad]" to flagged rows, as check.js does.
// The server: cargo run -p exp06 (EXP06=http://127.0.0.1:8060 to point elsewhere).
const base = process.env.EXP06 || "http://127.0.0.1:8060";
const settings = {};
for (const kv of process.argv.slice(2)) {
  const [k, v] = kv.split("=");
  settings[k] = k === "path" ? v : k === "windowQRS" ? v.split(",").map(Number) : +v;
}
(async () => {
  let res;
  try { res = await fetch(base + "/render", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(settings) }); }
  catch (e) { console.error("no server at", base, "-", e.message, "(cargo run -p exp06)"); process.exit(1); }
  if (!res.ok) { console.error("server", res.status, await res.text()); process.exit(1); }
  const out = await res.json();
  console.log("render ms", out.ms.toFixed(2), "path", out.path);
  console.log("-- facts");
  for (const f of out.facts) console.log(f.key + " | " + f.value + (process.env.FLAGS && f.bad ? " [bad]" : ""));
})();
