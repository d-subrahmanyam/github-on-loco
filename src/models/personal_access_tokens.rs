//! Personal access tokens used by git clients.
//!
//! A git remote cannot carry a JWT, so pushes authenticate with a PAT sent as
//! the HTTP Basic *password*. The token has the form `glp_<id>_<secret>`:
//!
//! * `<id>` lets us look the row up directly (hashes are salted, so we cannot
//!   search by hash);
//! * `<secret>` is never stored — only its password hash is. Anyone who reads
//!   the database cannot reconstruct a working token.
//!
//! The account password is still accepted by the git transport as a
//! convenience for local development, but PATs are the thing to use in a
//! remote.

use super::_entities::personal_access_tokens::Column;
pub use super::_entities::personal_access_tokens::{ActiveModel, Entity, Model};
use super::users;
use chrono::Utc;
use loco_rs::{hash, prelude::*};
use sea_orm::entity::prelude::*;

pub type PersonalAccessTokens = Entity;

/// Prefix that marks a string as one of our tokens.
pub const TOKEN_PREFIX: &str = "glp";
/// Number of random characters in the secret part of a token.
pub const TOKEN_SECRET_LEN: usize = 32;

/// Builds the string a user puts in their git remote.
#[must_use]
pub fn format_token(id: i64, secret: &str) -> String {
    format!("{TOKEN_PREFIX}_{id}_{secret}")
}

/// Extracts the row id encoded in a token.
#[must_use]
pub fn token_id(token: &str) -> Option<i64> {
    let rest = token.strip_prefix(&format!("{TOKEN_PREFIX}_"))?;
    let (id, secret) = rest.split_once('_')?;
    if secret.is_empty() {
        return None;
    }
    id.parse::<i64>().ok()
}

/// Extracts the secret part of a token.
#[must_use]
pub fn token_secret(token: &str) -> Option<&str> {
    let rest = token.strip_prefix(&format!("{TOKEN_PREFIX}_"))?;
    let (_id, secret) = rest.split_once('_')?;
    (!secret.is_empty()).then_some(secret)
}

/// Read-oriented domain logic.
impl Model {
    /// Whether this token has passed its expiry time. Tokens without an
    /// expiry never expire.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at.timestamp() < Utc::now().timestamp())
    }

    /// Records that the token was just used. Best-effort: a failed write here
    /// must not invalidate an otherwise successful authentication.
    ///
    /// # Errors
    /// Returns a database error if the update fails.
    pub async fn touch(&self, db: &DatabaseConnection) -> Result<()> {
        let now: DateTimeWithTimeZone = Utc::now().into();
        let active = ActiveModel {
            id: Set(self.id),
            last_used_at: Set(Some(now)),
            ..Default::default()
        };
        active.update(db).await?;
        Ok(())
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Issues a new token for a user and returns it alongside the stored row.
    ///
    /// The returned plaintext is shown to the user exactly once; the database
    /// only ever holds the hash.
    ///
    /// # Errors
    /// Returns an error if hashing or insertion fails.
    pub async fn issue(
        db: &DatabaseConnection,
        user_id: i64,
        name: &str,
        scopes: Option<&str>,
        expires_at: Option<DateTimeWithTimeZone>,
    ) -> Result<(Model, String)> {
        let secret = hash::random_string(TOKEN_SECRET_LEN);
        let token_hash = hash::hash_password(&secret)
            .map_err(|e| Error::string(&format!("failed to hash token: {e}")))?;

        let token = ActiveModel {
            user_id: Set(user_id),
            name: Set(name.to_owned()),
            token_hash: Set(token_hash),
            scopes: Set(scopes.map(str::to_owned)),
            expires_at: Set(expires_at),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok((token.clone(), format_token(token.id, &secret)))
    }
}

/// Custom finders and authentication.
impl Entity {
    /// Resolves a plaintext token to the token row and its owner.
    ///
    /// # Errors
    /// Returns a model error for a malformed, unknown, expired or mismatched
    /// token.
    pub async fn authenticate(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<(Model, users::Model)> {
        let id = token_id(token).ok_or_else(|| ModelError::msg("malformed token"))?;
        let secret = token_secret(token).ok_or_else(|| ModelError::msg("malformed token"))?;

        let pat = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;

        if !hash::verify_password(secret, &pat.token_hash) {
            return Err(ModelError::msg("invalid token"));
        }
        if pat.is_expired() {
            return Err(ModelError::msg("token expired"));
        }

        let user = users::Entity::find_by_id(pat.user_id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;

        Ok((pat, user))
    }

    /// All active tokens belonging to a user.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_user(db: &DatabaseConnection, user_id: i64) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Revokes a token, but only for the user who issued it.
    ///
    /// # Errors
    /// Returns 404 if the token does not exist or belongs to someone else.
    pub async fn revoke(db: &DatabaseConnection, id: i64, user_id: i64) -> Result<()> {
        let token = Entity::find_by_id(id).one(db).await?;
        match token {
            Some(token) if token.user_id == user_id => {
                ActiveModel {
                    id: Set(id),
                    ..Default::default()
                }
                .delete(db)
                .await?;
                Ok(())
            }
            _ => not_found(),
        }
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trips_through_id_and_secret() {
        let token = format_token(42, "abcdef");
        assert_eq!(token_id(&token), Some(42));
        assert_eq!(token_secret(&token), Some("abcdef"));
    }

    #[test]
    fn malformed_tokens_are_rejected() {
        for token in [
            "",
            "glp",
            "glp_",
            "glp_42",
            "glp_42_",
            "glp_x_y",
            "pat_42_secret",
        ] {
            assert_eq!(token_id(token), None, "expected {token:?} to be malformed");
        }
    }

    #[test]
    fn tokens_without_an_expiry_are_not_expired() {
        // A row built by hand stands in for a freshly issued token.
        let pat = Model {
            id: 1,
            name: "laptop".to_owned(),
            token_hash: "hash".to_owned(),
            scopes: None,
            expires_at: None,
            last_used_at: None,
            user_id: 1,
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
        };
        assert!(!pat.is_expired());
    }
}
