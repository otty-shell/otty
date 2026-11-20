//! The name of a Claude plan, as Claude Code shows it.

/// The plan as Claude Code names it, such as "Pro" or "Max 20x".
///
/// The plan is the subscription type, such as `max`. The tier, such as
/// `default_claude_max_20x`, adds how much more the plan allows. Both come
/// from `<config>/.credentials.json`.
pub(crate) fn plan_name(
    subscription_type: Option<&str>,
    rate_limit_tier: Option<&str>,
) -> Option<String> {
    let subscription = subscription_type?.trim();
    let mut letters = subscription.chars();
    let first = letters.next()?;
    let name = format!("{}{}", first.to_uppercase(), letters.as_str());

    match rate_limit_tier.and_then(tier_multiplier) {
        Some(multiplier) => Some(format!("{name} {multiplier}")),
        None => Some(name),
    }
}

/// The multiplier at the end of a rate limit tier, such as `20x` in
/// `default_claude_max_20x`.
fn tier_multiplier(tier: &str) -> Option<&str> {
    let last_part = tier.rsplit('_').next()?;
    let digits = last_part.strip_suffix('x')?;

    let is_number =
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());

    is_number.then_some(last_part)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_reads_like_the_name_claude_code_shows() {
        let cases = [
            (Some("pro"), Some("default_claude_pro"), Some("Pro")),
            (Some("max"), Some("default_claude_max_5x"), Some("Max 5x")),
            (Some("max"), Some("default_claude_max_20x"), Some("Max 20x")),
            (Some("max"), None, Some("Max")),
            (Some("team"), Some("default_raven"), Some("Team")),
            (Some(" "), Some("default_claude_max_5x"), None),
            (None, Some("default_claude_max_5x"), None),
        ];

        for (subscription, tier, plan) in cases {
            assert_eq!(
                plan_name(subscription, tier).as_deref(),
                plan,
                "{subscription:?} {tier:?}"
            );
        }
    }
}
