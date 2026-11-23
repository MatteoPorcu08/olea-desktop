//! OLEA desktop: archivio transazionale SQLite, backup e aggiornamenti da GitHub.
//! La vista e i calcoli di OLEA Site 2.0 restano nel frontend.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
mod github_updates;

const COLLECTIONS: [&str; 7] = [
    "orchards", "harvests", "customers", "sales", "payments", "expenses", "adjustments",
];
const MAX_JSON_SIZE: usize = 64 * 1024 * 1024;
const MAX_HISTORY: i64 = 30;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveSnapshot {
    data: Value,
    revision: i64,
    location: String,
    migrated_from_v2: bool,
}

#[derive(Serialize)]
struct HistoryEntry {
    at: String,
    data: Value,
}

fn empty_archive() -> Value {
    json!({
        "app":"OLEA_SITE", "schemaVersion":2, "updated":null,
        "orchards":[], "harvests":[], "customers":[], "sales":[],
        "payments":[], "expenses":[], "adjustments":[]
    })
}

fn validate_archive(v: &Value) -> Result<(), String> {
    let obj = v.as_object().ok_or("Il documento non è un oggetto JSON")?;
    let bytes = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_JSON_SIZE {
        return Err("Archivio troppo grande (limite 64 MB)".into());
    }
    for collection in COLLECTIONS {
        let rows = obj
            .get(collection)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("Elenco '{collection}' mancante o non valido"))?;
        let mut ids = HashSet::new();
        for row in rows {
            let row = row.as_object().ok_or_else(|| format!("Record non valido in '{collection}'"))?;
            let id = row.get("id").and_then(Value::as_str).unwrap_or("");
            if id.is_empty() || !ids.insert(id) {
                return Err(format!("ID mancante o duplicato in '{collection}'"));
            }
            if collection == "orchards" {
                if row.get("sharecropping").is_some_and(|v| !v.is_boolean()) {
                    return Err("La spunta mezzadria deve essere vera o falsa".into());
                }
                if let Some(percentage) = row.get("ownerSharePercent") {
                    let pct = percentage.as_f64().ok_or("Percentuale proprietario non numerica")?;
                    if !pct.is_finite() || !(0.0..=100.0).contains(&pct) {
                        return Err("La percentuale proprietario deve essere tra 0 e 100".into());
                    }
                }
            }
            for key in ["liters", "olivesKg", "price", "amount", "millingCost", "hectares", "trees"] {
                if let Some(number) = row.get(key) {
                    let n = number.as_f64().ok_or_else(|| format!("{collection}.{key} non è numerico"))?;
                    if !n.is_finite() || n.abs() > 1.0e12 {
                        return Err(format!("Valore non plausibile per {collection}.{key}"));
                    }
                }
            }
        }
    }
    Ok(())
}

fn init_db(conn: &Connection, legacy: Option<&Path>) -> Result<(), String> {
    conn.busy_timeout(Duration::from_secs(8)).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=FULL;
         PRAGMA foreign_keys=ON;
         CREATE TABLE IF NOT EXISTS archive (
           id INTEGER PRIMARY KEY CHECK (id = 1),
           data TEXT NOT NULL,
           revision INTEGER NOT NULL DEFAULT 0,
           updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS archive_history (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           data TEXT NOT NULL,
           saved_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE INDEX IF NOT EXISTS idx_archive_history_saved_at ON archive_history(saved_at);",
    ).map_err(|e| e.to_string())?;

    let has_archive: Option<i64> = conn.query_row(
        "SELECT id FROM archive WHERE id = 1", [], |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    if has_archive.is_none() {
        let mut first = empty_archive();
        if let Some(file) = legacy {
            if file.is_file() {
                if let Ok(raw) = fs::read(file) {
                    if let Ok(candidate) = serde_json::from_slice::<Value>(&raw) {
                        if validate_archive(&candidate).is_ok() {
                            first = candidate;
                        }
                    }
                }
            }
        }
        conn.execute(
            "INSERT INTO archive(id, data, revision) VALUES(1, ?1, 0)",
            params![first.to_string()],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn db_folder(app: &AppHandle) -> Result<PathBuf, String> {
    let folder = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder)
}

fn legacy_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|p| p.join("OLEA").join("Desktop2").join("archivio.json"))
}

fn connect_db(path: &Path, legacy: Option<&Path>) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| format!("Database: {e}"))?;
    init_db(&conn, legacy)?;
    Ok(conn)
}

fn connect_app(app: &AppHandle) -> Result<(Connection, PathBuf), String> {
    let path = db_folder(app)?.join("olea.sqlite3");
    let old = legacy_path();
    let conn = connect_db(&path, old.as_deref())?;
    Ok((conn, path))
}

fn read_db(conn: &Connection) -> Result<(Value, i64), String> {
    let (raw, rev): (String, i64) = conn.query_row(
        "SELECT data, revision FROM archive WHERE id = 1", [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|e| e.to_string())?;
    let value = serde_json::from_str::<Value>(&raw).map_err(|e| format!("Archivio danneggiato: {e}"))?;
    validate_archive(&value)?;
    Ok((value, rev))
}

fn save_db(conn: &mut Connection, next: &Value, expected_revision: i64) -> Result<i64, String> {
    validate_archive(next)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let (old_raw, rev): (String, i64) = tx.query_row(
        "SELECT data, revision FROM archive WHERE id = 1", [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|e| e.to_string())?;
    if rev != expected_revision {
        return Err("Il database è stato modificato da un'altra finestra. Riavvia OLEA senza sovrascrivere le modifiche.".into());
    }
    tx.execute("INSERT INTO archive_history(data) VALUES(?1)", params![old_raw])
        .map_err(|e| e.to_string())?;
    let next_rev = rev + 1;
    tx.execute(
        "UPDATE archive SET data=?1, revision=?2, updated_at=datetime('now') WHERE id=1",
        params![next.to_string(), next_rev],
    ).map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM archive_history WHERE id NOT IN (SELECT id FROM archive_history ORDER BY id DESC LIMIT ?1)",
        params![MAX_HISTORY],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(next_rev)
}

fn history_db(conn: &Connection) -> Result<Vec<HistoryEntry>, String> {
    let mut stmt = conn.prepare(
        "SELECT saved_at, data FROM archive_history ORDER BY id DESC LIMIT ?1",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![MAX_HISTORY], |row| {
        let at: String = row.get(0)?;
        let data: String = row.get(1)?;
        Ok((at, data))
    }).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        let (at, raw) = row.map_err(|e| e.to_string())?;
        let data: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        out.push(HistoryEntry { at: format!("{at}Z"), data });
    }
    Ok(out)
}

fn backup_snapshot(conn: &Connection, folder: &Path, name: &str) -> Result<PathBuf, String> {
    let directory = folder.join("backups");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let output = directory.join(name);
    if output.exists() {
        return Ok(output);
    }
    // SQLite crea una COPIA CONSISTENTE del database, anche se è in modalità WAL.
    conn.execute("VACUUM INTO ?1", params![output.to_string_lossy().to_string()])
        .map_err(|e| e.to_string())?;
    Ok(output)
}

fn periodic_backup(conn: &Connection, folder: &Path) -> Result<(), String> {
    // Una copia giornaliera consistente; le copie degli ultimi 10 giorni sono mantenute.
    let day = conn.query_row("SELECT strftime('%Y-%m-%d','now','localtime')", [], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let name = format!("giornaliero-{day}.sqlite3");
    backup_snapshot(conn, folder, &name)?;
    let mut entries: Vec<_> = fs::read_dir(folder.join("backups"))
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|s| s.to_str()).unwrap_or("").starts_with("giornaliero-"))
        .collect();
    entries.sort();
    if entries.len() > 10 {
        let remove_count = entries.len() - 10;
        for f in entries.into_iter().take(remove_count) {
            fs::remove_file(f).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn load_archive(app: AppHandle) -> Result<ArchiveSnapshot, String> {
    let (conn, path) = connect_app(&app)?;
    let result: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0)).map_err(|e| e.to_string())?;
    if result != "ok" {
        return Err(format!("Integrità del database non verificata: {result}. È necessario un recupero dal backup."));
    }
    let (data, revision) = read_db(&conn)?;
    if let Some(parent) = path.parent() {
        // Errori di backup vengono segnalati in diagnostica, senza impedire di lavorare.
        if let Err(error) = periodic_backup(&conn, parent) {
            eprintln!("OLEA backup giornaliero: {error}");
        }
    }
    let old = legacy_path();
    let migrated_from_v2 = revision == 0 && old.as_ref().is_some_and(|p| p.exists())
        && COLLECTIONS.iter().any(|key| data.get(*key).and_then(Value::as_array).is_some_and(|a| !a.is_empty()));
    Ok(ArchiveSnapshot {
        data,
        revision,
        location: path.to_string_lossy().to_string(),
        migrated_from_v2,
    })
}

#[tauri::command]
fn save_archive(app: AppHandle, payload: Value, expected_revision: i64) -> Result<i64, String> {
    let (mut conn, _) = connect_app(&app)?;
    save_db(&mut conn, &payload, expected_revision)
}

#[tauri::command]
fn list_archive_history(app: AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let (conn, _) = connect_app(&app)?;
    history_db(&conn)
}

#[tauri::command]
fn backup_database(app: AppHandle) -> Result<String, String> {
    let (conn, path) = connect_app(&app)?;
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_secs();
    let folder = path.parent().ok_or("Cartella database non trovata")?;
    let file = backup_snapshot(&conn, folder, &format!("manuale-{seconds}.sqlite3"))?;
    Ok(file.to_string_lossy().to_string())
}

#[tauri::command]
fn database_location(app: AppHandle) -> Result<String, String> {
    Ok(db_folder(&app)?.join("olea.sqlite3").to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    builder
        .invoke_handler(tauri::generate_handler![
            load_archive,
            save_archive,
            list_archive_history,
            backup_database,
            database_location,
            github_updates::check_updates,
            github_updates::install_updates
        ])
        .run(tauri::generate_context!())
        .expect("Impossibile avviare OLEA");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_save_reopen_and_revision() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("olea.sqlite3");
        let mut conn = connect_db(&dbfile, None).unwrap();
        let (mut data, initial_rev) = read_db(&conn).unwrap();
        assert_eq!(initial_rev, 0);
        data["orchards"] = json!([{"id":"o1","name":"Oliveto A"}]);
        let revision = save_db(&mut conn, &data, initial_rev).unwrap();
        assert_eq!(revision, 1);
        assert!(save_db(&mut conn, &empty_archive(), 0).is_err(), "deve evitare aggiornamenti concorrenti");
        drop(conn);
        let conn = connect_db(&dbfile, None).unwrap();
        let (read, rev) = read_db(&conn).unwrap();
        assert_eq!(rev, 1);
        assert_eq!(read["orchards"][0]["name"], "Oliveto A");
        assert_eq!(history_db(&conn).unwrap().len(), 1);
        backup_snapshot(&conn, tmp.path(), "test.sqlite3").unwrap();
        let backup = connect_db(&tmp.path().join("backups/test.sqlite3"), None).unwrap();
        assert_eq!(read_db(&backup).unwrap().1, 1);
    }
    #[test]
    fn test_invalid_archive_cannot_overwrite_saved_data() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn, None).unwrap();
        let mut invalid = empty_archive();
        invalid["sales"] = json!([{"id":"a"},{"id":"a"}]);
        assert!(save_db(&mut conn, &invalid, 0).is_err());
        assert_eq!(read_db(&conn).unwrap().1, 0);
    }
    #[test]
    fn test_owner_share_validation() {
        let mut archive = empty_archive();
        archive["orchards"] = json!([{"id":"o1","name":"Oliveto","sharecropping":true,"ownerSharePercent":35.5}]);
        assert!(validate_archive(&archive).is_ok());
        archive["orchards"][0]["ownerSharePercent"] = json!(101);
        assert!(validate_archive(&archive).is_err());
        archive["orchards"][0]["ownerSharePercent"] = json!(-0.5);
        assert!(validate_archive(&archive).is_err());
        archive["orchards"][0]["ownerSharePercent"] = json!(50);
        archive["orchards"][0]["sharecropping"] = json!("yes");
        assert!(validate_archive(&archive).is_err());
        archive["orchards"] = json!([{"id":"old","name":"Oliveto precedente"}]);
        assert!(validate_archive(&archive).is_ok());
    }
    #[test]
    fn test_migrates_existing_v2_json_without_removing_it() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("archivio.json");
        let mut old_data = empty_archive();
        old_data["customers"] = json!([{"id":"c1","name":"Cliente"}]);
        fs::write(&old, old_data.to_string()).unwrap();
        let conn = connect_db(&tmp.path().join("new.sqlite3"), Some(&old)).unwrap();
        assert_eq!(read_db(&conn).unwrap().0["customers"][0]["name"], "Cliente");
        assert!(old.exists());
    }
}
