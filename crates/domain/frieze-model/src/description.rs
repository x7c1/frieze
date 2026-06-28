//! Internal helper for normalizing description text.
//!
//! Following the "empty container omitted" principle, a description that
//! is empty or contains only whitespace must collapse to `None` so the
//! renderer never emits an empty `description` key. All constructors that
//! take an optional description funnel their input through
//! [`normalize_description`].

pub(crate) fn normalize_description(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_normalizes_to_none() {
        assert_eq!(normalize_description(String::new()), None);
    }

    #[test]
    fn whitespace_only_input_normalizes_to_none() {
        assert_eq!(normalize_description("   ".into()), None);
        assert_eq!(normalize_description("\n\t  \n".into()), None);
    }

    #[test]
    fn non_empty_input_is_preserved_verbatim() {
        assert_eq!(
            normalize_description("hello".into()),
            Some("hello".to_string())
        );
        // Internal whitespace and embedded newlines are preserved.
        assert_eq!(
            normalize_description("line one\nline two".into()),
            Some("line one\nline two".to_string())
        );
    }
}
