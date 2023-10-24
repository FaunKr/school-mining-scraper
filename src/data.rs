
use std::{path::Path, fs::File, io::{BufReader, Read}};

use chrono::{DateTime, Utc, Local, Datelike};
use rkyv::{Archive,Serialize,Deserialize, archived_root, ser::{serializers::AllocSerializer, Serializer}};

type Result<T> = anyhow::Result<T>;

/// 'ExportFile' repräsentiert die Datei in der die Rohdaten gespeichert werden. 
#[derive(Archive,Serialize,Deserialize,Debug)]
pub struct ExportFile {
    /// Datum der Exportieren Daten
    date: DateTime<Utc>,
    /// Snapshots des Stundenplans in der Exportieren Datei
    snapshots: Vec<Snapshot>, 
}

impl ExportFile{
    /// Lädt die Exportierte Datei aus dem angegebenen Pfad. Wenn die Datei nicht existiert wird eine neue Datei erstellt.
    /// # Arguments
    /// * `path` - Pfad an dem die Datei gespeichert werden soll
    /// 
    /// # Returns
    /// * `ExportFile` - Exportierte Datei
    pub fn load(path: &str) -> Result<Self>{
        // Ruft das aktuelle Datum ab
        let now = Local::now();
        // Erzeugt den Pfad an dem die Datei gespeichert werden soll
        let folder = format!("{}/{}/{}/", path,  now.year(),now.month()); 
        let full_path = format!("{}/{}.bin", folder,   now.day());
        // Überprüft ob die Datei existiert
        if Path::new(&full_path).exists(){
            // Lädt die Datei
            if let Ok(file) = File::open(full_path){

                // Liest die Datei in einen Buffer
                let mut buf_reader = BufReader::new(file);
                let mut buffer = Vec::new();
                let _ = buf_reader.read_to_end(&mut buffer);
                // Lädt die Datei aus dem Buffer
                let archived = unsafe { archived_root::<Self>(&buffer) };

                // Deserialisiert die Datei
                return  Ok(archived.deserialize(&mut rkyv::de::deserializers::SharedDeserializeMap::default())?);


            }
        }else{
            // Erstellt den Ordner in dem die Datei gespeichert werden soll
            std::fs::create_dir_all(folder).unwrap();
        } 

        // Gib das ExportFile struct zurück
        Ok(Self { date: Utc::now(), snapshots: Vec::new() })
    }

    /// Speichert die Datei an dem angegebenen Pfad
    /// 
    /// # Arguments
    /// * `path` - Pfad an dem die Datei gespeichert werden soll
    /// 
    pub fn save(self,path: &str) -> Result<()>{
        // Ruft das aktuelle Datum ab und erstellt den Pfad an dem die Datei gespeichert werden soll
        let now = Local::now();
        let path = format!("{}/{}/{}/{}.bin", path,  now.year(),now.month(),now.day()); 
        
        // Erstellt einen Serializer
        let mut serializer = AllocSerializer::<1024>::default();
        
        // Serialisiert das ExportFile struct
        serializer.serialize_value(&self).unwrap();
        let data = serializer.into_serializer().into_inner();
        
        // Speichert die Datei an dem angegebenen Pfad
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Fügt einen Snapshot der ExportFile hinzu
    /// 
    /// # Arguments
    /// * `snapshot` - Snapshot der hinzugefügt werden soll
    /// 
    pub fn add(&mut self, snapshot: Snapshot){
        self.snapshots.push(snapshot);
    }
}


#[derive(Archive,Serialize,Deserialize,Debug)]
/// 'Snapshot' ist eine Momentaufnahme des Stundenplans. 
pub struct Snapshot {
    /// Datum mit Zeitpunkt des jeweiligen Snapshots
    datetime: DateTime<Utc>, 
    /// Unterrichtstunden die zum Zeitpunkt des Snapshots auf den Stundenplan hinterlegt waren
    lessons: Vec<Lesson>, 
}

impl Snapshot {
    /// Erstellt einen neuen Snapshot
    /// 
    /// # Returns
    /// * `Snapshot` - Neuer Snapshot
    pub fn new() -> Self {
        Self { datetime: Utc::now(), lessons: Vec::new() }
    }
    
    /// Fügt eine Unterrichtsstunde dem Snapshot hinzu
    /// 
    /// # Arguments
    /// * `lesson` - Unterrichtsstunde die hinzugefügt werden soll
    pub fn add_lesson(&mut self, lesson: Lesson){
        self.lessons.push(lesson)
    }
}


#[derive(Archive,Serialize,Deserialize,Debug)]
/// 'Lesson' repräsentiert eine Unterrichtsstunde, die auf dem Stundenplan hinterlegt ist.
/// 
pub struct Lesson {
    /// Klassen die an der Unterrichtsstunde teilnehmen
    pub classes: Vec<String>,
    /// Lehrer die die Unterrichtsstunde halten
    pub teachers: Vec<String>,
    /// Räume in denen die Unterrichtsstunde stattfindet
    pub rooms: Vec<String>,
    /// Art der Unterrichtsstunde
    pub lesson_code: LessonCode,
    /// Beschreibung der Unterrichtsstunde
    pub description: String,
    /// Thema der Unterrichtsstunde
    pub topic: String,
    /// Vertretingshinweis der Unterrichtsstunde
    pub sub_text: Option<String>,
}

#[derive(Archive,Serialize,Deserialize,Debug)]
/// 'LessonCode' repräsentiert die Art der Unterrichtsstunde
pub enum LessonCode{
    /// Reguläre Unterrichtsstunde
    Regular,
    /// Unregelmäßige Unterrichtsstunde (z.B. Vertretung)
    Irregular,
    /// Ausgefallene Unterrichtsstunde
    Cancelled
}

impl From<&untis::Lesson> for Lesson{
    fn from(value: &untis::Lesson) -> Self {

        // Erstellt einen String mit dem Wert 'None' als Standardwert für das Thema der Unterrichtsstunde
        let none = "None".to_string();
        Lesson { 
            // Konvertiert die Klassen, Lehrer und Räume in einen String Vector
            classes: value.classes.iter().map(|class|class.name.to_string()).collect(), 
            teachers: value.teachers.iter().map(|teacher| teacher.name.to_string()).collect(), 
            rooms: value.rooms.iter().map(|room|room.name.to_string()).collect(), 

            // Konvertiert den Unterrichtsstunden Code in einen LessonCode
            lesson_code: match value.code {
                untis::LessonCode::Regular => LessonCode::Regular,
                untis::LessonCode::Irregular => LessonCode::Irregular,
                untis::LessonCode::Cancelled => LessonCode::Cancelled,
            },
            description: value.lstext.to_owned(), 
            // Konvertiert das Thema der Unterrichtsstunde in einen String, wenn es vorhanden ist. Ansonsten wird der Standardwert 'None' zurückgegeben
            // Sollte mehr als ein Thema vorhanden sein, wird nur das erste Thema zurückgegeben
            topic: value.subjects.iter().take(1).map(|subject|subject.name.clone()).collect::<Vec<String>>().get(0).unwrap_or(&none).to_owned(), 
            sub_text: value.subst_text.to_owned()
        }
    }
}