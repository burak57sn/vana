use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoContainer {
    Mp4,
    Mkv,
}

impl VideoContainer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    M4a,
    Wav,
}

impl AudioFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Wav => "wav",
        }
    }
}

/// Download mode chosen by the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum Selection {
    #[serde(rename_all = "camelCase")]
    Video {
        container: VideoContainer,
        /// Upper bound of the chosen resolution. `None` = best quality.
        height: Option<u32>,
        subtitles: bool,
    },
    #[serde(rename_all = "camelCase")]
    Audio {
        format: AudioFormat,
        /// WAV is lossless, so it takes no bitrate.
        kbps: Option<u32>,
    },
}

impl Selection {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Video { container, .. } => container.as_str(),
            Self::Audio { format, .. } => format.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub url: String,
    pub selection: Selection,
    /// Short quality label written into the output name (`1080p`, `320kbps`).
    pub quality_label: String,
    /// Estimated sizes of separate video/audio streams, used to weight progress.
    #[serde(default)]
    pub est_video_bytes: Option<u64>,
    #[serde(default)]
    pub est_audio_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Downloading,
    Merging,
    Converting,
    Embedding,
    Finishing,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressUpdate {
    pub percent: f64,
    pub speed_bps: Option<f64>,
    pub eta_secs: Option<u64>,
    pub stage: Stage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoOption {
    /// Real height for the `[height<=?N]` selector (the long edge for vertical video).
    pub height: u32,
    pub width: Option<u32>,
    /// Menu label: `1080p Full HD`, `4K`.
    pub label: String,
    /// For list labels and file names: `1080p`, `4K`.
    pub short_label: String,
    pub fps: Option<f64>,
    pub size_mp4: Option<u64>,
    pub size_mkv: Option<u64>,
    /// Size of the separate video stream alone (for progress weighting).
    pub video_bytes_mp4: Option<u64>,
    pub video_bytes_mkv: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOption {
    pub kbps: u32,
    pub label: String,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub is_playlist: bool,
    pub url: String,
    pub platform: String,
    pub title: String,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub video_options: Vec<VideoOption>,
    pub audio_options: Vec<AudioOption>,
    pub wav_size: Option<u64>,
    /// Real bitrate of the best source audio; converting cannot add quality above it.
    pub audio_source_kbps: Option<u32>,
    pub audio_bytes: Option<u64>,
    pub has_subtitles: bool,
    pub entries: Vec<PlaylistEntry>,
    /// For links like `watch?v=…&list=…`, to fetch the whole playlist.
    pub parent_playlist_url: Option<String>,
}
