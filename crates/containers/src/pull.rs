//! How far an image pull is, added up over its layers from the engine's
//! pull stream.

#[cfg(feature = "client")]
use bollard::models::CreateImageInfo;

/// The layer status the engine sends, with byte counts, while it downloads
/// a layer.
#[cfg(feature = "client")]
const DOWNLOADING: &str = "Downloading";
/// Layer statuses after which the whole layer is downloaded.
#[cfg(feature = "client")]
const DOWNLOADED: [&str; 3] =
    ["Verifying Checksum", "Download complete", "Extracting"];
/// Layer statuses after which the layer is on this computer.
#[cfg(feature = "client")]
const FINISHED: [&str; 2] = ["Pull complete", "Already exists"];
/// Layer statuses before the download starts.
#[cfg(feature = "client")]
const WAITING: [&str; 2] = ["Pulling fs layer", "Waiting"];

/// One layer of a pull.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Layer {
    id: String,
    downloaded: u64,
    total: u64,
    is_finished: bool,
}

/// How far an image pull is: its layers, the bytes downloaded of the bytes
/// known so far, and how many layers are on this computer.
///
/// The engine sends one line per step of each layer ("Pulling fs layer",
/// "Downloading" with byte counts, "Download complete", "Pull complete",
/// "Already exists"); lines about the whole image (`Pulling from`,
/// `Digest:`, `Status:`) are not layers. [`crate::Engine::pull`] gives a
/// new snapshot after every line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullProgress {
    layers: Vec<Layer>,
}

impl PullProgress {
    /// How many layers the image has, as far as the engine said.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// How many layers are on this computer (pulled or already there).
    pub fn finished_layers(&self) -> usize {
        self.layers.iter().filter(|layer| layer.is_finished).count()
    }

    /// Bytes downloaded so far.
    pub fn downloaded(&self) -> u64 {
        self.layers.iter().map(|layer| layer.downloaded).sum()
    }

    /// Bytes to download, over the layers whose size is known yet.
    pub fn total(&self) -> u64 {
        self.layers.iter().map(|layer| layer.total).sum()
    }

    /// How far the pull is, from 0 to 1: by bytes once sizes are known,
    /// else by finished layers.
    pub fn fraction(&self) -> f32 {
        let total = self.total();
        if total > 0 {
            return (self.downloaded() as f64 / total as f64).min(1.0) as f32;
        }

        if self.layers.is_empty() {
            return 0.0;
        }
        self.finished_layers() as f32 / self.layers.len() as f32
    }
}

#[cfg(feature = "client")]
impl PullProgress {
    /// Take one line of the pull stream.
    pub(crate) fn apply(&mut self, info: &CreateImageInfo) {
        let (Some(id), Some(status)) =
            (info.id.as_deref(), info.status.as_deref())
        else {
            return;
        };
        let is_layer_status = status == DOWNLOADING
            || DOWNLOADED.contains(&status)
            || FINISHED.contains(&status)
            || WAITING.contains(&status);
        if !is_layer_status {
            return;
        }

        let layer = self.layer(id);
        if status == DOWNLOADING {
            let detail = info.progress_detail.as_ref();
            let bytes = |value: Option<i64>| {
                value.and_then(|value| u64::try_from(value).ok())
            };
            if let Some(total) = bytes(detail.and_then(|detail| detail.total)) {
                layer.total = total;
            }
            if let Some(current) =
                bytes(detail.and_then(|detail| detail.current))
            {
                layer.downloaded = current;
            }
        } else if DOWNLOADED.contains(&status) {
            layer.downloaded = layer.total;
        } else if FINISHED.contains(&status) {
            layer.downloaded = layer.total;
            layer.is_finished = true;
        }
    }

    /// The layer `id`, added at the end the first time it shows up.
    fn layer(&mut self, id: &str) -> &mut Layer {
        let index = match self.layers.iter().position(|layer| layer.id == id) {
            Some(index) => index,
            None => {
                self.layers.push(Layer {
                    id: id.to_string(),
                    downloaded: 0,
                    total: 0,
                    is_finished: false,
                });
                self.layers.len() - 1
            },
        };

        &mut self.layers[index]
    }
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use bollard::models::CreateImageInfo;

    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/docker-pull.jsonl");

    fn events(lines: &str) -> Vec<CreateImageInfo> {
        lines
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("pull event"))
            .collect()
    }

    fn progress_after(lines: &str) -> PullProgress {
        let mut progress = PullProgress::default();
        for event in events(lines) {
            progress.apply(&event);
        }
        progress
    }

    #[test]
    fn a_pull_in_progress_adds_up_its_layers_and_bytes() {
        let progress = progress_after(FIXTURE);

        assert_eq!(progress.layer_count(), 3);
        assert_eq!(progress.finished_layers(), 1);
        assert_eq!(progress.downloaded(), 15_000_000);
        assert_eq!(progress.total(), 47_000_000);
        let fraction = progress.fraction();
        assert!((fraction - 15.0 / 47.0).abs() < 1e-6, "{fraction}");
    }

    #[test]
    fn a_finished_pull_has_every_layer_done_and_every_byte() {
        let finished = format!(
            "{FIXTURE}{}",
            r#"
{"status":"Download complete","progressDetail":{},"id":"a1b2c3d4e5f6"}
{"status":"Extracting","progressDetail":{"current":1000,"total":20000000},"id":"a1b2c3d4e5f6"}
{"status":"Pull complete","progressDetail":{},"id":"a1b2c3d4e5f6"}
{"status":"Verifying Checksum","progressDetail":{},"id":"b2c3d4e5f6a1"}
{"status":"Download complete","progressDetail":{},"id":"b2c3d4e5f6a1"}
{"status":"Pull complete","progressDetail":{},"id":"b2c3d4e5f6a1"}
{"status":"Digest: sha256:0123456789abcdef"}
{"status":"Status: Downloaded newer image for nginx:1.27-alpine"}
"#
        );

        let progress = progress_after(&finished);

        assert_eq!(progress.layer_count(), 3);
        assert_eq!(progress.finished_layers(), 3);
        assert_eq!(progress.downloaded(), 47_000_000);
        assert_eq!(progress.total(), 47_000_000);
        assert!((progress.fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn extracting_counts_the_layer_as_downloaded() {
        let progress = progress_after(
            r#"
{"status":"Downloading","progressDetail":{"current":10,"total":100},"id":"l1"}
{"status":"Extracting","progressDetail":{"current":5,"total":100},"id":"l1"}
"#,
        );

        assert_eq!(progress.downloaded(), 100);
        assert_eq!(progress.finished_layers(), 0);
    }

    #[test]
    fn lines_about_the_whole_image_are_not_layers() {
        let progress = progress_after(
            r#"
{"status":"Pulling from library/nginx","id":"latest"}
{"status":"Digest: sha256:0123456789abcdef"}
{"status":"Status: Image is up to date for nginx:latest"}
"#,
        );

        assert_eq!(progress, PullProgress::default());
        assert_eq!(progress.layer_count(), 0);
        assert_eq!(progress.fraction(), 0.0);
    }

    #[test]
    fn without_sizes_the_fraction_counts_finished_layers() {
        let progress = progress_after(
            r#"
{"status":"Already exists","progressDetail":{},"id":"l1"}
{"status":"Pulling fs layer","progressDetail":{},"id":"l2"}
"#,
        );

        assert_eq!(progress.total(), 0);
        assert!((progress.fraction() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_layer_seen_again_keeps_its_place() {
        let progress = progress_after(
            r#"
{"status":"Pulling fs layer","progressDetail":{},"id":"l1"}
{"status":"Pulling fs layer","progressDetail":{},"id":"l1"}
{"status":"Downloading","progressDetail":{"current":1,"total":4},"id":"l1"}
"#,
        );

        assert_eq!(progress.layer_count(), 1);
        assert_eq!(progress.downloaded(), 1);
        assert_eq!(progress.total(), 4);
    }
}
