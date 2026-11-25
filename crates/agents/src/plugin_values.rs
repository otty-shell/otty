//! The values of the plugin contracts turned into the values of this crate
//! and back: a profile as a part is asked about it, and the tokens,
//! times, providers and limits a part answers.

use std::time::{Duration, SystemTime};

use otty_plugin_host::agents::agent::{
    Profile as PartProfile, ProviderRef as PartProviderRef,
    TokenCount as PartTokenCount,
};
use otty_plugin_host::agents::provider::{
    AccountNumbers as PartNumbers, Credits as PartCredits,
};

use crate::limits::{
    Billing, Credits, LimitWindow, Limits, UsageWindows, WindowId,
};
use crate::profile::Profile;
use crate::provider::ProviderRef;
use crate::timestamp::time_from_millis;
use crate::token_count::TokenCount;

/// `profile` as an agent part is asked about it: the agent's id, the
/// folder, and whether it is the agent's default folder.
pub(crate) fn part_profile(profile: &Profile) -> PartProfile {
    PartProfile {
        agent: profile.agent().as_str().to_string(),
        folder: profile.config_dir().display().to_string(),
        is_default: profile.is_in_default_config_dir(),
    }
}

/// The time `millis` milliseconds after the Unix epoch; the epoch itself
/// for a time before it.
pub(crate) fn time_of(millis: i64) -> SystemTime {
    time_from_millis(millis as f64).unwrap_or(SystemTime::UNIX_EPOCH)
}

/// The tokens of a part's answer. The part's reasoning tokens are a share
/// of its output, which OTTY counts already.
pub(crate) fn tokens_of(tokens: &PartTokenCount) -> TokenCount {
    TokenCount::new(tokens.input, tokens.cached, tokens.output)
        .with_cache_written(tokens.cache_written)
}

/// The provider a part names: by id, or by the host of a key's server,
/// in small letters.
pub(crate) fn provider_of(provider: &PartProviderRef) -> ProviderRef {
    match provider {
        PartProviderRef::Id(id) => ProviderRef::of_id(id.as_str()),
        PartProviderRef::Host(host) => {
            ProviderRef::Host(host.trim().to_ascii_lowercase())
        },
    }
}

/// The limits in the numbers a provider part read: its usage windows,
/// else its credits; a plan and the time the numbers were true, when the
/// part tells.
pub(crate) fn limits_of(limits: &PartNumbers) -> Limits {
    let windows: Vec<LimitWindow> = limits
        .windows
        .iter()
        .map(|window| {
            let length = window
                .length_minutes
                .map(|minutes| Duration::from_secs(u64::from(minutes) * 60));
            let mut id = WindowId::new(length);
            if let Some(model) = &window.model {
                id = id.for_model(model.as_str());
            }
            let resets_at = window.resets_at_ms.map(time_of);
            let limit = LimitWindow::new(id, window.percent_used, resets_at);

            match &window.reset_text {
                Some(text) => limit.with_reset_text(text.as_str()),
                None => limit,
            }
        })
        .collect();

    let billing = match (&limits.credits, windows.is_empty()) {
        (Some(credits), true) => Billing::Credits(credits_of(credits)),
        _ => Billing::UsageWindows(UsageWindows::new(windows, None)),
    };

    let read = Limits::new(billing, limits.plan.clone());
    match limits.recorded_at_ms {
        Some(millis) => read.with_recorded_at(time_of(millis)),
        None => read,
    }
}

/// The credits a part read.
pub(crate) fn credits_of(credits: &PartCredits) -> Credits {
    if credits.is_unlimited {
        return Credits::unlimited();
    }

    Credits::new(
        credits.left,
        credits.used,
        credits.pool,
        credits.renews_at_ms.map(time_of),
    )
}

#[cfg(test)]
mod tests {
    use otty_plugin_host::agents::provider::LimitWindow as PartWindow;

    use super::*;

    fn part_limits(
        windows: Vec<PartWindow>,
        credits: Option<PartCredits>,
    ) -> PartNumbers {
        PartNumbers {
            windows,
            credits,
            extra_usage: None,
            left: None,
            key_limit: None,
            spent_this_month: None,
            plan: Some("Pro".to_string()),
            recorded_at_ms: None,
        }
    }

    #[test]
    fn usage_windows_keep_their_length_model_and_reset() {
        let window = PartWindow {
            length_minutes: Some(300),
            model: Some("Opus".to_string()),
            percent_used: 62.0,
            resets_at_ms: Some(1_000),
            reset_text: None,
        };

        let limits = limits_of(&part_limits(vec![window], None));

        let Billing::UsageWindows(usage) = limits.billing() else {
            panic!("usage windows: {limits:?}");
        };
        let window = &usage.windows()[0];
        assert_eq!(window.id(), &WindowId::hours(5).for_model("Opus"));
        assert_eq!(window.resets_at(), Some(time_of(1_000)));
        assert_eq!(limits.plan(), Some("Pro"));
    }

    #[test]
    fn credits_with_no_window_are_credits() {
        let credits = PartCredits {
            left: Some(12.0),
            used: None,
            pool: Some(20.0),
            renews_at_ms: None,
            is_unlimited: false,
        };

        let limits = limits_of(&part_limits(Vec::new(), Some(credits)));

        assert_eq!(
            limits.billing(),
            &Billing::Credits(Credits::new(Some(12.0), None, Some(20.0), None))
        );
    }

    #[test]
    fn a_host_is_kept_in_small_letters_and_an_id_as_it_is() {
        let host = PartProviderRef::Host(" API.Tealcloud.example ".to_string());
        let id = PartProviderRef::Id("openai".to_string());

        assert_eq!(
            provider_of(&host),
            ProviderRef::Host("api.tealcloud.example".to_string())
        );
        assert_eq!(provider_of(&id), ProviderRef::of_id("openai"));
    }
}
