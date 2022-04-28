use regex::Regex;

pub fn get_id(url:&str) -> Option<String> {
    lazy_static! {
        static ref ID_RE:Regex = Regex::new(r"https?://mangadex\.org/title/((?:[0-9a-fA-F]+-?)+)/?.*").unwrap();
    }

    let id = ID_RE.captures(url)?.get(1)?.as_str().to_string();
    Some(id)
}

pub fn escape_path(path:&str) -> String {
    lazy_static! {
        static ref RESERVED_RE:Regex = Regex::new(r"[\\/:|&<>]").unwrap();
    }

    RESERVED_RE.replace_all(path, "").to_string()
}
