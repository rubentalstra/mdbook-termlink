//! How linked glossary terms are rendered in the output HTML.

use std::str::FromStr;

/// Output shape chosen for every linked glossary occurrence.
///
/// See the `display-mode` option in the README for the rendered HTML produced
/// by each variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    /// `<a href title class>term</a>` — anchor with native browser tooltip.
    #[default]
    Link,
    /// `<abbr title tabindex class>term</abbr>` — tooltip only, no navigation.
    Tooltip,
    /// `<a href class><abbr title tabindex>term</abbr></a>` — anchor wrapping
    /// a semantic abbreviation.
    Both,
}

/// Error returned when a string cannot be parsed into a [`DisplayMode`].
#[derive(Debug, thiserror::Error)]
#[error("invalid display-mode '{value}'; expected 'link', 'tooltip', or 'both'")]
pub struct InvalidDisplayMode {
    /// The verbatim string that failed to parse.
    pub value: String,
}

impl FromStr for DisplayMode {
    type Err = InvalidDisplayMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "link" => Ok(Self::Link),
            "tooltip" => Ok(Self::Tooltip),
            "both" => Ok(Self::Both),
            other => Err(InvalidDisplayMode {
                value: other.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_variant() {
        assert_eq!("link".parse::<DisplayMode>().unwrap(), DisplayMode::Link);
        assert_eq!(
            "tooltip".parse::<DisplayMode>().unwrap(),
            DisplayMode::Tooltip
        );
        assert_eq!("both".parse::<DisplayMode>().unwrap(), DisplayMode::Both);
    }

    #[test]
    fn rejects_unknown_value() {
        let err = "nonsense".parse::<DisplayMode>().unwrap_err();
        assert_eq!(err.value, "nonsense");
        assert!(err.to_string().contains("nonsense"));
    }

    #[test]
    fn rejects_empty() {
        assert!("".parse::<DisplayMode>().is_err());
    }

    #[test]
    fn default_is_link() {
        assert_eq!(DisplayMode::default(), DisplayMode::Link);
    }
}
