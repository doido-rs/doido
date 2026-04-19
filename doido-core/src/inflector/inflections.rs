use regex::Regex;
use std::collections::HashSet;

pub struct Inflections {
    pub(crate) plurals: Vec<(Regex, String)>,
    pub(crate) singulars: Vec<(Regex, String)>,
    pub(crate) irregulars: Vec<(String, String)>,
    pub(crate) uncountables: HashSet<String>,
    pub(crate) acronyms: HashSet<String>,
}

impl Inflections {
    pub fn new() -> Self {
        Self {
            plurals: Vec::new(),
            singulars: Vec::new(),
            irregulars: Vec::new(),
            uncountables: HashSet::new(),
            acronyms: HashSet::new(),
        }
    }

    pub fn plural(&mut self, pattern: &str, replacement: &str) {
        self.plurals.push((
            Regex::new(pattern)
                .unwrap_or_else(|e| panic!("invalid plural pattern `{pattern}`: {e}")),
            replacement.to_string(),
        ));
    }

    pub fn singular(&mut self, pattern: &str, replacement: &str) {
        self.singulars.push((
            Regex::new(pattern)
                .unwrap_or_else(|e| panic!("invalid singular pattern `{pattern}`: {e}")),
            replacement.to_string(),
        ));
    }

    pub fn irregular(&mut self, singular: &str, plural: &str) {
        self.irregulars
            .push((singular.to_lowercase(), plural.to_lowercase()));
    }

    pub fn uncountable(&mut self, word: &str) {
        self.uncountables.insert(word.to_lowercase());
    }

    pub fn acronym(&mut self, word: &str) {
        self.acronyms.insert(word.to_uppercase());
    }

    /// Returns the plural form of `word` according to the configured rules.
    pub fn pluralize(&self, word: &str) -> String {
        let lower = word.to_lowercase();

        // 1. Uncountables are returned unchanged.
        if self.uncountables.contains(&lower) {
            return word.to_string();
        }

        // 2. Irregulars take priority over regex rules.
        for (singular, plural) in &self.irregulars {
            if singular == &lower {
                return plural.clone();
            }
            if plural == &lower {
                // Already plural — return as-is.
                return plural.clone();
            }
        }

        // 3. Regex rules — tried in reverse add-order (last added = highest priority).
        for (pattern, replacement) in self.plurals.iter().rev() {
            if pattern.is_match(&lower) {
                return pattern.replace(&lower, replacement.as_str()).to_string();
            }
        }

        word.to_string()
    }

    /// `post_comment` → `PostComment` (or `APIClient` when "API" is a registered acronym).
    pub fn camelize(&self, s: &str) -> String {
        s.split('_')
            .map(|word| {
                let up = word.to_uppercase();
                if self.acronyms.contains(&up) {
                    up
                } else {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                }
            })
            .collect()
    }

    /// `post_comment` → `postComment`.
    pub fn camelize_lower(&self, s: &str) -> String {
        let camelized = self.camelize(s);
        let mut chars = camelized.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        }
    }

    /// `PostComment` → `post_comment`. Handles runs of uppercase (acronyms) correctly.
    pub fn underscore(&self, s: &str) -> String {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::with_capacity(s.len() + 4);
        for (i, &c) in chars.iter().enumerate() {
            if c.is_uppercase() && i > 0 {
                let prev = chars[i - 1];
                let next = chars.get(i + 1);
                let next_is_lower = next.is_some_and(|n| n.is_lowercase());
                let prev_is_lower = prev.is_lowercase() || prev.is_numeric();
                if prev_is_lower || (prev.is_uppercase() && next_is_lower) {
                    out.push('_');
                }
            }
            out.extend(c.to_lowercase());
        }
        out.replace('-', "_")
    }

    /// `post_comment` → `post-comment`.
    pub fn dasherize(&self, s: &str) -> String {
        s.replace('_', "-")
    }

    /// `post_comment` → `Post comment`. Strips `_id` suffix.
    pub fn humanize(&self, s: &str) -> String {
        let s = s.strip_suffix("_id").unwrap_or(s);
        let s = s.replace('_', " ");
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    /// `post_comment` → `POST_COMMENT`, `active-record` → `ACTIVE_RECORD`.
    pub fn constantize(&self, s: &str) -> String {
        s.replace('-', "_").to_uppercase()
    }

    /// Returns the singular form of `word` according to the configured rules.
    pub fn singularize(&self, word: &str) -> String {
        let lower = word.to_lowercase();

        // 1. Uncountables.
        if self.uncountables.contains(&lower) {
            return word.to_string();
        }

        // 2. Irregulars — lookup by plural form.
        for (singular, plural) in &self.irregulars {
            if plural == &lower {
                return singular.clone();
            }
            if singular == &lower {
                // Already singular — return as-is.
                return singular.clone();
            }
        }

        // 3. Regex rules.
        for (pattern, replacement) in self.singulars.iter().rev() {
            if pattern.is_match(&lower) {
                return pattern.replace(&lower, replacement.as_str()).to_string();
            }
        }

        word.to_string()
    }

    /// `PostComment` → `post_comments`  (underscore then pluralize).
    pub fn tableize(&self, s: &str) -> String {
        self.pluralize(&self.underscore(s))
    }

    /// `post_comments` → `PostComment`  (singularize then camelize).
    pub fn classify(&self, s: &str) -> String {
        self.camelize(&self.singularize(s))
    }

    /// `PostComment` → `post_comment_id`.
    pub fn foreign_key(&self, s: &str) -> String {
        format!("{}_id", self.underscore(s))
    }
}

impl Default for Inflections {
    fn default() -> Self {
        crate::inflector::rules::defaults()
    }
}
