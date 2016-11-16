// Copyright (c) 2016 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Write, Read};
use std::path::Path;
use std::time::{UNIX_EPOCH, SystemTime};
use uuid::Uuid;
use rustc_serialize::json::{ToJson, Json};
use fs::cache_analytics_path;

// Supported events
pub const EVENT_BUILDER_PROJECT_CREATE: &'static str = "builder-project-create";

// Sample event JSON payload (compatible with Segment.io)
// {
//   "type": "track",
//   "event": "builder-project-create",
//   "properties": {
//     "clientid" : "0a5c0882-ade5-46cf-821d-8d3853cd0d41"
//     "timestamp": "1479330000.13442404",
//   }
// }

const CLIENT_ID_METAFILE: &'static str = "CLIENT_ID";

#[derive(Debug, Clone)]
pub struct Event {
    name: String,
    clientid: String,
    timestamp: String,
    properties: BTreeMap<String, String>,
}

impl Event {
    pub fn new(name: &str, clientid: &str, timestamp: &str) -> Self {
        let mut properties = BTreeMap::new();
        properties.insert("timestamp".to_string(), timestamp.to_string());
        properties.insert("clientid".to_string(), clientid.to_string());

        Event {
            name: name.to_string(),
            clientid: clientid.to_string(),
            timestamp: timestamp.to_string(),
            properties: properties,
        }
    }
}

impl ToJson for Event {
    fn to_json(&self) -> Json {
        let mut p = BTreeMap::new();
        for (key, value) in self.properties.iter() {
            p.insert(key.to_string(), value.to_json());
        }

        let mut m = BTreeMap::new();
        m.insert("type".to_string(), "track".to_string().to_json());
        m.insert("event".to_string(), self.name.to_json());
        m.insert("properties".to_string(), p.to_json());

        Json::Object(m)
    }
}

fn read_file(file_path: &Path) -> String {
    let mut content = String::new();
    let mut file = File::open(file_path).expect("Unable to open file");
    file.read_to_string(&mut content).expect("Unable to read file");
    content
}

fn write_file(parent_dir: &Path, file_path: &Path, content: &str) {
    fs::create_dir_all(parent_dir).expect("Unable to create directory");
    let mut file = File::create(&file_path).expect("Unable to create file");
    file.write_all(content.as_bytes()).expect("Unable to write file");
}

fn timestamp() -> String {
    let (secs, subsec_nanos) = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => (duration.as_secs(), duration.subsec_nanos()),
        Err(e) => {
            debug!("Cannot generate system time: {}", e);
            return "0.0".to_string();
        }
    };
    format!("{}.{}", secs, subsec_nanos)
}

fn client_id() -> String {
    let cache_dir = cache_analytics_path(None);
    let file_path = cache_dir.join(CLIENT_ID_METAFILE);

    if file_path.exists() {
        read_file(&file_path)
    } else {
        let uuid = Uuid::new_v4().hyphenated().to_string();
        write_file(&cache_dir, &file_path, &uuid);
        uuid
    }
}

pub fn record_event(name: &str) {
    let timestamp: &str = &timestamp();
    let clientid: &str = &client_id();
    let event = Event::new(name, timestamp, clientid);

    let cache_dir = cache_analytics_path(None);
    let file_path = cache_dir.join(format!("event-{}.json", &event.timestamp));

    write_file(&cache_dir, &file_path, &event.to_json().to_string());
}

#[cfg(test)]
mod test {
    use super::Event;
    use rustc_serialize::json::ToJson;

    #[test]
    fn event_to_json() {
        let event = Event::new("foo", "bar", "baz");
        let encoded = event.to_json();
        let expected =
            r#"{"event":"foo","properties":{"clientid":"bar","timestamp":"baz"},"type":"track"}"#;
        assert!(encoded.to_string() == expected.to_string());
    }
}
