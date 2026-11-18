//! Claude's usage windows, the same for every source: the five-hour
//! window, the weekly one, and the week of one model, such as Opus.

use crate::limits::WindowId;

/// The five-hour window.
pub(super) fn five_hours() -> WindowId {
    WindowId::hours(5)
}

/// The weekly window of all models.
pub(super) fn week() -> WindowId {
    WindowId::days(7)
}

/// The weekly window of one model, such as the week of "Fable" for
/// `fable`: the model with its first letter in upper case.
pub(super) fn model_week(model: &str) -> WindowId {
    let mut letters = model.trim().chars();
    let model: String = match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::new(),
    };

    week().for_model(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_week_is_named_after_the_model_with_a_capital_letter() {
        assert_eq!(model_week("opus"), WindowId::days(7).for_model("Opus"));
        assert_eq!(model_week(" Fable "), WindowId::days(7).for_model("Fable"));
        assert_eq!(
            model_week("Example Model"),
            WindowId::days(7).for_model("Example Model")
        );
    }
}
