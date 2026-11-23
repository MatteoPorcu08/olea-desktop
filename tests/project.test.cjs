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

test('mes e pari: percentuali diverse, vecchi oliveti e giacenza vendibile per campagna',()=>{
  const vm=require('node:vm');
  const js=file('frontend/app.js');
  const names=['round','sum','dataSeason','ownerPct','ownerLiters','myLiters','metrics','stockYear'];
  const functions=names.map(name=>{
    const source=js.split('\n').find(line=>line.startsWith('function '+name+'('));
    assert.ok(source, 'Funzione assente: '+name);
    return source;
  }).join('\n');
  const data={
    orchards:[
      {id:'a',name:'In mezzadria',sharecropping:true,ownerSharePercent:50},
      {id:'b',name:'Vecchio oliveto'}
    ],
    harvests:[
      {id:'h1',orchardId:'a',season:2026,liters:200,olivesKg:1200},
      {id:'h2',orchardId:'b',season:2026,liters:40,olivesKg:300},
      {id:'h3',orchardId:'a',season:2025,liters:100,olivesKg:600}
    ],
    sales:[{id:'s1',season:2026,liters:70,price:10}],
    adjustments:[{id:'ad1',season:2026,liters:10}],
    expenses:[],payments:[]
  };
  const ui={season:2026};
  const context={db:data,ui,CURRENT_YEAR:2026,
    bySeason:arr=>arr.filter(x=>x.season===ui.season),
    expensesOfSeason:()=>[],
    saleTotal:s=>s.liters*s.price,
    paid:()=>0
  };
  const fn=vm.runInNewContext(functions+'\n({ownerPct,ownerLiters,myLiters,metrics,stockYear})',context);
  assert.equal(fn.ownerPct(data.orchards[1]),0,'oliveti vecchi: quota tua 100%');
  assert.equal(fn.ownerPct({sharecropping:true}),50,'default 50%');
  assert.equal(fn.ownerLiters(data.harvests[0]),100);
  assert.equal(fn.myLiters(data.harvests[0]),100);
  let m=fn.metrics();
  assert.equal(m.produced,240,'produzione lorda sempre totale');
  assert.equal(m.ownerProduced,100);
  assert.equal(m.myProduced,140);
  assert.equal(m.stock,80,'magazzino = quota tua + rettifiche - vendite');
  assert.equal(fn.stockYear(2026),80);
  assert.equal(fn.stockYear(2025),50,'campagne separate');
  data.orchards[0].ownerSharePercent=30;
  m=fn.metrics();
  assert.equal(m.ownerProduced,60);
  assert.equal(m.myProduced,180);
  assert.equal(m.stock,120);
  data.orchards[0].sharecropping=false;
  m=fn.metrics();
  assert.equal(m.ownerProduced,0);
  assert.equal(m.myProduced,240);
  assert.equal(m.stock,180);
  data.orchards[0].sharecropping=true;
  data.orchards[0].ownerSharePercent=100;
  assert.equal(fn.stockYear(2026),-20,'una modifica a 100% deve risultare non valida per la giacenza');
  const precise={liters:1.01,orchardId:'a'};
  data.orchards[0].ownerSharePercent=50;
  assert.equal(fn.ownerLiters(precise)+fn.myLiters(precise),1.01,'quote sommate senza perdita per arrotondamento');
});

test('mes e pari: modulo oliveto, retroattività, blocco giacenze negative e CSV',()=>{
  const js=file('frontend/app.js'),css=file('frontend/style.css'),rust=file('src-tauri/src/lib.rs');
  for(const required of ['id="form-sharecropping"','id="form-ownerSharePercent"','updateOrchardSharePreview','fd.has(\'sharecropping\')',
    'ownerPct(old)!==ownerPct(p)',"type==='orchard'&&(!old||ownerPct(old)!==ownerPct(p))",'m.ownerProduced','m.myProduced',
    "'Quota proprietario'", "'La tua quota'", "'Mes e pari'"]) {
    assert.ok(js.includes(required),'Manca '+required);
  }
  assert.ok(css.includes('.summary-strip.summary-strip-five'));
  assert.ok(css.includes('.field .checkbox-row input[type="checkbox"]'));
  assert.ok(rust.includes('ownerSharePercent'));
  assert.ok(rust.includes('test_owner_share_validation'));
});
