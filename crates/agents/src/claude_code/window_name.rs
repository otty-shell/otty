//! The names of Claude's usage windows, the same for every source: "5
//! hours", "7 days" and "7 days · Opus" for the week of one model.

/// The name of the five-hour window.
pub(super) const FIVE_HOURS: &str = "5 hours";
/// The name of the weekly window of all models.
pub(super) const WEEK: &str = "7 days";

/// The name of the weekly window of one model, such as "7 days · Fable"
/// for `fable`: the model with its first letter in upper case.
pub(super) fn model_week(model: &str) -> String {
    let mut letters = model.trim().chars();
    let model = match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::new(),
    };

    format!("{WEEK} · {model}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_week_is_named_after_the_model_with_a_capital_letter() {
        assert_eq!(model_week("opus"), "7 days · Opus");
        assert_eq!(model_week(" Fable "), "7 days · Fable");
        assert_eq!(model_week("Example Model"), "7 days · Example Model");
    }
}
