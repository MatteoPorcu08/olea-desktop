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
test('build GitHub con release e updater solo per tag versionati',()=>{
  const workflow=file('.github/workflows/release.yml');
  assert.match(workflow,/tags: \['v\*'\]/);
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
