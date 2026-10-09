use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use tauri::Manager;
const REDIS_SETTINGS_KEY: &str = "mtoken:settings";

#[derive(Serialize)]
struct AppStatus {
    status: String,
    build: String,
    system: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct SettingItem {
    key: String,
    value: String,
}

#[derive(Deserialize, Serialize)]
struct CsvSettingRecord {
    name: String,
    value: String,
}

#[derive(Serialize)]
struct AddResult {
    added: bool,
    queued: bool,
    item: SettingItem,
    message: String,
}

#[derive(Serialize)]
struct SubmitResult {
    saved: bool,
    queued: bool,
    count: usize,
    message: String,
}

#[derive(Serialize)]
struct DeleteResult {
    deleted: bool,
    queued: bool,
    key: String,
    remaining_count: usize,
    message: String,
}

#[derive(Serialize)]
struct UpdateResult {
    updated: bool,
    queued: bool,
    item: SettingItem,
    message: String,
}

struct CommitJob {
    settings: Vec<SettingItem>,
}

struct SettingsStore {
    cache: RwLock<Vec<SettingItem>>,
    load_error: RwLock<Option<String>>,
    records_path: PathBuf,
    queue: mpsc::SyncSender<CommitJob>,
}

impl SettingsStore {
    fn new(records_path: PathBuf) -> Arc<Self> {
        let (queue, receiver) = mpsc::sync_channel::<CommitJob>(64);
        let (settings, load_error) = match read_records(&records_path) {
            Ok(settings) => (settings, None),
            Err(error) => {
                eprintln!("Could not load {}: {}", records_path.display(), error);
                (Vec::new(), Some(error))
            }
        };
        let store = Arc::new(Self {
            cache: RwLock::new(settings),
            load_error: RwLock::new(load_error),
            records_path,
            queue,
        });

        thread::Builder::new()
            .name("settings-commit-worker".to_string())
            .spawn(move || commit_worker(receiver))
            .expect("failed to start settings commit worker");

        store
    }

    fn to_items(&self) -> Vec<SettingItem> {
        self.cache.read().unwrap().clone()
    }

    fn save_records(&self, settings: &[SettingItem]) -> Result<(), String> {
        let parent = self.records_path.parent().ok_or("records.csv path has no parent")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let mut writer = csv::Writer::from_path(&self.records_path)
            .map_err(|error| format!("could not create records.csv: {}", error))?;
        for item in settings {
            writer
                .serialize(CsvSettingRecord {
                    name: item.key.clone(),
                    value: item.value.clone(),
                })
                .map_err(|error| format!("could not write records.csv: {}", error))?;
        }
        writer
            .flush()
            .map_err(|error| format!("could not flush records.csv: {}", error))
    }

    fn enqueue(&self) -> Result<(), String> {
        self.queue
            .try_send(CommitJob {
                settings: self.to_items(),
            })
            .map_err(|error| format!("settings commit queue is unavailable: {}", error))
    }
}

fn read_records(path: &PathBuf) -> Result<Vec<SettingItem>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut reader = csv::Reader::from_path(path)
        .map_err(|error| format!("could not open records.csv: {}", error))?;
    reader
        .deserialize()
        .collect::<Result<Vec<CsvSettingRecord>, csv::Error>>()
        .map(|records| {
            records
                .into_iter()
                .map(|record| SettingItem {
                    key: record.name,
                    value: record.value,
                })
                .collect()
        })
        .map_err(|error| format!("could not parse records.csv: {}", error))
}

fn commit_worker(receiver: mpsc::Receiver<CommitJob>) {
    for job in receiver {
        let mut committed = false;
        for attempt in 1..=3 {
            match commit_snapshot(&job.settings) {
                Ok(_) => {
                    committed = true;
                    break;
                }
                Err(error) => eprintln!("settings commit attempt {} failed: {}", attempt, error),
            }
        }

        if !committed {
            eprintln!("settings commit abandoned after 3 attempts; the cache remains available");
        }
    }
}

fn commit_snapshot(settings: &[SettingItem]) -> Result<(), String> {
    let payload = serde_json::to_string(settings)
        .map_err(|error| format!("could not encode settings: {}", error))?;
    let redis_url = env::var("REDIS_URL").map_err(|_| "REDIS_URL is not configured".to_string())?;
    let redis_client = redis::Client::open(redis_url).map_err(|error| error.to_string())?;
    let mut redis_connection = redis_client
        .get_connection()
        .map_err(|error| format!("Redis connection failed: {}", error))?;
    redis::cmd("SET")
        .arg(REDIS_SETTINGS_KEY)
        .arg(&payload)
        .query::<()>(&mut redis_connection)
        .map_err(|error| format!("Redis write failed: {}", error))?;

    let database_url =
        env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not configured".to_string())?;
    let mut database = postgres::Client::connect(&database_url, postgres::NoTls)
        .map_err(|error| format!("PostgreSQL connection failed: {}", error))?;
    let mut transaction = database
        .transaction()
        .map_err(|error| format!("PostgreSQL transaction failed: {}", error))?;
    transaction
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS settings (
                setting_key TEXT PRIMARY KEY,
                setting_value TEXT NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .map_err(|error| format!("PostgreSQL schema setup failed: {}", error))?;
    transaction
        .execute("DELETE FROM settings", &[])
        .map_err(|error| format!("PostgreSQL snapshot delete failed: {}", error))?;

    for item in settings {
        transaction
            .execute(
                "INSERT INTO settings (setting_key, setting_value) VALUES ($1, $2)",
                &[&item.key, &item.value],
            )
            .map_err(|error| format!("PostgreSQL snapshot insert failed: {}", error))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("PostgreSQL commit failed: {}", error))?;
    Ok(())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn read_settings(state: tauri::State<'_, Arc<SettingsStore>>) -> Result<Vec<SettingItem>, String> {
    if let Some(error) = state.load_error.read().unwrap().as_ref() {
        return Err(error.clone());
    }
    Ok(state.to_items())
}

#[tauri::command]
fn get_app_status() -> AppStatus {
    let operating_system = match env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    };
    AppStatus {
        status: "Ready".to_string(),
        build: env!("CARGO_PKG_VERSION").to_string(),
        system: format!("{} ({})", operating_system, env::consts::ARCH),
    }
}

#[tauri::command]
fn add_setting(item: SettingItem, state: tauri::State<'_, Arc<SettingsStore>>) -> AddResult {
    let mut cache = state.cache.write().unwrap();
    cache.push(item.clone());
    drop(cache);

    match state.enqueue() {
        Ok(_) => AddResult {
            added: true,
            queued: true,
            item: item.clone(),
            message: format!(
                "Added '{}' to cache and queued Redis/PostgreSQL commit.",
                item.key
            ),
        },
        Err(err) => AddResult {
            added: false,
            queued: false,
            item: item.clone(),
            message: format!(
                "Setting '{}' is cached but could not be queued: {}",
                item.key, err
            ),
        },
    }
}

#[tauri::command]
fn update_setting(item: SettingItem, state: tauri::State<'_, Arc<SettingsStore>>) -> UpdateResult {
    let mut cache = state.cache.write().unwrap();
    if let Some(existing) = cache.iter_mut().find(|existing| existing.key == item.key) {
        *existing = item.clone();
    } else {
        cache.push(item.clone());
    }
    drop(cache);

    match state.enqueue() {
        Ok(_) => UpdateResult {
            updated: true,
            queued: true,
            item: item.clone(),
            message: format!(
                "Updated '{}' in cache and queued Redis/PostgreSQL commit.",
                item.key
            ),
        },
        Err(err) => UpdateResult {
            updated: false,
            queued: false,
            item: item.clone(),
            message: format!(
                "Setting '{}' is cached but could not be queued: {}",
                item.key, err
            ),
        },
    }
}

#[tauri::command]
fn submit_settings(
    settings: Vec<SettingItem>,
    state: tauri::State<'_, Arc<SettingsStore>>,
) -> SubmitResult {
    let count = settings.len();
    if let Err(error) = state.save_records(&settings) {
        return SubmitResult {
            saved: false,
            queued: false,
            count,
            message: format!("Could not save records.csv: {}", error),
        };
    }

    let mut cache = state.cache.write().unwrap();
    *cache = settings;
    drop(cache);
    *state.load_error.write().unwrap() = None;

    let records_path = state.records_path.display().to_string();

    match state.enqueue() {
        Ok(_) => SubmitResult {
            saved: true,
            queued: true,
            count,
            message: format!("Saved {} entries to {}.", count, records_path),
        },
        Err(err) => SubmitResult {
            saved: true,
            queued: false,
            count,
            message: format!("Saved {} entries to {}; remote sync unavailable: {}", count, records_path, err),
        },
    }
}

#[tauri::command]
fn delete_setting(key: String, state: tauri::State<'_, Arc<SettingsStore>>) -> DeleteResult {
    {
        let mut cache = state.cache.write().unwrap();
        if let Some(index) = cache.iter().position(|item| item.key == key) {
            cache.remove(index);
        }
    }

    let remaining_count = state.to_items().len();
    match state.enqueue() {
        Ok(_) => DeleteResult {
            deleted: true,
            queued: true,
            key: key.clone(),
            remaining_count,
            message: format!(
                "Deleted '{}' from cache and queued Redis/PostgreSQL commit.",
                key
            ),
        },
        Err(err) => DeleteResult {
            deleted: false,
            queued: false,
            key: key.clone(),
            remaining_count,
            message: format!(
                "'{}' is deleted from cache but could not be queued: {}",
                key, err
            ),
        },
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let records_path = app.path().app_data_dir()?.join("records.csv");
            fs::create_dir_all(records_path.parent().unwrap())?;
            app.manage(SettingsStore::new(records_path));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            read_settings,
            get_app_status,
            add_setting,
            update_setting,
            submit_settings,
            delete_setting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_csv_round_trips_name_and_value() {
        let path = env::temp_dir().join(format!("mtokengui-records-{}.csv", std::process::id()));
        let (queue, _receiver) = mpsc::sync_channel(1);
        let store = SettingsStore {
            cache: RwLock::new(Vec::new()),
            load_error: RwLock::new(None),
            records_path: path.clone(),
            queue,
        };
        let settings = vec![SettingItem {
            key: "Display name".to_string(),
            value: "A, B".to_string(),
        }];

        store.save_records(&settings).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap().lines().next(), Some("name,value"));
        assert_eq!(read_records(&path).unwrap()[0].key, "Display name");
        assert_eq!(read_records(&path).unwrap()[0].value, "A, B");
        fs::remove_file(path).unwrap();
    }
}
