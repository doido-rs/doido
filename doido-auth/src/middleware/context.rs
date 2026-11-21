//! Lifecycle contexts passed to auth middleware callbacks.

use crate::user_model::{FetchPlan, SignInRow, UserStore};
use doido_controller::Context;
use serde_json::Value;

pub struct ValidateContext<'a> {
    pub email: &'a str,
    pub password: &'a str,
}

pub struct SignInContext<'a> {
    pub store: &'a dyn UserStore,
    pub email: &'a str,
    pub password: &'a str,
    pub plan: &'a FetchPlan,
    pub row: Option<SignInRow>,
    pub user_id: Option<Value>,
}

impl<'a> SignInContext<'a> {
    pub fn new(
        store: &'a dyn UserStore,
        email: &'a str,
        password: &'a str,
        plan: &'a FetchPlan,
    ) -> Self {
        Self {
            store,
            email,
            password,
            plan,
            row: None,
            user_id: None,
        }
    }
}

pub struct SignUpContext<'a> {
    pub store: &'a dyn UserStore,
    pub email: &'a str,
    pub password: &'a str,
}

pub struct SignOutContext<'a> {
    pub store: &'a dyn UserStore,
    pub ctx: &'a mut Context,
    pub email: Option<String>,
}
