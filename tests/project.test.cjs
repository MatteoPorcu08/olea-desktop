const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root=path.join(__dirname,'..');
function file(s){return fs.readFileSync(path.join(root,s),'utf8');}
test('frontend fornisce le sezioni di gestione',()=>{
  const html=file('frontend/index.html');
  for(const name of ['dashboard','reports','sales','payments','customers','orchards','harvests','stock','expenses','archive']){
    assert.ok(html.includes(`data-nav="${name}"`), name);
  }
  assert.ok(html.includes('data-action="check-updates"'));
});
test('CRUD, salvataggio asincrono e importazione OLEA in frontend',()=>{
  const js=file('frontend/app.js');
  assert.match(js,/async function commit\(previous\)/);
  assert.match(js,/desktopInvoke\('save_archive'/);
  assert.match(js,/expectedRevision:desktopRevision/);
  assert.match(js,/desktopInvoke\('load_archive'/);
  assert.match(js,/async function importFile/);
  assert.match(js,/async function saveRecord/);
  assert.match(js,/async function removeRecord/);
});
test('SQLite: WAL + transazione + storico + backup consistente',()=>{
  const rust=file('src-tauri/src/lib.rs');
  for(const bit of ['PRAGMA journal_mode=WAL','PRAGMA synchronous=FULL','TransactionBehavior::Immediate','archive_history','VACUUM INTO','expected_revision','integrity_check'])assert.ok(rust.includes(bit),bit);
});
test('build GitHub e updater SHA-256 integrato',()=>{
  const workflow=file('.github/workflows/release.yml');
  assert.match(workflow,/workflow_dispatch:/);
  assert.match(workflow,/TAURI_SIGNING_PRIVATE_KEY:/);
  assert.match(workflow,/--features updates/);
  assert.match(file('scripts/configure_updates.py'),/latest\.json/);
});

test('modulo vendite con barra Salva sempre visibile',()=>{
  const css=file('frontend/style.css');
  const js=file('frontend/app.js');
  assert.match(css, /#entry-form\{display:flex;flex:1 1 auto;min-height:0;flex-direction:column;overflow:hidden;\}/);
  assert.match(css, /#entry-form>\.dialog-body\{flex:1 1 auto;min-height:0;overflow-y:auto;/);
  assert.match(css, /#entry-form>\.dialog-footer\{flex:0 0 auto;/);
  assert.ok(js.includes("'Salva vendita'"));
});

test('CI Windows pubblica release e nessun tool extra è richiesto sul PC',()=>{
  const ci=file('.github/workflows/ci.yml');
  assert.ok(ci.includes('gh release create'));
  assert.ok(ci.includes('npx tauri build'));
  const js=file('frontend/app.js');
  assert.ok(js.includes('setTimeout(()=>checkForUpdates(true),1800)'));
  const cargo=file('src-tauri/Cargo.toml');
  assert.ok(cargo.includes('sha2 = "0.10"'));
  assert.ok(cargo.includes('reqwest ='));
});

test('updater GitHub integrato e protetto da verifica hash SHA256',()=>{
  const rust=file('src-tauri/src/github_updates.rs');
  const js=file('frontend/app.js');
  assert.match(rust,/api.github.com\/repos\/MatteoPorcu08\/olea-desktop\/releases/);
  assert.match(rust,/Sha256::digest/);
  assert.match(rust,/pre-update-/);
  assert.match(rust,/expected_version/);
  assert.match(js,/expectedVersion:info.version/);
  assert.match(js,/checkForUpdates\(true\),1800/);
});
