//! Re-exports the [`UserModel`] contract (legacy `user` module path).

pub use crate::user_model::{
    authenticate_password, find_by_email_mapped, find_by_id_mapped, id_to_sea_value,
    RegisterableUserModel, UserModel,
};
