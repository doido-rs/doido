//! Composable user-record mutations — keyed off [`super::UserField`], not Devise module names.

use super::plan::UserField;

/// How to resolve the row to update.
#[derive(Debug, Clone)]
pub enum UserLocator {
    ByEmail(String),
    ByField { field: UserField, value: String },
}

/// One column change.
#[derive(Debug, Clone)]
pub enum FieldChange {
    Set { field: UserField, value: String },
    Clear { field: UserField },
    Increment { field: UserField, by: i32 },
}

/// Secondary index for token lookups (Redis; ignored by SQL).
#[derive(Debug, Clone)]
pub struct TokenIndex {
    pub field: UserField,
    pub token: String,
}

/// Fluent patch: chain `.set()` / `.clear()` / `.increment()`, then `.build()`.
#[derive(Debug, Clone, Default)]
pub struct UserPatchBuilder {
    locator: Option<UserLocator>,
    changes: Vec<FieldChange>,
    add_indexes: Vec<TokenIndex>,
    remove_indexes: Vec<TokenIndex>,
}

impl UserPatchBuilder {
    pub fn by_email(email: impl Into<String>) -> Self {
        Self {
            locator: Some(UserLocator::ByEmail(email.into())),
            ..Default::default()
        }
    }

    pub fn by_field(field: UserField, value: impl Into<String>) -> Self {
        Self {
            locator: Some(UserLocator::ByField {
                field,
                value: value.into(),
            }),
            ..Default::default()
        }
    }

    pub fn set(mut self, field: UserField, value: impl Into<String>) -> Self {
        self.changes.push(FieldChange::Set {
            field,
            value: value.into(),
        });
        self
    }

    pub fn clear(mut self, field: UserField) -> Self {
        self.changes.push(FieldChange::Clear { field });
        self
    }

    pub fn increment(mut self, field: UserField) -> Self {
        self.changes.push(FieldChange::Increment { field, by: 1 });
        self
    }

    pub fn increment_by(mut self, field: UserField, by: i32) -> Self {
        self.changes.push(FieldChange::Increment { field, by });
        self
    }

    /// Register token → email lookup (confirmation / reset tokens on Redis).
    pub fn index_token(mut self, field: UserField, token: impl Into<String>) -> Self {
        self.add_indexes.push(TokenIndex {
            field,
            token: token.into(),
        });
        self
    }

    pub fn drop_token_index(mut self, field: UserField, token: impl Into<String>) -> Self {
        self.remove_indexes.push(TokenIndex {
            field,
            token: token.into(),
        });
        self
    }

    pub fn build(self) -> Result<UserStoreAction, crate::error::AuthError> {
        let locator = self.locator.ok_or_else(|| {
            crate::error::AuthError::Internal(
                "UserPatchBuilder: set locator with by_email or by_field before build".into(),
            )
        })?;
        if self.changes.is_empty() && self.add_indexes.is_empty() && self.remove_indexes.is_empty()
        {
            return Err(crate::error::AuthError::Internal(
                "UserPatchBuilder: no field changes or indexes".into(),
            ));
        }
        Ok(UserStoreAction::Patch {
            locator,
            changes: self.changes,
            add_indexes: self.add_indexes,
            remove_indexes: self.remove_indexes,
        })
    }
}

/// Storage command — either a composable patch or one optimized multi-column update.
#[derive(Debug, Clone)]
pub enum UserStoreAction {
    Patch {
        locator: UserLocator,
        changes: Vec<FieldChange>,
        add_indexes: Vec<TokenIndex>,
        remove_indexes: Vec<TokenIndex>,
    },
    /// Shifts `current_*` into `last_*` and bumps `sign_in_count` (single SQL round-trip).
    RecordSuccessfulSignIn {
        email: String,
        signed_in_at: String,
        ip: Option<String>,
    },
}
