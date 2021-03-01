use std::io::prelude::*;
use std::thread;
use std::os::unix::net::{UnixStream, UnixListener};
use std::io::BufReader;
use std::sync::Arc;
use std::fs;
use std::env::var;

use adblock::engine::Engine;
use adblock::lists::{FilterFormat, FilterSet};

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
    let mut rules = String::new();
    let home_dir = var("HOME").unwrap();
    let lists_dir = home_dir.to_owned() + "/.config/ars/lists";

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

    let blocker = Arc::new(Engine::from_filter_set(filter_set, true));

    let socket_path = "/tmp/ars";

    if std::path::Path::new(socket_path).exists() {
        fs::remove_file(socket_path).unwrap();
    }

    let listener = UnixListener::bind(socket_path).unwrap();
    println!("init-done");

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
