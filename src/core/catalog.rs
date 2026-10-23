use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::core::skill::Skill;

#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub skills: Vec<Skill>,
}

impl Catalog {
    pub fn new(skills: Vec<Skill>) -> Self {
        Self { skills }
    }

    pub fn filter(&self, source_filter: Option<&str>, search_query: &str) -> Vec<Skill> {
        let matcher = SkimMatcherV2::default();

        let mut results: Vec<(i64, Skill)> = self
            .skills
            .iter()
            .filter(|skill| {
                if let Some(src) = source_filter {
                    if src != "all" && skill.source_alias != src {
                        return false;
                    }
                }
                true
            })
            .filter_map(|skill| {
                if search_query.trim().is_empty() {
                    return Some((0, skill.clone()));
                }

                // Match against name, description, tags, and source_alias
                let search_target = format!(
                    "{} {} {} {}",
                    skill.name,
                    skill.description,
                    skill.tags.join(" "),
                    skill.source_alias
                );

                matcher
                    .fuzzy_match(&search_target, search_query)
                    .map(|score| (score, skill.clone()))
            })
            .collect();

        if !search_query.trim().is_empty() {
            results.sort_by_key(|a| std::cmp::Reverse(a.0));
        } else {
            results.sort_by_key(|a| a.1.name.to_lowercase());
        }

        results.into_iter().map(|(_, s)| s).collect()
    }
}
