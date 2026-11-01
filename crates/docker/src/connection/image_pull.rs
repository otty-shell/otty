//! Pulling an image: the engine's pull stream, read into
//! [`PullProgress`] snapshots.

use bollard::query_parameters::CreateImageOptions;
use futures_util::StreamExt;
use futures_util::future::ready;
use futures_util::stream::BoxStream;

use super::{Engine, EngineError};
use crate::pull::PullProgress;

impl Engine {
    /// Pull `image` at `tag` (for example `nginx` and `1.27-alpine`). The
    /// stream gives a [`PullProgress`] after every line the engine sends
    /// and ends when the pull is done. An error (for example "manifest
    /// unknown") comes as an `Err` item, after which the engine stops.
    /// Dropping the stream closes the request, which stops the pull.
    pub fn pull(
        &self,
        image: &str,
        tag: &str,
    ) -> BoxStream<'static, Result<PullProgress, EngineError>> {
        let options = CreateImageOptions {
            from_image: Some(image.to_string()),
            tag: Some(tag.to_string()),
            ..CreateImageOptions::default()
        };

        self.client
            .create_image(Some(options), None, None)
            .scan(PullProgress::default(), |progress, line| {
                let update = match line {
                    Ok(info) => {
                        progress.apply(&info);
                        Ok(progress.clone())
                    },
                    Err(error) => Err(EngineError::from(error)),
                };
                ready(Some(update))
            })
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;

    use super::*;
    use crate::test_support::FakeEngine;

    const FIXTURE: &str =
        include_str!("../../tests/fixtures/docker-pull.jsonl");

    #[tokio::test]
    async fn a_pull_asks_for_the_image_and_tag_and_reports_each_step() {
        let fake = FakeEngine::start(vec![("200 OK", FIXTURE.to_string())]);
        let engine = Engine::connect(fake.info()).expect("engine");

        let updates: Vec<Result<PullProgress, EngineError>> =
            engine.pull("nginx", "1.27-alpine").collect().await;

        assert_eq!(updates.len(), FIXTURE.lines().count());
        let last = updates
            .last()
            .and_then(|update| update.as_ref().ok())
            .expect("a last snapshot");
        assert_eq!(last.layer_count(), 3);
        assert_eq!(last.downloaded(), 15_000_000);
        let requests = fake.requests();
        assert!(
            requests[0].line.starts_with("POST "),
            "{}",
            requests[0].line
        );
        assert!(requests[0].line.contains("/images/create"));
        assert!(requests[0].line.contains("fromImage=nginx"));
        assert!(requests[0].line.contains("tag=1.27-alpine"));
    }

    #[tokio::test]
    async fn an_error_in_the_stream_carries_the_engines_message() {
        let fake = FakeEngine::start(vec![(
            "200 OK",
            String::from(concat!(
                r#"{"status":"Pulling from library/nope","id":"latest"}"#,
                "\n",
                r#"{"errorDetail":{"message":"manifest unknown"},"error":"manifest unknown"}"#,
                "\n"
            )),
        )]);
        let engine = Engine::connect(fake.info()).expect("engine");

        let updates: Vec<Result<PullProgress, EngineError>> =
            engine.pull("nope", "latest").collect().await;

        let error = updates
            .iter()
            .find_map(|update| update.as_ref().err())
            .expect("an error");
        assert_eq!(error.message(), "manifest unknown");
        fake.requests();
    }
}
