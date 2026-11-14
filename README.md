# OLEA Desktop 3.0 · Rust + Tauri 2 + SQLite

Applicazione desktop per **Windows 10/11 x64**, basata sull'interfaccia di OLEA Site 2.0 e riscritta con un backend Rust che archivia le registrazioni in SQLite.

## Cos'è pronto

- Gestione completa di **oliveti, raccolte, frantoio, magazzino, vendite, pagamenti, clienti, spese**.
- Dashboard con **fatturato**, **incassato**, **da incassare**, **spese** e **guadagno gestionale**; grafici, filtri e campagne annuali.
- Aggiunta, modifica ed eliminazione delle registrazioni con convalida dei valori.
- Salvataggi **transazionali**, serializzati e confermati prima di comunicare successo; storico delle ultime 30 revisioni del database.
- Database SQLite in modalità WAL con sincronizzazione FULL, verifica di integrità e controllo delle revisioni per non sovrascrivere dati modificati da un'altra finestra.
- Backup SQLite giornaliero con copie degli ultimi 10 giorni; backup manuali SQLite e JSON.
- **Migrazione automatica senza cancellazioni** dall'archivio della versione Desktop 2.1, se il nuovo database è vuoto.
- Base GitHub Actions per test, compilazione installer NSIS e **aggiornamenti Tauri firmati** da GitHub Releases.

## Perché Tauri e non un browser o solo Rust GUI?

La grafica già sviluppata per il Site può essere riutilizzata con maggiore libertà estetica. Il motore del programma è Rust e accede al disco senza server HTTP locale; Tauri crea l'installer Windows e integra un updater basato su firma. La finestra usa WebView2 (componente Microsoft di Windows), senza aprire manualmente il browser. SQLite è un archivio transazionale locale: **GitHub contiene il codice e gli installer, non i dati dei clienti**.

## Dove sono i dati?

OLEA usa la cartella app local data di Tauri, su Windows normalmente:

`%LOCALAPPDATA%\it.olea.gestionale\olea.sqlite3`

Nella sottocartella `backups` sono salvate copie del database. Il percorso esatto è mostrato nella schermata **Archivio e backup**. Aggiornando l'app, i dati non si spostano né si cancellano.

Se non esiste ancora un database per OLEA 3.0, l'app cerca:

`%LOCALAPPDATA%\OLEA\Desktop2\archivio.json`

Se è valido, lo copia nel nuovo archivio. **Il vecchio JSON non viene mai cancellato.** Per altri formati OLEA usa l'importazione JSON dalla schermata Archivio. Se ci sono dubbi, esporta sempre un backup da OLEA 2.1 prima del primo avvio.

## Eseguire o compilare su Windows

**Prerequisiti:** Windows 10/11; Rust >=1.90, Node.js LTS, Microsoft C++ Build Tools e WebView2. Segui https://v2.tauri.app/start/prerequisites/.

Da PowerShell, nella directory del progetto:

```powershell
npm install
npm run test:js
cargo test --manifest-path src-tauri/Cargo.toml
npx tauri dev
npx tauri build
```

L'installer NSIS **non aggiornabile** sarà in:

`src-tauri\target\release\bundle\nsis\`

Lo script CI compila l'installer direttamente sui runner Windows di GitHub Actions. In questo pacchetto non è incluso un EXE Windows già compilato: nell'ambiente utilizzato per preparare il progetto manca il toolchain Rust/Windows necessario. Non confondere l'anteprima HTML con l'app compilata.

## Collegare a GitHub (una volta)

1. Crea un repository **nuovo** su GitHub per OLEA, per esempio `MatteoPorcu08/olea-desktop` (consigliato pubblico per distribuire gli aggiornamenti senza token nell'app). **Non caricare mai gli archivi di clienti**.
2. Estrai questo ZIP, poi dalla cartella principale esegui:

```powershell
git init
git add .
git commit -m "OLEA 3.0 - Tauri Rust e SQLite"
git branch -M main
git remote add origin https://github.com/MatteoPorcu08/olea-desktop.git
git push -u origin main
```

3. Genera una coppia di chiavi di firma **sul tuo PC**, non nel repository:

```powershell
npx tauri signer generate -w "$env:USERPROFILE\olea-signing.key"
```

   Il file `olea-signing.key` è **privato e va custodito**. Il file pubblico `olea-signing.key.pub` può essere condiviso. Non perderli: una nuova coppia non può sostituire quella delle versioni già distribuite senza procedura di migrazione.
4. In **GitHub → repository → Settings → Secrets and variables → Actions**:
   - **Variables:** `OLEA_UPDATER_PUBKEY` = contenuto completo del file `.pub`.
   - **Secrets:** `TAURI_SIGNING_PRIVATE_KEY` = contenuto completo del file privato `.key`.
   - **Secrets:** `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = la password impostata durante la generazione (se presente).
5. Il workflow `.github/workflows/release.yml` genera automaticamente `latest.json`, l'installer NSIS e la firma degli aggiornamenti per ogni nuovo tag `vX.Y.Z` (per esempio `v3.0.0`). La versione di `src-tauri/tauri.conf.json` e `src-tauri/Cargo.toml` deve corrispondere al tag.

```powershell
git tag v3.0.0
git push origin v3.0.0
```

6. Attendi il completamento del workflow GitHub Actions, poi dalla pagina Releases scarica l'installer Windows firmato per l'updater. A ogni nuova release `v3.0.1`, `v3.1.0`, ecc., l'app proporrà l'aggiornamento.

L'**updater è intenzionalmente DISABILITATO nelle build locali non firmate**. Si abilita solo nella release con le chiavi e il repository reali: `.github/workflows/release.yml` crea una configurazione temporanea ignorata da Git. Non usare placeholder come chiavi pubbliche.

**GitHub non pubblica automaticamente una nuova versione per ogni commit o correzione:** il rilascio avviene dopo test e creazione di un tag. Questo evita update non verificati.

## Bilancio: significato degli indicatori

- **Fatturato gestionale**: quantità di olio vendute × prezzo al litro, indipendentemente dall'incasso.
- **Incassato**: denaro ricevuto e registrato nei pagamenti.
- **Da incassare**: parte non ancora pagata del valore venduto.
- **Spese**: costi inseriti nella sezione Spese, inclusi i costi di frantoio associati alla raccolta (senza contarli due volte).
- **Guadagno gestionale**: fatturato meno spese registrate; non rappresenta automaticamente l'utile fiscale.

## Sicurezza / limiti

- Questa app è **offline** per i dati. Il controllo aggiornamenti usa internet solo per richiedere la nuova versione da GitHub.
- SQLite evita perdita di dati per file JSON troncati e garantisce atomicità delle registrazioni; non sostituisce una copia su supporto esterno.
- Il file `.sqlite3` è un database locale **non cifrato**, protetto dagli accessi del sistema operativo. Se serve cifratura a riposo, va prevista un'ulteriore fase.
- I database già esistenti non vengono eliminati in caso di update. Le migrazioni future vanno collaudate con backup.
- I test Rust e il pacchetto Windows devono essere eseguiti dai runner di GitHub o su Windows prima dell'uso operativo; qui non è stato possibile compilare su Windows.
