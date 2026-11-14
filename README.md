# OLEA Desktop 3.0.2 — Aggiornamenti automatici firmati

OLEA usa **Rust + Tauri 2** per Windows 10/11 x64, una grafica HTML/CSS/JS e **SQLite** per salvare le registrazioni. Il database dei clienti e delle vendite resta sul computer.

## Stato e differenza tra installer

- Le release di **anteprima** (`preview-*`) sono installer NSIS Windows **senza auto-update**. Sono utili per testare la grafica e il database.
- Le release **stabili** (`v3.0.2`, `v3.0.3`, …) sono create dal workflow **firmato** e includono il controllo aggiornamenti Tauri.
- Un installer di anteprima, compresi 3.0.0 e 3.0.1, non può diventare auto-aggiornabile da solo: **installa manualmente la prima release stabile firmata**, poi le versioni successive verranno rilevate all'avvio e proposte all'utente.
- Ogni aggiornamento effettuato dall'app richiede una **firma verificata** e crea una copia SQLite prima di installare. La firma è obbligatoria per Tauri Updater.

## Attiva la firma sul tuo PC (una sola volta)

Non caricare chiavi private, password o database nel repository pubblico. **Non inviare chiavi in chat.**

1. Installa [Node.js LTS](https://nodejs.org) e [GitHub CLI](https://cli.github.com) su Windows.
2. Scarica o clona i sorgenti del repository, apri PowerShell nella cartella principale del progetto.
3. Avvia:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup-update-signing.ps1
```

Lo script, dopo l'accesso GitHub tramite browser, crea le chiavi **sul tuo PC** sotto `%USERPROFILE%\OLEA-Firma-Aggiornamenti`, carica la chiave privata e la password come **GitHub Actions Secrets** tramite `gh`, carica la chiave pubblica come variable `OLEA_UPDATER_PUBKEY` e avvia il workflow `release.yml`.

**Conserva una copia cifrata della chiave privata e della password in un posto sicuro:** non cambiarle per le future release senza una procedura specifica di migrazione.

Se preferisci farlo manualmente, genera le chiavi con `npx tauri signer generate -w <file-key>` e inserisci in [GitHub Actions Secrets e Variables](https://github.com/MatteoPorcu08/olea-desktop/settings/secrets/actions) i valori `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` e `OLEA_UPDATER_PUBKEY`; poi avvia [il workflow](https://github.com/MatteoPorcu08/olea-desktop/actions/workflows/release.yml) con **Run workflow**.

## Distribuzione aggiornamenti

- `.github/workflows/ci.yml` verifica test e build e pubblica solo una *preview* non firmata.
- `.github/workflows/release.yml` viene eseguito manualmente da GitHub Actions oppure su ogni modifica del file `src-tauri/tauri.conf.json` su `main`. Con le chiavi configurate, prima verifica i test, poi costruisce l'installer firmato e pubblica una release stabile con `latest.json`.
- **Per ogni nuova versione**, aggiornare insieme `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` e `package.json`, per esempio da 3.0.2 a 3.0.3; il workflow rifiuta versioni incoerenti e non sovrascrive release già pubblicate.
- L'app controlla una nuova versione all'avvio e tramite **Aggiornamenti**; dopo la conferma dell'utente scarica e installa l'update firmato.
- Endpoint di controllo: `https://github.com/MatteoPorcu08/olea-desktop/releases/latest/download/latest.json`. Le release di anteprima non sono usate per l'updater.

## Backup e compatibilità

Il database rimane in `%LOCALAPPDATA%\it.olea.gestionale\olea.sqlite3`, separato dall'installer. Le copie sono nella sottocartella `backups`, con backup giornalieri e pre-aggiornamento. Le versioni 3.0.0, 3.0.1 e 3.0.2 utilizzano lo stesso formato; il file precedente non è cancellato. Il progetto può importare il vecchio `archivio.json` della Desktop 2.1.

## Sviluppo locale

Su Windows, con Node.js LTS, Rust >=1.90, Build Tools C++ e WebView2:

```powershell
npm install
npm run test:js
cargo test --manifest-path src-tauri/Cargo.toml
npx tauri dev
npx tauri build
```

Il normale `npx tauri build` produce un installer **senza updater**, utile per sviluppo. Il workflow firmato applica la configurazione effimera di `scripts/configure_updates.py` e `--features updates`. Gli archivi SQLite **non** vengono sincronizzati su GitHub.
