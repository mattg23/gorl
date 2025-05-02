use grep::matcher::Matcher;
use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use serde_derive::Deserialize;

type HighlightColor = (u8, u8, u8);

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct HighlightSetting {
    pub fg_color: HighlightColor,
    pub bg_color: HighlightColor,
    pub expr: String,
    pub case_insensitive: bool,
}

#[derive(Clone)]
pub(crate) struct Highlighter {
    settings: Vec<(RegexMatcher, HighlightSetting)>,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct HighlightMatch {
    pub fg_color: HighlightColor,
    pub bg_color: HighlightColor,
}

impl Highlighter {
    pub fn new(highlight_settings: Vec<HighlightSetting>) -> Self {
        let settings = highlight_settings
            .iter()
            .filter_map(Self::create_matcher_from)
            .collect();

        Self { settings }
    }

    pub fn matches(&self, text: &str) -> Option<HighlightMatch> {
        if self.settings.is_empty() {
            return None;
        }

        let haystack = text.as_bytes();

        self.settings
            .iter()
            .find(|(matcher, _)| matcher.is_match(haystack).unwrap_or(false))
            .map(|(_, s)| HighlightMatch {
                fg_color: s.fg_color,
                bg_color: s.bg_color,
            })
    }

    fn create_matcher_from(setting: &HighlightSetting) -> Option<(RegexMatcher, HighlightSetting)> {
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(setting.case_insensitive)
            .build(setting.expr.as_str())
            .ok()?;

        Some((matcher, setting.clone()))
    }
}
