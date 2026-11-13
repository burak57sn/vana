//! Turns yt-dlp output into progress updates.

use super::model::{ProgressUpdate, Stage};

pub const PROGRESS_PREFIX: &str = "VANA_P|";
pub const POSTPROCESS_PREFIX: &str = "VANA_PP|";
pub const PATH_PREFIX: &str = "VANA_FILE|";

pub const PROGRESS_TEMPLATE: &str = "VANA_P|%(info.format_id)s|%(info.vcodec)s|%(info.acodec)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s";
pub const POSTPROCESS_TEMPLATE: &str = "VANA_PP|%(progress.status)s|%(progress.postprocessor)s";

/// Default share of the separate video stream when its weight is unknown.
const DEFAULT_VIDEO_SHARE: f64 = 0.85;
/// The bar does not pass this value until post-processing finishes.
const MAX_BEFORE_DONE: f64 = 99.0;

pub enum OutputLine {
    Progress(ProgressUpdate),
    FinalPath(String),
    Other,
}

/// Combines several streams (video + audio) into a single progress bar.
pub struct ProgressTracker {
    video_share: f64,
    current_format: Option<String>,
    completed: f64,
    current_weight: f64,
    last_percent: f64,
    stage: Stage,
}

impl ProgressTracker {
    pub fn new(est_video: Option<u64>, est_audio: Option<u64>) -> Self {
        let video_share = match (est_video, est_audio) {
            (Some(v), Some(a)) if v + a > 0 => v as f64 / (v + a) as f64,
            _ => DEFAULT_VIDEO_SHARE,
        };
        Self {
            video_share,
            current_format: None,
            completed: 0.0,
            current_weight: 1.0,
            last_percent: 0.0,
            stage: Stage::Downloading,
        }
    }

    pub fn parse_line(&mut self, line: &str) -> OutputLine {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(PATH_PREFIX) {
            return OutputLine::FinalPath(rest.to_string());
        }
        if let Some(rest) = line.strip_prefix(POSTPROCESS_PREFIX) {
            return self.parse_postprocess(rest);
        }
        match line.strip_prefix(PROGRESS_PREFIX) {
            Some(rest) => self.parse_progress(rest),
            None => OutputLine::Other,
        }
    }

    fn parse_progress(&mut self, rest: &str) -> OutputLine {
        let f: Vec<&str> = rest.split('|').collect();
        let [format_id, vcodec, acodec, downloaded, total, estimate, speed, eta] = f[..] else {
            return OutputLine::Other;
        };

        if self.current_format.as_deref() != Some(format_id) {
            if self.current_format.is_some() {
                self.completed = (self.completed + self.current_weight).min(1.0);
            }
            self.current_format = Some(format_id.to_string());
            self.current_weight = if vcodec != "none" && acodec == "none" {
                self.video_share
            } else if vcodec == "none" && self.completed > 0.0 {
                (1.0 - self.completed).max(0.0)
            } else {
                1.0
            };
        }

        let downloaded = num(downloaded).unwrap_or(0.0);
        let total = num(total).or_else(|| num(estimate)).filter(|t| *t > 0.0);
        let fraction = total.map_or(0.0, |t| (downloaded / t).clamp(0.0, 1.0));
        let percent = ((self.completed + self.current_weight * fraction) * 100.0)
            .clamp(self.last_percent, MAX_BEFORE_DONE);
        self.last_percent = percent;

        OutputLine::Progress(ProgressUpdate {
            percent,
            speed_bps: num(speed).filter(|s| *s > 0.0),
            eta_secs: num(eta).filter(|e| *e >= 0.0).map(|e| e as u64),
            stage: self.stage,
        })
    }

    fn parse_postprocess(&mut self, rest: &str) -> OutputLine {
        let Some((status, processor)) = rest.split_once('|') else {
            return OutputLine::Other;
        };
        if status != "started" {
            return OutputLine::Other;
        }
        self.stage = match processor {
            "Merger" => Stage::Merging,
            "ExtractAudio" | "VideoRemuxer" | "VideoConvertor" => Stage::Converting,
            "EmbedThumbnail" | "EmbedSubtitle" | "Metadata" | "FFmpegMetadata" | "Fixup" => {
                Stage::Embedding
            }
            name if name.starts_with("FixupM") => Stage::Embedding,
            _ => Stage::Finishing,
        };
        self.last_percent = MAX_BEFORE_DONE;
        OutputLine::Progress(ProgressUpdate {
            percent: MAX_BEFORE_DONE,
            speed_bps: None,
            eta_secs: None,
            stage: self.stage,
        })
    }
}

fn num(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pct(t: &mut ProgressTracker, line: &str) -> f64 {
        match t.parse_line(line) {
            OutputLine::Progress(p) => p.percent,
            _ => panic!("ilerleme bekleniyordu: {line}"),
        }
    }

    #[test]
    fn merges_video_and_audio_streams_into_one_bar() {
        let mut t = ProgressTracker::new(Some(900), Some(100));
        assert_eq!(
            pct(&mut t, "VANA_P|137|avc1|none|450|900|NA|1000.5|3"),
            45.0
        );
        assert_eq!(pct(&mut t, "VANA_P|137|avc1|none|900|900|NA|1000|0"), 90.0);
        assert_eq!(pct(&mut t, "VANA_P|140|none|mp4a|50|100|NA|500|1"), 95.0);
        assert_eq!(pct(&mut t, "VANA_P|140|none|mp4a|100|100|NA|500|0"), 99.0);
    }

    #[test]
    fn single_progressive_stream_uses_full_range() {
        let mut t = ProgressTracker::new(None, None);
        assert_eq!(pct(&mut t, "VANA_P|18|avc1|mp4a|25|100|NA|NA|NA"), 25.0);
    }

    #[test]
    fn uses_estimate_and_tolerates_missing_values() {
        let mut t = ProgressTracker::new(None, None);
        assert_eq!(pct(&mut t, "VANA_P|hls|avc1|mp4a|10|NA|40.0|NA|NA"), 25.0);
        let OutputLine::Progress(p) = t.parse_line("VANA_P|hls|avc1|mp4a|20|NA|40.0|2048.0|7")
        else {
            panic!()
        };
        assert_eq!(p.speed_bps, Some(2048.0));
        assert_eq!(p.eta_secs, Some(7));
    }

    #[test]
    fn progress_never_moves_backwards() {
        let mut t = ProgressTracker::new(None, None);
        assert_eq!(pct(&mut t, "VANA_P|hls|avc1|mp4a|50|100|NA|NA|NA"), 50.0);
        // The estimate can grow during fragmented downloads.
        assert_eq!(pct(&mut t, "VANA_P|hls|avc1|mp4a|55|200|NA|NA|NA"), 50.0);
    }

    #[test]
    fn postprocess_and_final_path() {
        let mut t = ProgressTracker::new(None, None);
        let OutputLine::Progress(p) = t.parse_line("VANA_PP|started|Merger") else {
            panic!()
        };
        assert_eq!(p.stage, Stage::Merging);
        assert!(matches!(
            t.parse_line("VANA_PP|finished|Merger"),
            OutputLine::Other
        ));
        let OutputLine::FinalPath(path) = t.parse_line("VANA_FILE|/d/Şarkı [320kbps].mp3") else {
            panic!()
        };
        assert_eq!(path, "/d/Şarkı [320kbps].mp3");
        assert!(matches!(
            t.parse_line("[download] something"),
            OutputLine::Other
        ));
    }
}
