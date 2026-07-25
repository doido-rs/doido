//! Core extensions (Rails `blank?`/`present?` and friends).

/// `blank?`/`present?` for strings, options, and collections.
pub trait Blank {
    /// Whether the value is "blank": empty/whitespace/None.
    fn is_blank(&self) -> bool;
    /// The inverse of [`is_blank`](Self::is_blank).
    fn is_present(&self) -> bool {
        !self.is_blank()
    }
}

impl Blank for str {
    fn is_blank(&self) -> bool {
        self.trim().is_empty()
    }
}

impl<T: Blank + ?Sized> Blank for &T {
    fn is_blank(&self) -> bool {
        (**self).is_blank()
    }
}

impl Blank for String {
    fn is_blank(&self) -> bool {
        self.as_str().is_blank()
    }
}

impl<T: Blank> Blank for Option<T> {
    fn is_blank(&self) -> bool {
        self.as_ref().map(Blank::is_blank).unwrap_or(true)
    }
}

impl<T> Blank for Vec<T> {
    fn is_blank(&self) -> bool {
        self.is_empty()
    }
}

/// String conveniences (Rails `truncate`).
pub trait StringExt {
    /// Truncate to at most `len` characters, appending `…` when cut.
    fn truncate_chars(&self, len: usize) -> String;
}

impl StringExt for str {
    fn truncate_chars(&self, len: usize) -> String {
        if self.chars().count() <= len {
            self.to_string()
        } else {
            let kept: String = self.chars().take(len.saturating_sub(1)).collect();
            format!("{kept}…")
        }
    }
}
