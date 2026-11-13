//! Converts `yt-dlp -J` output into the [`MediaInfo`] the UI uses.

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;
use serde_json::Value;
use url::Url;

use super::model::*;

/// (short edge, long edge, menu label, short label)
const STANDARD: [(u32, u32, &str, &str); 9] = [
    (144, 256, "144p", "144p"),
    (240, 426, "240p", "240p"),
    (360, 640, "360p", "360p"),
    (480, 854, "480p", "480p"),
    (720, 1280, "720p HD", "720p"),
    (1080, 1920, "1080p Full HD", "1080p"),
    (1440, 2560, "2K", "2K"),
    (2160, 3840, "4K", "4K"),
    (4320, 7680, "8K", "8K"),
];

const AUDIO_QUALITIES: [(u32, &str); 4] = [
    (320, "320 kbps Studio"),
    (256, "256 kbps High"),
    (192, "192 kbps Normal"),
    (128, "128 kbps Small"),
];

/// 16 bit, 48 kHz, stereo PCM.
const WAV_BYTES_PER_SEC: f64 = 192_000.0;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawInfo {
    #[serde(rename = "_type")]
    kind: Option<String>,
    id: Option<String>,
    title: Option<String>,
    uploader: Option<String>,
    channel: Option<String>,
    thumbnail: Option<String>,
    thumbnails: Option<Vec<RawThumb>>,
    duration: Option<f64>,
    webpage_url: Option<String>,
    extractor_key: Option<String>,
    formats: Option<Vec<RawFormat>>,
    entries: Option<Vec<Option<RawEntry>>>,
    subtitles: Option<HashMap<String, Value>>,
    automatic_captions: Option<HashMap<String, Value>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawThumb {
    url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawEntry {
    id: Option<String>,
    url: Option<String>,
    webpage_url: Option<String>,
    title: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    thumbnails: Option<Vec<RawThumb>>,
    ie_key: Option<String>,
}

impl RawEntry {
    /// On a channel's "Playlists" tab the entries are sub-playlists, not videos.
    fn is_nested_playlist(&self) -> bool {
        self.ie_key.as_deref().is_some_and(|k| k.ends_with("Tab"))
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct RawFormat {
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f64>,
    vcodec: Option<String>,
    acodec: Option<String>,
    abr: Option<f64>,
    tbr: Option<f64>,
    filesize: Option<f64>,
    filesize_approx: Option<f64>,
    has_drm: Option<Value>,
}

impl RawFormat {
    fn has_video(&self) -> bool {
        self.height.is_some() && self.vcodec.as_deref() != Some("none")
    }

    fn has_audio(&self) -> bool {
        match (self.acodec.as_deref(), self.vcodec.as_deref()) {
            (Some("none"), _) => false,
            (Some(_), _) => true,
            // Direct files without codec info are combined streams.
            (None, None) => true,
            (None, Some(_)) => false,
        }
    }

    fn is_h264(&self) -> bool {
        self.vcodec
            .as_deref()
            .is_some_and(|c| c.starts_with("avc") || c.starts_with("h264"))
    }

    fn is_h265(&self) -> bool {
        self.vcodec
            .as_deref()
            .is_some_and(|c| c.starts_with("hev") || c.starts_with("hvc") || c.starts_with("h265"))
    }

    fn is_vp9(&self) -> bool {
        self.vcodec
            .as_deref()
            .is_some_and(|c| c.starts_with("vp9") || c.starts_with("vp09"))
    }

    fn is_aac(&self) -> bool {
        self.acodec
            .as_deref()
            .is_some_and(|c| c.starts_with("mp4a") || c.starts_with("aac"))
    }

    fn is_drm(&self) -> bool {
        matches!(&self.has_drm, Some(Value::Bool(true)))
    }

    fn known_size(&self) -> Option<f64> {
        self.filesize.or(self.filesize_approx).filter(|s| *s > 0.0)
    }

    fn size(&self, duration: Option<f64>) -> Option<u64> {
        self.known_size()
            .or_else(|| Some(self.tbr? * 125.0 * duration?))
            .filter(|s| *s > 0.0)
            .map(|s| s as u64)
    }
}

pub fn platform_name(extractor: Option<&str>, url: &str) -> String {
    let key = extractor.unwrap_or("").to_ascii_lowercase();
    let host = Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
        .unwrap_or_default();
    let matches = |k: &str, h: &[&str]| key.starts_with(k) || h.iter().any(|d| host.ends_with(d));
    if matches("youtube", &["youtube.com", "youtu.be"]) {
        "YouTube".into()
    } else if matches("tiktok", &["tiktok.com"]) {
        "TikTok".into()
    } else if matches("instagram", &["instagram.com"]) {
        "Instagram".into()
    } else if matches("twitter", &["x.com", "twitter.com"]) {
        "X".into()
    } else {
        extractor.unwrap_or("Web").to_string()
    }
}

/// Treats a `watch?v=…&list=…` link as a single video by default and offers the playlist separately.
pub fn parent_playlist(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    if !(host.ends_with("youtube.com") || host.ends_with("youtu.be")) {
        return None;
    }
    let param = |name: &str| {
        parsed
            .query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    let is_video = param("v").is_some() || host.ends_with("youtu.be");
    let list = param("list").filter(|_| is_video)?;
    Some(format!("https://www.youtube.com/playlist?list={list}"))
}

/// Maps the real size to a menu tier. Labels vertical and cinemascope video correctly;
/// non-standard sizes (like 576p) are shown as they are.
fn bucket(width: Option<u32>, height: u32) -> (u32, String, String) {
    let (short, long) = match width {
        Some(w) => (w.min(height), w.max(height)),
        None => (height, height),
    };
    let close =
        |a: u32, b: u32, tolerance: f64| (a as f64 - b as f64).abs() <= b as f64 * tolerance;
    for (s, l, label, short_label) in STANDARD {
        if close(short, s, 0.03) || (width.is_some() && close(long, l, 0.015)) {
            return (s, label.to_string(), short_label.to_string());
        }
    }
    (short, format!("{short}p"), format!("{short}p"))
}

/// Closest guess of the stream yt-dlp's `-S res,fps,…` sort will pick:
/// highest fps, then known size, then last in the list (best).
fn pick<'a>(formats: impl Iterator<Item = &'a RawFormat>) -> Option<&'a RawFormat> {
    formats.fold(None, |best: Option<&RawFormat>, f| match best {
        None => Some(f),
        Some(b) => {
            let rank = |x: &RawFormat| (x.fps.unwrap_or(0.0), x.known_size().is_some());
            let (fa, ka) = rank(f);
            let (fb, kb) = rank(b);
            if fa > fb || (fa == fb && ka >= kb) {
                Some(f)
            } else {
                Some(b)
            }
        }
    })
}

fn best_audio<'a>(formats: &'a [RawFormat], prefer_aac: bool) -> Option<&'a RawFormat> {
    let audio_only = || {
        formats
            .iter()
            .filter(|f| !f.has_video() && f.has_audio() && !f.is_drm())
    };
    let best_of = |items: Vec<&'a RawFormat>| {
        items
            .into_iter()
            .fold(None, |best: Option<&RawFormat>, f| match best {
                Some(b) if b.abr.unwrap_or(0.0) > f.abr.unwrap_or(0.0) => Some(b),
                _ => Some(f),
            })
    };
    if prefer_aac {
        if let Some(found) = best_of(audio_only().filter(|f| f.is_aac()).collect()) {
            return Some(found);
        }
    }
    best_of(audio_only().collect())
}

fn video_options(formats: &[RawFormat], duration: Option<f64>) -> Vec<VideoOption> {
    let mut buckets: BTreeMap<u32, (String, String, Vec<&RawFormat>)> = BTreeMap::new();
    for f in formats.iter().filter(|f| f.has_video() && !f.is_drm()) {
        let (rank, label, short_label) = bucket(f.width, f.height.unwrap_or(0));
        buckets
            .entry(rank)
            .or_insert_with(|| (label, short_label, Vec::new()))
            .2
            .push(f);
    }

    let aac = best_audio(formats, true).and_then(|f| f.size(duration));
    let any = best_audio(formats, false).and_then(|f| f.size(duration));
    let total = |f: &RawFormat, audio: Option<u64>| -> Option<u64> {
        let video = f.size(duration)?;
        Some(if f.has_audio() {
            video
        } else {
            video + audio.unwrap_or(0)
        })
    };
    let video_only = |f: &RawFormat| {
        if f.has_audio() {
            None
        } else {
            f.size(duration)
        }
    };

    buckets
        .into_iter()
        .rev()
        .map(|(_, (label, short_label, items))| {
            // Without H.264, `-S vcodec:h264` falls back to the closest codec: H.265, then VP9, then AV1.
            let mp4 = [RawFormat::is_h264, RawFormat::is_h265, RawFormat::is_vp9]
                .iter()
                .find_map(|matches| pick(items.iter().copied().filter(|f| matches(f))))
                .or_else(|| pick(items.iter().copied()))
                .expect("tier cannot be empty");
            let mkv = pick(items.iter().copied()).expect("tier cannot be empty");
            let tallest = items
                .iter()
                .copied()
                .max_by_key(|f| f.height)
                .unwrap_or(mkv);
            VideoOption {
                height: tallest.height.unwrap_or(0),
                width: tallest.width,
                label,
                short_label,
                fps: mkv.fps,
                size_mp4: total(mp4, aac),
                size_mkv: total(mkv, any),
                video_bytes_mp4: video_only(mp4),
                video_bytes_mkv: video_only(mkv),
            }
        })
        .collect()
}

fn audio_options(duration: Option<f64>) -> Vec<AudioOption> {
    AUDIO_QUALITIES
        .iter()
        .map(|(kbps, label)| AudioOption {
            kbps: *kbps,
            label: label.to_string(),
            size: duration.map(|d| (*kbps as f64 * 125.0 * d) as u64),
        })
        .collect()
}

fn last_thumb(thumbs: &Option<Vec<RawThumb>>) -> Option<String> {
    thumbs.as_ref()?.iter().rev().find_map(|t| t.url.clone())
}

pub fn parse_media_info(json: &str, requested_url: &str) -> Result<MediaInfo, serde_json::Error> {
    let raw: RawInfo = serde_json::from_str(json)?;
    let url = raw
        .webpage_url
        .clone()
        .unwrap_or_else(|| requested_url.to_string());
    let platform = platform_name(raw.extractor_key.as_deref(), &url);
    let thumbnail = raw
        .thumbnail
        .clone()
        .or_else(|| last_thumb(&raw.thumbnails));
    let title = raw
        .title
        .clone()
        .or(raw.id.clone())
        .unwrap_or_else(|| "Untitled".to_string());
    let uploader = raw.uploader.clone().or(raw.channel.clone());

    if raw.kind.as_deref() == Some("playlist") || raw.entries.is_some() {
        let entries: Vec<PlaylistEntry> = raw
            .entries
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .filter(|e| !e.is_nested_playlist())
            .enumerate()
            .filter_map(|(i, e)| {
                let thumb = e.thumbnail.clone().or_else(|| last_thumb(&e.thumbnails));
                let entry_url = e.webpage_url.or(e.url)?;
                Some(PlaylistEntry {
                    id: e.id.unwrap_or_else(|| i.to_string()),
                    title: e.title.unwrap_or_else(|| format!("Video {}", i + 1)),
                    url: entry_url,
                    duration: e.duration,
                    thumbnail: thumb,
                })
            })
            .collect();
        let total = entries.iter().filter_map(|e| e.duration).sum::<f64>();
        return Ok(MediaInfo {
            is_playlist: true,
            thumbnail: thumbnail.or_else(|| entries.iter().find_map(|e| e.thumbnail.clone())),
            url,
            platform,
            title,
            uploader,
            duration: (total > 0.0).then_some(total),
            video_options: standard_video_options(),
            audio_options: audio_options(None),
            wav_size: None,
            audio_source_kbps: None,
            audio_bytes: None,
            has_subtitles: true,
            entries,
            parent_playlist_url: None,
        });
    }

    let formats = raw.formats.unwrap_or_default();
    let has_subtitles = [&raw.subtitles, &raw.automatic_captions]
        .iter()
        .any(|subs| {
            subs.as_ref()
                .is_some_and(|m| m.keys().any(|k| k != "live_chat"))
        });
    let audio = best_audio(&formats, false);

    Ok(MediaInfo {
        is_playlist: false,
        parent_playlist_url: parent_playlist(requested_url),
        video_options: video_options(&formats, raw.duration),
        audio_options: audio_options(raw.duration),
        wav_size: raw.duration.map(|d| (d * WAV_BYTES_PER_SEC) as u64),
        audio_source_kbps: audio
            .and_then(|f| f.abr)
            .filter(|abr| *abr > 0.0)
            .map(|abr| abr.round() as u32),
        audio_bytes: audio.and_then(|f| f.size(raw.duration)),
        has_subtitles,
        duration: raw.duration,
        entries: Vec::new(),
        url,
        platform,
        title,
        uploader,
        thumbnail,
    })
}

/// Formats are not fetched per video in a playlist; quality is applied as an upper limit.
fn standard_video_options() -> Vec<VideoOption> {
    STANDARD
        .iter()
        .rev()
        .map(|(short, long, label, short_label)| VideoOption {
            height: *short,
            width: Some(*long),
            label: label.to_string(),
            short_label: short_label.to_string(),
            fps: None,
            size_mp4: None,
            size_mkv: None,
            video_bytes_mp4: None,
            video_bytes_mkv: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: Value) -> MediaInfo {
        parse_media_info(&value.to_string(), "https://example.com/v").unwrap()
    }

    #[test]
    fn vertical_video_is_labelled_by_its_short_edge() {
        let info = parse(json!({
            "extractor_key": "TikTok", "title": "t", "duration": 11.0,
            "formats": [
                {"width": 576, "height": 1024, "vcodec": "h264", "acodec": "aac", "filesize": 1000},
                {"width": 720, "height": 1280, "vcodec": "h265", "acodec": "aac", "filesize": 2000}
            ]
        }));
        assert_eq!(info.platform, "TikTok");
        let labels: Vec<_> = info
            .video_options
            .iter()
            .map(|o| o.label.as_str())
            .collect();
        assert_eq!(labels, ["720p HD", "576p"]);
        // For vertical video the selector must use the real height.
        assert_eq!(info.video_options[0].height, 1280);
        assert_eq!(info.video_options[0].size_mp4, Some(2000));
    }

    #[test]
    fn labels_sizes_and_codec_preference() {
        let info = parse(json!({
            "extractor_key": "Youtube", "title": "t", "duration": 100.0,
            "formats": [
                {"acodec": "opus", "vcodec": "none", "abr": 160.0, "filesize": 2000},
                {"acodec": "mp4a.40.2", "vcodec": "none", "abr": 128.0, "filesize": 1600},
                {"width": 1920, "height": 1080, "vcodec": "avc1.64", "acodec": "none", "fps": 30.0, "filesize": 10000},
                {"width": 1920, "height": 1080, "vcodec": "vp9", "acodec": "none", "fps": 30.0, "filesize": 7000},
                {"width": 3840, "height": 2160, "vcodec": "vp9", "acodec": "none", "fps": 30.0, "tbr": 8.0},
                {"width": 3840, "height": 2160, "vcodec": "av01.0.13M.08", "acodec": "none", "fps": 30.0, "tbr": 4.0}
            ]
        }));
        let o4k = &info.video_options[0];
        assert_eq!((o4k.label.as_str(), o4k.short_label.as_str()), ("4K", "4K"));
        // Unknown size falls back to tbr × duration: 8 kbps × 100 s = 100 kB + AAC.
        assert_eq!(o4k.size_mp4, Some(100_000 + 1600));
        let o1080 = &info.video_options[1];
        assert_eq!(o1080.label, "1080p Full HD");
        assert_eq!(o1080.size_mp4, Some(10000 + 1600));
        assert_eq!(o1080.size_mkv, Some(7000 + 2000));
        assert_eq!(o1080.video_bytes_mp4, Some(10000));
        assert_eq!(info.audio_source_kbps, Some(160));
        assert_eq!(info.audio_options[0].size, Some(4_000_000));
        assert_eq!(info.wav_size, Some(19_200_000));
    }

    #[test]
    fn letterboxed_video_keeps_standard_label() {
        assert_eq!(bucket(Some(1920), 1036).2, "1080p");
        assert_eq!(bucket(Some(1080), 1920).2, "1080p");
        assert_eq!(bucket(None, 720).2, "720p");
    }

    #[test]
    fn playlist_lists_entries_and_skips_nulls() {
        let info = parse(json!({
            "_type": "playlist", "extractor_key": "YoutubeTab", "title": "Liste",
            "entries": [
                {"id": "a", "url": "https://y/a", "title": "A", "duration": 5.0, "thumbnails": [{"url": "s"}, {"url": "b"}]},
                null,
                {"id": "c", "title": "URL'siz"},
                {"id": "d", "webpage_url": "https://y/d"},
                {"id": "n", "url": "https://y/playlist?list=1", "ie_key": "YoutubeTab"}
            ]
        }));
        assert!(info.is_playlist);
        assert_eq!(info.platform, "YouTube");
        assert_eq!(info.entries.len(), 2);
        assert_eq!(info.entries[0].thumbnail.as_deref(), Some("b"));
        assert_eq!(info.thumbnail.as_deref(), Some("b"));
        assert_eq!(info.video_options[0].label, "8K");
    }

    #[test]
    fn detects_parent_playlist() {
        assert_eq!(
            parent_playlist("https://www.youtube.com/watch?v=x&list=PL1").as_deref(),
            Some("https://www.youtube.com/playlist?list=PL1")
        );
        assert_eq!(parent_playlist("https://www.youtube.com/watch?v=x"), None);
        assert_eq!(
            parent_playlist("https://www.youtube.com/playlist?list=PL1"),
            None
        );
        assert_eq!(
            parent_playlist("https://example.com/watch?v=x&list=PL1"),
            None
        );
    }
}
