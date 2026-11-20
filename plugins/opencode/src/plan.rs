//! The name of a ChatGPT plan, as ChatGPT shows it.

/// The plan a ChatGPT login token names, such as `plus`, as ChatGPT names
/// it, such as "Plus"; an unknown plan with its first letter in capitals.
/// `None` for a blank one.
pub(crate) fn plan_name(plan_type: &str) -> Option<String> {
    let plan_type = plan_type.trim();
    let known = match plan_type {
        "free" => Some("Free"),
        "go" => Some("Go"),
        "plus" => Some("Plus"),
        "pro" => Some("Pro 200"),
        "prolite" => Some("Pro 100"),
        "promax" => Some("Pro 500"),
        "team" | "self_serve_business_usage_based" => Some("Business"),
        "self_serve_business_prolite" => Some("Business Premium"),
        "enterprise_cbp_automation" => Some("Enterprise (Automation)"),
        "business" | "enterprise" | "ent26" | "enterprise_cbp_usage_based" => {
            Some("Enterprise")
        },
        "edu" => Some("Edu"),
        "edu_plus" => Some("Edu Plus"),
        "edu_pro" => Some("Edu Pro"),
        "sci" => Some("Academic Researchers"),
        "sci_plus" => Some("Academic Researchers Plus"),
        _ => None,
    };
    if let Some(name) = known {
        return Some(name.to_string());
    }

    let words = plan_type.replace('_', " ");
    let mut letters = words.chars();
    let first = letters.next()?;

    Some(format!("{}{}", first.to_uppercase(), letters.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_reads_like_the_name_chatgpt_shows() {
        let cases = [
            ("plus", Some("Plus")),
            ("team", Some("Business")),
            ("pro", Some("Pro 200")),
            ("new_plan", Some("New plan")),
            (" ", None),
        ];

        for (plan_type, name) in cases {
            assert_eq!(plan_name(plan_type).as_deref(), name, "{plan_type}");
        }
    }
}
