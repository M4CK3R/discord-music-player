use regex::Regex;
use youtube_dl::SingleVideo;

static YOUTUBE_REGEX: &str = r"(https?:\/\/)?(www\.)?(m\.)?(music\.)?((youtube)|(youtu\.be)).*";

pub(crate) fn is_youtube_link(url: &str) -> bool {
    Regex::new(YOUTUBE_REGEX).unwrap().is_match(url)
}

pub(crate) fn get_url(sv: &SingleVideo) -> Option<String> {
    let webpage_url = sv.webpage_url.clone();
    if webpage_url.is_some() {
        return webpage_url;
    }

    return sv.url.clone();
}
