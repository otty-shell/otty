//! The read of the list prices of the money estimate (spec Decision 16):
//! the LiteLLM price table, fetched at most once a day, or its copy kept in
//! OTTY's cache folder. The fetch is a request over the network, so it runs
//! on a thread of its own.

use std::path::Path;
use std::time::SystemTime;

use iced::Task;
use otty_agents::PriceSource;
use otty_app_agents::{AgentsIntent, LoadedPrices};

use crate::load::on_thread;

/// The folder of the kept price table in OTTY's cache folder.
const PRICES_FOLDER: &str = "prices";

/// Read the list prices in the background, keeping the price table in
/// `prices` under `cache_dir` (`~/.cache/otty/prices`). The table is
/// fetched when the kept copy is a day old or missing, or with
/// `fetch_again`; the request sends nothing about the user. The answer
/// comes back as [`AgentsIntent::PricesLoaded`].
///
/// # Example
///
/// ```
/// use std::path::Path;
///
/// let read = otty_app_agents_io::load_prices(
///     Path::new("/home/user/.cache/otty"),
///     false,
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_prices(cache_dir: &Path, fetch_again: bool) -> Task<AgentsIntent> {
    let kept_dir = cache_dir.join(PRICES_FOLDER);

    Task::perform(
        on_thread(move || read_prices(&kept_dir, fetch_again)),
        |loaded| {
            let loaded = loaded.unwrap_or_else(|| {
                log::warn!("reading the list prices stopped");
                LoadedPrices {
                    prices: Err(Vec::new()),
                    read_at: SystemTime::now(),
                }
            });
            AgentsIntent::PricesLoaded(loaded)
        },
    )
}

/// The prices from the LiteLLM price table or its copy kept in
/// `kept_dir`.
fn read_prices(kept_dir: &Path, fetch_again: bool) -> LoadedPrices {
    let source = PriceSource::new(kept_dir);

    LoadedPrices {
        prices: source.read(SystemTime::now(), fetch_again),
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use otty_agents::SourceId;
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::load::tests::run;

    /// A made-up price file with one model.
    const PRICES: &str = r#"{"gpt-5": {"input_cost_per_token": 1.25e-06, "output_cost_per_token": 1e-05, "litellm_provider": "openai"}}"#;

    /// A kept copy written just now is used: nothing is fetched, so the
    /// test never reaches the network.
    #[test]
    fn the_read_runs_in_the_background_and_uses_the_kept_copy_of_today() {
        let cache = TempDir::new("agents-prices-kept");
        let kept_dir = cache.path().join(PRICES_FOLDER);
        let kept_file = PriceSource::new(&kept_dir).kept_file();
        std::fs::create_dir_all(&kept_dir).expect("the prices folder");
        std::fs::write(&kept_file, PRICES).expect("the kept copy");

        let answers = run(load_prices(cache.path(), false));

        assert!(
            matches!(
                answers.as_slice(),
                [AgentsIntent::PricesLoaded(LoadedPrices { prices: Ok(answer), .. })]
                    if answer.source() == SourceId::LiteLlmPrices
                        && answer.value().table().price_of("gpt-5").is_some()
            ),
            "{answers:?}"
        );
    }
}
