# School-Mining-Scraper

School Mining ist ein Projekt welches im Rahmen des Wahlpflichtfachs "Data Mining" an der BBS Montabaur entstanden ist. Dieses Repository enthält den Quellcode für den Scraper, welcher Daten von den WebUntis Stundenplänen der Schule extrahiert und in einen binär Format basierend auf [rkyv](https://github.com/rkyv/rkyv) speichert.

Die Daten werden anschließend von einem anderen Programm für die Datenbank aufbereitet und in diese eingefügt.

## Installation

### Voraussetzungen für automatische Installation

Betriebssystem: Ubuntu 22.04

Das Verzeichnis `/srv` darf nicht existieren oder von der Gruppe `admin` besessen werden.
Die Gruppe `admin` sollte nicht existieren, da diese angelegt wird und der aktuelle Benutzer dieser Gruppe hinzugefügt wird.


### Schritte

1. Rust installieren: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
2. Just installieren: `cargo install just`
3. Repository klonen: `git clone https://github.com/FaunKr/school-mining-scraper.git`
4. System vorbereiten: `just prepare`
5. Scraper ausführen: `just install`

#### Hinweise

`just prepare` muss nur einmal ausgeführt werde und legt das Verzeichnis `/srv` an, erstellt die Gruppe `admin`, fügt den aktuellen Benutzer dieser Gruppe hinzu und gibt der Gruppe die Rechte für das Verzeichnis `/srv`.

`just install` erstellt das Verzeichnis `/srv/school-mining` und installiert den Scraper in dieses Verzeichnis. Für die Ausführung des Scrapers müssen allerdings noch die Umgebungsvariablen gesetzt werden. Dies kann über die Shell oder eine .env Datei geschehen.

Ein cron Job wird nicht automatisch erstellt.

## Konfiguration

Die Konfiguration erfolgt über Umgebungsvariablen. Diese können entweder über die Shell oder eine .env Datei gesetzt werden.
Folgenden Umgebungsvariablen können gesetzt werden:


| **Variable**      | **Erklärung**                                                              |
| ------------------- | :---------------------------------------------------------------------------- |
| `USERNAME`        | Benutzername für den Login bei der WebUntis Api                            |
| `PASSWORD`        | Passwort für den Login bei der WebUntis Api                                |
| `SCHOOL`          | Name der Schule                                                             |
| `SERVER`          | Server der Schule                                                           |
| `SECRET`          | Secret für die WebUntis Api                                                |
| `STORAGE_PATH`    | Pfad zum Speichern der Daten                                                |
| `STATE_PATH`      | Pfad zum Speichern des Zustands (Für Failover Betrieb)                                            |
| `STATE_CHECK_URL` | Url zum Überprüfen des Zustands falls ein Failover Server eingesetzt wird  (Für Failover Betrieb) |
| `RUST_LOG` | Log Level (`trace`,`debug`,`info`,`warn`,`error`) |
| `LOG_PATH` | Path to logging directory |


### Beispiel .env Datei

```env
USERNAME=webuntis
PASSWORD=webuntis
SCHOOL=Schule
SERVER=schule.webuntis.com
SECRET=secret
STORAGE_PATH=/srv/school-mining/storage
STATE_PATH=/srv/school-mining/state
STATE_CHECK_URL=https://192.168.178.12/check.json
```

### Beispiel für den Cron Job

#### Hauptserver  
```cron
0 2,6,8,20 * * * cd /srv/school-mining; ./school-mining-scraper
```
#### Failoverserver
```cron	
10 2,6,8,20 * * * cd /srv/school-mining; ./school-mining-scraper
```
## Vorraussetzungen für ein Setup mit Failover Server
Die folgenden Vorraussetzungen müssen erfüllt sein, damit ein Failover Server eingesetzt werden kann.

1. Der Failover Server muss übers Netzwerk den Zustand des Hauptserver überprüfen können. Dies geschieht über eine GET Anfrage an die URL `STATE_CHECK_URL`.
2. Auf den Hauptserver muss ein Webserver installiert sein, welcher statische Dateien ausliefern kann. Der Scraper muss die Berechtigung haben, diese Dateien zu erstellen und zu überschreiben. Die Umgebungsvariable `STATE_PATH` muss auf ein Verzeichnis zeigen, welches vom Webserver ausgeliefert wird.
3. Auf dem Failover Server muss die Umgebungsvariable `SECRET` gesetzt sein. Diese muss mit der Umgebungsvariable `SECRET` auf dem Hauptserver übereinstimmen.
4. Auf den Failover Server muss ein Cronjob sein, der den Scraper regelmäßig ausführt. Dies sollte 10-20 Minuten nach dem Cronjob auf dem Hauptserver geschehen. 
5. Die Variable `STORAGE_PATH` sollte auf ein Verzeichnis zeigen, welches von beiden Servern erreichbar ist. Dies kann z.B. ein NFS Share sein.