use std::{future::Future, time::Duration};
use youtube_dl::{SingleVideo, YoutubeDl};

use crate::audio_manager::utils::get_url;

async fn get_urls(url: &str) -> Vec<String> {
    let info = YoutubeDl::new(url).flat_playlist(true).run_async().await;
    if info.is_err() {
        return vec![];
    }
    let info = info.unwrap();

    if let Some(sv) = info.clone().into_single_video() {
        match get_url(&sv) {
            Some(url) => return vec![url],
            None => return vec![],
        }
    }

    if let Some(pl) = info.into_playlist() {
        let mut res = Vec::new();
        if pl.entries.is_none() {
            return vec![];
        }
        let entries = pl.entries.unwrap();
        for sv in entries {
            if let Some(url) = get_url(&sv) {
                res.push(url);
            }
        }
        return res;
    }

    vec![]
}

async fn download_song(
    url: &str,
    audio_files_output_template: &str,
) -> Result<SingleVideo, String> {
    let i = YoutubeDl::new(url.clone())
    .output_template(audio_files_output_template)
    .format("ba")
    .run_async()
    .await
    .map_err(|e| e.to_string())?;

    YoutubeDl::new(url.clone())
        .output_template(audio_files_output_template)
        .format("ba")
        .download_to_async("./")
        .await
        .map_err(|e| e.to_string())?;

    i.into_single_video()
        .ok_or("Not a single video".to_string())
}

pub(crate) async fn download_audio_files(
    url: &str,
    audio_files_output_template: &str,
) -> Result<Vec<SingleVideo>, String> {
    let urls = get_urls(url).await;
    let mut handles: Vec<_> = Vec::new();
    for url in urls {
        let t = audio_files_output_template.to_string();
        let h = tokio::spawn(async move {
            let sv = retry(|| download_song(&url, &t), 5).await;
            if sv.is_err() {
                return None;
            }
            let sv = sv.unwrap();
            return Some(sv);
        });
        handles.push(h);
    }
    let mut res: Vec<SingleVideo> = Vec::new();
    for h in handles {
        let sv = h.await;
        if sv.is_err() {
            continue;
        }
        let sv = sv.unwrap();
        if let Some(sv) = sv {
            res.push(sv);
        }
    }
    Ok(res)
}

async fn retry<F, R, E, Fut>(f: F, n: u32) -> Result<R, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<R, E>>,
{
    let mut i = 0;
    loop {
        let res = f().await;
        if res.is_ok() {
            return res;
        }
        i += 1;
        if i >= n {
            return res;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}