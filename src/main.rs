use std::io::prelude::*;
use std::thread;
use std::os::unix::net::{UnixStream, UnixListener};
use std::io::BufReader;
use std::sync::Arc;
use std::fs;
use std::env::var;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use adblock::engine::Engine;
use adblock::lists::{FilterFormat, FilterSet};

use attohttpc;

fn handle_client(mut stream: UnixStream, blocker: Arc<Engine>) {
    let buf = BufReader::new(stream.try_clone().unwrap());
    for line in buf.lines() {
        let uline = line.unwrap();
        let mut parts = uline.split(' ');
        let mut res = String::new();

        match parts.next().unwrap() {
            "n" => {
                let source = parts.next().unwrap();
                let req_url = parts.next().unwrap();
                let req_type = parts.next().unwrap();

                let result = blocker.check_network_urls(source, req_url, req_type);

                if result.matched == true { res.push('1') } else { res.push('0') };
            },
            "c" => {
                let url = parts.next().unwrap();
                let resources = blocker.url_cosmetic_resources(url);
                let ids : Vec<String> = parts.next().unwrap().split('\t').map(|x| x.to_string()).collect();
                let classes : Vec<String> = parts.next().unwrap().split('\t').map(|x| x.to_string()).collect();

                let mut selectors = blocker.hidden_class_id_selectors(&classes, &ids, &resources.exceptions);
                let mut hides : Vec<String> = resources.hide_selectors.into_iter().collect();
                selectors.append(&mut hides);
                let mut style = selectors.join(", ");

                if style.len() != 0 {
                    style.push_str(" { display: none !important; }");
                }
                style.push('\n');

                res.push_str(&style);
            },
            _ => {
                res.push_str("Unknown code supplied");
            }
        };

        stream.write(res.as_bytes()).unwrap();
    }
}

fn main() -> std::io::Result<()> {
    let home_dir = var("HOME").unwrap();
    let config_dir = home_dir.to_owned() + "/.config/ars";
    let lists_dir = config_dir.to_owned() + "/lists";
    let engine_file = config_dir.to_owned() + "/engine";
    let urls_file = config_dir.to_owned() + "/urls";

    let mut updated = false;
    let mut blocker;

    fs::create_dir_all(&lists_dir).unwrap();

    if Path::new(&urls_file).exists() {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&urls_file)
            .unwrap();
        let reader = BufReader::new(file.try_clone().unwrap());
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let mut out = String::new();

        for line in reader.lines() {
            let line = line.unwrap();
            let mut parts = line.split(' ');
            let url = parts.next().unwrap();

            if parts.clone().count() == 0 || parts.next().unwrap().parse::<u64>().unwrap() < timestamp.as_secs() {
                // list needs to be updated
                updated = true;
                let filename = url.split('/').last().unwrap();
                let mut res = attohttpc::get(&url).send().unwrap();
                let f = fs::File::create(lists_dir.to_owned() + "/" + filename).unwrap();

                let freader = BufReader::new(&mut res);
                for fline in freader.lines() {
                    let fline = fline.unwrap();
                    if fline.contains("! Expires: ") {
                        let days = fline.split(' ').nth(2).unwrap().parse::<u64>().unwrap() * 24 * 3600;
                        let stamp = std::time::Duration::new(days, 0) + SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                        out.push_str(&url);
                        out.push(' ');
                        out.push_str(&stamp.as_secs().to_string());
                        break;
                    }
                }

                res.write_to(f).unwrap();
            } else {
                out.push_str(&line);
            }

            out.push('\n');
        }

        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        file.write_all(&out.into_bytes()).unwrap();
    }

    if Path::new(&engine_file).exists() && !updated {
        blocker = Engine::new(true);
        let data = fs::read(engine_file).unwrap();
        blocker.deserialize(&data).unwrap();
    } else {
        let mut rules = String::new();

        for entry in fs::read_dir(lists_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_file() {
                let mut temp = String::new();
                let mut file = fs::File::open(path).unwrap();
                file.read_to_string(&mut temp).unwrap();
                rules.push_str(&temp);
            }
        }

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filter_list(&rules, FilterFormat::Standard);
        blocker = Engine::from_filter_set(filter_set, true);
        let data = blocker.serialize().unwrap();
        fs::write(engine_file, data).unwrap();
    }

    let socket_path = "/tmp/ars";

    if std::path::Path::new(socket_path).exists() {
        fs::remove_file(socket_path).unwrap();
    }

    let listener = UnixListener::bind(socket_path).unwrap();
    println!("init-done");

    let blocker = Arc::new(blocker);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let blocker = blocker.clone();
                thread::spawn(move || handle_client(stream, blocker));
            }
            Err(err) => {
                println!("Error: {}", err);
            }
        }
    }

    Ok(())
}
