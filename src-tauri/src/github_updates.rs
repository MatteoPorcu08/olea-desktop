//! Aggiornamenti integrati dalle Release pubbliche di OLEA.
//! Nessuna applicazione aggiuntiva o chiave di firma richiesta sul PC.
//! Verifica SHA-256 con il digest pubblicato dalle API HTTPS di GitHub.
//! Nota: il digest GitHub non sostituisce una firma indipendente del produttore.
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf, time::{Duration, SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager};

const REPO: &str = "MatteoPorcu08/olea-desktop";
const RELEASES_API: &str = "https://api.github.com/repos/MatteoPorcu08/olea-desktop/releases?per_page=30";
const MAX_INSTALLER_BYTES: u64 = 160 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateInfo {
    pub version: String,
    pub tag: String,
    pub sha256: String,
    pub asset_url: String,
    pub size: u64,
}

fn choose_release(document: &Value, current_version: &str) -> Result<Option<UpdateInfo>, String> {
    let current = semver::Version::parse(current_version)
        .map_err(|e| format!("Versione corrente non valida: {e}"))?;
    let releases = document.as_array().ok_or("Risposta GitHub non valida")?;
    let mut best: Option<(semver::Version, UpdateInfo)> = None;
    for release in releases {
        if release.get("draft").and_then(Value::as_bool).unwrap_or(true) { continue; }
        let Some(tag) = release.get("tag_name").and_then(Value::as_str) else { continue };
        let number = tag.strip_prefix("preview-").or_else(||tag.strip_prefix('v'));
        let Some(version_raw) = number else { continue };
        let Ok(version) = semver::Version::parse(version_raw) else { continue };
        if version <= current || best.as_ref().is_some_and(|(v, _)| version <= *v) { continue; }
        let expected_name = format!("OLEA.Desktop_{}_x64-setup.exe", version);
        let Some(assets) = release.get("assets").and_then(Value::as_array) else { continue };
        let Some(asset) = assets.iter().find(|a|a.get("name").and_then(Value::as_str)==Some(expected_name.as_str())) else { continue };
        let Some(url) = asset.get("browser_download_url").and_then(Value::as_str) else { continue };
        let expected_url = format!("https://github.com/{REPO}/releases/download/{tag}/{expected_name}");
        if url != expected_url { continue; }
        let Some(digest) = asset.get("digest").and_then(Value::as_str).and_then(|d|d.strip_prefix("sha256:")) else { continue };
        if digest.len() != 64 || !digest.bytes().all(|c|c.is_ascii_hexdigit()) { continue; }
        let Some(size) = asset.get("size").and_then(Value::as_u64) else { continue };
        if size < 1024 || size > MAX_INSTALLER_BYTES { continue; }
        let info = UpdateInfo { version:version.to_string(), tag:tag.into(),sha256:digest.to_ascii_lowercase(),asset_url:url.into(),size };
        best = Some((version, info));
    }
    Ok(best.map(|(_,info)|info))
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(100))
        .user_agent(format!("OLEA-Desktop/{}", env!("CARGO_PKG_VERSION")))
        .build().map_err(|e|format!("Connessione GitHub: {e}"))
}

async fn latest_update() -> Result<Option<UpdateInfo>, String> {
    let client=http_client()?;
    let response=client.get(RELEASES_API).header("Accept", "application/vnd.github+json")
        .send().await.map_err(|e|format!("GitHub non raggiungibile: {e}"))?
        .error_for_status().map_err(|e|format!("GitHub: {e}"))?;
    let data:Value=response.json().await.map_err(|e|format!("Metadati release: {e}"))?;
    choose_release(&data,env!("CARGO_PKG_VERSION"))
}

#[tauri::command]
pub(crate) async fn check_updates() -> Result<Option<UpdateInfo>, String> {
    latest_update().await
}

#[tauri::command]
pub(crate) async fn install_updates(app:AppHandle,expected_version:String) -> Result<(),String> {
    #[cfg(not(target_os="windows"))]
    { let _ = (app,expected_version); return Err("Aggiornamento diretto disponibile solo su Windows".into()); }
    #[cfg(target_os="windows")]
    {
        let release=latest_update().await?.ok_or("Nessun aggiornamento disponibile")?;
        if release.version != expected_version {
            return Err("La versione pubblicata è cambiata: ripeti la verifica degli aggiornamenti".into());
        }
        let response=http_client()?.get(&release.asset_url).send().await
            .map_err(|e|format!("Download aggiornamento non riuscito: {e}"))?
            .error_for_status().map_err(|e|format!("Download aggiornamento: {e}"))?;
        if response.content_length().is_some_and(|n|n > MAX_INSTALLER_BYTES) {
            return Err("Installer troppo grande".into());
        }
        let bytes=response.bytes().await.map_err(|e|format!("File incompleto: {e}"))?;
        if bytes.len() as u64 != release.size || bytes.len() < 2 || &bytes[..2] != b"MZ" {
            return Err("Installer non valido oppure download incompleto".into());
        }
        let digest=format!("{:x}",Sha256::digest(&bytes));
        if digest != release.sha256 {
            return Err("Firma SHA-256 del file diversa da quella pubblicata da GitHub: installazione bloccata".into());
        }
        // La copia dell'archivio precede il lancio dell'installer.
        let folder=app.path().app_local_data_dir().map_err(|e|e.to_string())?;
        let database=folder.join("olea.sqlite3");
        if !database.is_file() { return Err("Database OLEA non trovato: aggiornamento interrotto".into()); }
        let (conn,_) = super::connect_app(&app)?;
        let parent=database.parent().ok_or("Cartella database mancante")?;
        let timestamp=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs();
        super::backup_snapshot(&conn,parent,&format!("pre-update-{timestamp}.sqlite3"))?;
        drop(conn);
        let tmp=std::env::temp_dir().join("OLEA-updates");
        fs::create_dir_all(&tmp).map_err(|e|e.to_string())?;
        let installer:PathBuf=tmp.join(format!("OLEA-{}-{timestamp}-setup.exe",release.version));
        let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&installer)
            .map_err(|e|format!("Non riesco a creare il file di aggiornamento: {e}"))?;
        file.write_all(&bytes).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
        drop(file);
        // NSIS usa /S per installare senza mostrare una nuova procedura guidata.
        // L'app viene chiusa per consentire all'installer di sostituire i file.
        std::process::Command::new(&installer).arg("/S").spawn()
            .map_err(|e|format!("Impossibile avviare l'installer: {e}"))?;
        app.exit(0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(version:&str,digest:&str,url:&str)->Value {
        let tag=format!("preview-{version}");
        serde_json::json!({"draft":false,"tag_name":tag,
          "assets":[{"name":format!("OLEA.Desktop_{version}_x64-setup.exe"),
          "size":3000000,"digest":format!("sha256:{digest}"),"browser_download_url":url}]})
    }
    #[test]
    fn versioning_and_no_downgrade(){
        let d="a".repeat(64);
        let url="https://github.com/MatteoPorcu08/olea-desktop/releases/download/preview-3.1.0/OLEA.Desktop_3.1.0_x64-setup.exe";
        let document=serde_json::json!([release("3.0.3",&d,"bad"),release("3.1.0",&d,url)]);
        assert_eq!(choose_release(&document,"3.0.4").unwrap().unwrap().version,"3.1.0");
        assert!(choose_release(&document,"3.1.0").unwrap().is_none());
    }
    #[test]
    fn reject_untrusted_url_or_missing_sha(){
        let d="b".repeat(64);
        let document=serde_json::json!([release("3.1.0",&d,"https://example.com/evil.exe")]);
        assert!(choose_release(&document,"3.0.4").unwrap().is_none());
        let url="https://github.com/MatteoPorcu08/olea-desktop/releases/download/preview-3.1.0/OLEA.Desktop_3.1.0_x64-setup.exe";
        let document=serde_json::json!([release("3.1.0","wrong",url)]);
        assert!(choose_release(&document,"3.0.4").unwrap().is_none());
    }
}
