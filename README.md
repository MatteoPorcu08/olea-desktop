# OLEA Desktop 3.0.5 — Aggiornamenti integrati, Rust + Tauri + SQLite

## Mes’e pari (mezzadria) per oliveto

Nella scheda **Nuovo oliveto** o **Modifica oliveto** è disponibile la spunta *Mes’e pari / mezzadria*. Quando è attiva puoi indicare una **percentuale di prodotto spettante al proprietario** da 0 a 100 (50% iniziale). La tua quota è il complemento a 100.

Il campo **Olio ottenuto (litri)** nella raccolta indica sempre la produzione **totale**. Il registro delle raccolte, le schede degli oliveti e le esportazioni mostrano il totale e le due quote. **Solo la tua quota alimenta il magazzino**. Il fatturato continua a derivare esclusivamente dalle vendite realmente registrate, e i costi di frantoio restano quelli inseriti (non vengono divisi in automatico).

La percentuale è una proprietà dell’oliveto e si applica a tutte le sue raccolte, comprese quelle già registrate. Modificandola con uno storico presente, OLEA chiede conferma e blocca qualsiasi modifica che creerebbe giacenze negative in una campagna. Prima di correggere un accordo storico, è consigliabile esportare un backup JSON. Gli oliveti precedenti senza spunta restano al **100% di quota tua**.


Questa versione aggiorna OLEA direttamente dal programma: controlla automaticamente GitHub Releases all'avvio, propone il nuovo installer e, **solo dopo conferma**, scarica e controlla l'impronta SHA-256 pubblicata da GitHub, esegue un backup SQLite e avvia l'installer NSIS Windows in modalità silenziosa. **Non serve installare GitHub CLI, Node, Rust o altri programmi sul PC.**

**Importante:** per passare dalla 3.0.3 o precedenti alla 3.0.4 occorre **installare una sola volta** la 3.0.4 da GitHub Releases: il vecchio eseguibile non possiede questo sistema. Gli aggiornamenti successivi potranno essere installati direttamente da OLEA.

- Il database rimane in `%LOCALAPPDATA%\it.olea.gestionale\olea.sqlite3` (o il percorso mostrato in Archivio e backup) e non viene sovrascritto.
- Ogni aggiornamento crea un file `pre-update-*.sqlite3` nella sottocartella `backups`.
- L'installer si avvia solo dopo consenso dell'utente. L'app si chiude durante l'installazione; puoi riaprirla al termine.
- L'installer NSIS usa `/S`; in caso di installazione per tutti gli utenti potrebbe comunque richiedere i permessi di Windows.
- La verifica SHA-256 tutela l'integrità del download confrontandolo con i metadati delle API HTTPS di GitHub; **non è una firma crittografica indipendente del produttore**. Le release sono pubbliche e il repository deve restare sotto il tuo controllo. Gli installer Windows attuali non sono firmati con un certificato Authenticode.
- GitHub Actions pubblica le release `preview-X.Y.Z` a ogni nuova versione compilata su main; l'app seleziona la versione più recente e ignora downgrade, URL esterni e asset senza digest SHA-256 valido.

Questo installer non richiede aggiornamenti automatici del database né software aggiuntivi. L'app funziona offline per dati, vendite, oliveti, spese e report; serve una connessione solamente per controllare e scaricare una nuova versione.

[Sorgenti e release Windows](https://github.com/MatteoPorcu08/olea-desktop/releases)
