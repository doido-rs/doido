pub mod deliverer;
pub mod mail;

pub use deliverer::{Deliverer, LogDeliverer, TestDeliverer};
pub use doido_mailer_macros::mailer;
pub use mail::Mail;
