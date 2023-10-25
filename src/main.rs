use chrono::{DateTime, Utc};
use data::{ExportFile, Snapshot};
use dotenvy;
use flexi_logger::{Logger, LoggerHandle, FileSpec, FlexiLoggerError};
use serde::{Deserialize, Serialize};
use std::env;
use untis::Date; 
use sha2::{Digest, Sha256};
use crate::data::Lesson;
use log::{error,  info,  trace};

mod data;

type Result<T> = anyhow::Result<T>;

#[derive(Debug)]
/// Config repräsentiert die Konfiguration die aus der .env Datei geladen wird.
struct Config {
    /// Server auf dem Untis läuft
    server: String,
    /// Schule für die der Stundenplan abgerufen werden soll
    school: String,
    /// Benutzername für den Untis Account
    user: String,
    /// Passwort für den Untis Account
    password: String,
    /// Secret für die Pseudonymisierung der Lehrernamen
    secret: String,
    /// Pfad an dem die Daten gespeichert werden sollen
    path: String,
    /// Pfad an dem die Status Datei gespeichert werden soll
    state_file_path: Option<String>,
    /// URL unter der die Status Datei abgerufen werden kann
    state_file_check: Option<String>,
}


/// Lädt die .env Datei und überschreibt die bereits gesetzten Variablen
fn load_dotenv(){
    // Lädt die .env Datei, wenn sie nicht gefunden wird wird eine Fehlermeldung ausgegeben.
    if let Err(_) = dotenvy::dotenv_override() {
        println!("Failed to load \".env\" file.");
    }
}

/// Lädt die Konfiguration aus der .env Datei
fn load_config() -> Result<Config> {

    // Lädt die Variablen aus der .env Datei, wenn eine Variable nicht gefunden wird, wird ein Fehler zurückgegeben.
    Ok(Config {
        server: env::var("SERVER")?,
        school: env::var("SCHOOL")?,
        user: env::var("USERNAME")?,
        password: env::var("PASSWORD")?,
        secret: env::var("SECRET")?,
        path: env::var("STORAGE_PATH")?,
        state_file_path: env::var("STATE_PATH").ok(),
        state_file_check: env::var("STATE_CHECK_URL").ok(),
    })
}

/// Erstellt einen Snapshot des Stundenplans
///
/// # Arguments
/// * `client` - Untis Client mit dem die Daten abgerufen werden sollen
/// * `secret` - Das Secret die Pseudonymisierung der Lehrernamen benötigt wird
///
/// # Returns
/// * `Snapshot` - Snapshot des Stundenplans

fn create_snapshot(client: &mut untis::Client, secret: &str) -> Result<Snapshot> {
    // Erstellt einen neuen Snapshot
    let mut snapshot = Snapshot::new();

    // Lädt alle Klassen der Schule
    let classes = client.classes().unwrap();

    // Füge die Stundenpläne der Klassen zum Snapshot hinzu
    classes.iter().for_each(|class| {

        trace!("Lade Stundenplan für Klasse: {}", class.name);
        // Lädt den Stundenplan der Klasse
        match client.timetable_between(
            &class.id,
            &untis::ElementType::Class,
            &Date::today(),
            &Date::today(),
        ) {
            Ok(lessons) => {
                // Gehe durch alle Stunden und füge sie zum Snapshot hinzu
                lessons.iter().for_each(|lesson| {
                    // Wandelt die Lesson in eine Lesson um, die in der ExportDatei gespeichert werden kann
                    let mut lesson: Lesson = lesson.into();

                    // Pseudonymisiere die Lehrernamen
                    let teachers = lesson
                        .teachers
                        .iter()
                        .map(|teacher| {
                            // Erstellt einen Hash aus dem Secret und dem Lehrernamen
                            let mut hasher = Sha256::new();
                            hasher.update(secret);
                            hasher.update(teacher);

                            // Gibt den Hash als Hex String zurück
                            format!("{:x}", hasher.finalize())
                        })
                        .collect();
                    // Speichert die pseudonymisierten Lehrernamen
                    lesson.teachers = teachers;

                    // Fügt die Lesson zum Snapshot hinzu
                    snapshot.add_lesson(lesson)
                })
            }
            Err(e) => {
                error!("Error: {:#?}", e)
            }
        }
    });

    Ok(snapshot)
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
/// Status repräsentiert den Status des Programms
pub enum State {
    /// Das Programm wurde erfolgreich ausgeführt
    SUCCESS,
    /// Das Programm wurde mit einem Fehler beendet
    ERROR(String),
    /// Das Programm wurde gestartet und läuft noch
    STARTED,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
/// ReportedState repräsentiert den Status des Programms der in der Status Datei gespeichert wird
pub struct ReportedState {
    /// Status des Programms
    state: State,
    /// Zeitpunkt zu dem der Status gesetzt wurde
    timestamp: DateTime<Utc>,
}

/// Aktualisiert den Status des Programms
///
/// # Arguments
/// * `path` - Pfad an dem die Status Datei gespeichert werden soll
/// * `state` - Status der gesetzt werden soll
fn update_state(path: &str, state: State) -> Result<()> {
    let state = ReportedState {
        state,
        timestamp: Utc::now(),
    };
    let state = serde_json::to_string(&state)?;
    std::fs::write(path, state)?;
    Ok(())
}

/// Erstellt einen Logger mit den Log Leveln die in der .env Datei gesetzt sind.
/// Der Logger loggt in die Konsole und in eine Log Datei. Die Log Datei wird jeden Tag rotiert.
/// 
/// # Returns
/// * `LoggerHandle` - LoggerHandle mit dem der Logger gesteuert werden kann
fn init_logger() -> std::result::Result<LoggerHandle,FlexiLoggerError>{ 
    Logger::try_with_env_or_str("warn")? 
    .log_to_file(FileSpec::default().directory(env::var("LOG_PATH").unwrap_or("log/".to_string()))) // Logt in eine Datei im Ordner "log" oder in den Ordner der in der .env Datei unter "LOG_PATH" gesetzt ist
    .rotate(flexi_logger::Criterion::Age(flexi_logger::Age::Day), flexi_logger::Naming::Numbers, flexi_logger::Cleanup::KeepLogFiles(10)) // Rotiert die Log Datei
    .duplicate_to_stderr(flexi_logger::Duplicate::All) // Logt zusätzlich in die Konsole
    .start()
}

fn main() {

    load_dotenv();
    let _logger = match init_logger(){
        Ok(logger) => logger,
        Err(e) => {
            let error_msg = format!("Logger konnte nicht erstellt werden. {:#?}",e);
            println!("{}", error_msg); 
            return;
        }
    };
    info!("Logger wurde erstellt.");
    // Lädt die Konfiguration aus der .env Datei, wenn eine Variable nicht gefunden wird, wird das Programm beendet.
    let config = match load_config() {
        Ok(config) => config,
        Err(_) => {
            let error_msg = "Laden der Konfiguration fehlgeschlagen.";
            error!("{}", error_msg); 
            return;
        }
    };
 
    // Wenn STATE_CHECK_URL gesetzt ist wird der Status des Programms auf dem Hauptserver abgefragt
    if let Some(status_file_check) = &config.state_file_check {
        match reqwest::blocking::get(status_file_check) {
            // Wenn der Status erfolgreich abgerufen wurde, wird überprüft ob das Programm bereits läuft oder erfolgreich ausgeführt wurde
            Ok(response) => {
                if response.status().is_success() {
                    // Deserialisiert den Status
                    let state: ReportedState = response.json().unwrap();
                    // Prüfe ob der Status vor weniger als einer Stunde gesetzt wurde
                    if state.timestamp + chrono::Duration::hours(1) > Utc::now() {
                        // Wenn der Status vor weniger als einer Stunde gesetzt wurde, wird überprüft ob das Programm bereits läuft oder erfolgreich ausgeführt wurde
                        match state.state {
                            // Wenn das Programm bereits läuft wird eine Meldung ausgegeben und das Programm beendet
                            State::STARTED => {
                                info!("Das Programm läuft auf den Hauptserver bereits.");
                                return;
                            }
                            // Wenn das Programm erfolgreich ausgeführt wurde wird eine Meldung ausgegeben und das Programm beendet
                            State::SUCCESS => {
                                info!("Das Programm wurde auf den Hauptserver erfolgreich ausgeführt.");
                                return;
                            }

                            // Wenn das Programm mit einem Fehler beendet wurde wird eine Meldung ausgegeben und das Programm wird fortgesetzt
                            State::ERROR(error_msg) => {
                                error!("Hauptserver hat den Fehler: \"{}\"", error_msg);
                                info!("Daten werden Lokal abgerufen.")
                            }
                        }
                    }
                }
            }

            // Wenn der Status nicht erfolgreich abgerufen werden konnte wird eine Meldung ausgegeben und das Programm wird fortgesetzt
            Err(e) => {
                let error_msg = format!("Fehler beim abrufen des Status: \"{:#?}\"", e);
                error!("{}", error_msg);
                info!("Daten werden abgerufen.");
            }
        }
    }

    // Wenn STATE_PATH gesetzt ist wird der Status des Programms auf STARTED gesetzt
    if let Some(path) = &config.state_file_path {
        if let Err(e) = update_state(path, State::STARTED) {
            let error_msg = format!("Fehler beim setzen des Status. {:#?}", e);
            error!("{}", error_msg);
        }
    }

    // Erstellt einen neuen Client und loggt sich ein. Wenn das Login fehlschlägt wird eine Fehlermeldung ausgegeben und das Programm beendet.
    let mut client = match untis::Client::login(
        &config.server,
        &config.school,
        &config.user,
        &config.password,
    ) {
        Ok(client) => client,
        Err(e) => {
            let error_msg = format!("Login fehlgeschlagen. {:#?}", e);
            error!("{}", error_msg);

            // Wenn STATE_PATH gesetzt ist wird der Status des Programms auf ERROR gesetzt
            if let Some(path) = &config.state_file_path {
                if let Err(e) = update_state(path, State::ERROR(error_msg)) {
                    let error_msg = format!("Fehler beim setzen des Status. {:#?}", e);
                    error!("{}", error_msg);
                }
            }
            return;
        }
    };

    // Lädt die ExportDatei, wenn sie nicht existiert wird eine neue erstellt.
    let mut export_file = match ExportFile::load(&config.path) {
        Ok(export_file) => export_file,
        Err(e) => {
            let error_msg = format!("Fehler beim Laden der ExportFile. {:#?}", e);
            error!("{}", error_msg);

            // Wenn STATE_PATH gesetzt ist wird der Status des Programms auf ERROR gesetzt
            if let Some(path) = &config.state_file_path {
                if let Err(e) = update_state(path, State::ERROR(error_msg)) {
                    let error_msg = format!("Fehler beim setzen des Status. {:#?}", e);
                    error!("{}", error_msg);
                }
            }
            return;
        }
    };

    let snapshot = {
        match create_snapshot(&mut client, &config.secret) {
            Ok(snapshot) => snapshot,
            Err(e) => {
                let error_msg = format!("Fehler beim erstellen des Snapshots. {:#?}", e);
                error!("{}", error_msg);

                // Wenn STATE_PATH gesetzt ist wird der Status des Programms auf ERROR gesetzt
                if let Some(path) = &config.state_file_path {
                    if let Err(e) = update_state(path, State::ERROR(error_msg)) {
                        let error_msg = format!("Fehler beim setzen des Status. {:#?}", e);
                        error!("{}", error_msg);
                    }
                }
                return;
            }
        }
    };

    // Füge den Snapshot zur ExportDatei hinzu
    export_file.add(snapshot);

    // Speichert die ExportDatei
    if let Err(e) = export_file.save(&config.path) {
        let error_msg = format!("Fehler beim Speichern der ExportFile. {:#?}", e);
        error!("{}", error_msg);

        // Wenn STATE_PATH gesetzt ist wird der Status des Programms auf ERROR gesetzt
        if let Some(path) = &config.state_file_path {
            if let Err(e) = update_state(path, State::ERROR(error_msg)) {
                let error_msg = format!("Fehler beim setzen des Status. {:#?}", e);
                error!("{}", error_msg);
            }
        }
        return;
    } 
    // Wenn STATE_PATH gesetzt ist wird der Status des Programms auf SUCCESS gesetzt
    if let Some(path) = &config.state_file_path {
        if let Err(e) = update_state(path, State::SUCCESS) {
            let error_msg = format!("Fehler beim setzen des Status. {:#?}", e);
            error!("{}", error_msg);
        }
    }
    info!("Daten wurden erfolgreich abgerufen.")
}
