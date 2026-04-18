#[derive(Clone, Debug, Default)]
pub struct Mail {
    pub from: Option<String>,
    pub to: String,
    pub subject: String,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
}

impl Mail {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to = to.into();
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn body_html(mut self, html: impl Into<String>) -> Self {
        self.body_html = Some(html.into());
        self
    }

    pub fn body_text(mut self, text: impl Into<String>) -> Self {
        self.body_text = Some(text.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Mail;

    #[test]
    fn test_mail_builder_defaults() {
        let m = Mail::new()
            .to("user@example.com")
            .subject("Hello")
            .body_text("Hi there");
        assert_eq!(m.to, "user@example.com");
        assert_eq!(m.subject, "Hello");
        assert!(m.body_html.is_none());
        assert_eq!(m.body_text, Some("Hi there".to_string()));
    }

    #[test]
    fn test_mail_with_html_body() {
        let m = Mail::new()
            .from("sender@example.com")
            .to("user@example.com")
            .subject("Test")
            .body_html("<p>Hello</p>")
            .body_text("Hello");
        assert_eq!(m.from, Some("sender@example.com".to_string()));
        assert_eq!(m.body_html, Some("<p>Hello</p>".to_string()));
    }
}
