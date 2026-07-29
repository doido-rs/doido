pub mod config;
pub mod deliverer;
pub mod global;
pub mod interceptors;
pub mod jobs;
pub mod layout;
pub mod mail;
pub mod mailer;
pub mod mime;
pub mod preview;
pub mod sendmail;
pub mod smtp;

pub use config::{Backend, MailerConfig};
pub use deliverer::{Deliverer, LogDeliverer, TestDeliverer};
pub use doido_mailer_macros::mailer;
pub use mail::Mail;
pub use mailer::Mailer;
