use sqlx::{PgPool, Row};
use chrono::{Utc, DateTime};

use super::models::{DbUser, CreateUser, DbTemplate, CreateTemplate, DbTemplateField, CreateTemplateField, CreateSubmitter, DbSubmitter, DbPaymentRecord, CreatePaymentRecord, DbSignatureData, DbSubscriptionPlan, DbTemplateFolder, CreateTemplateFolder, DbSubmissionField, CreateSubmissionField, DbGlobalSettings, UpdateGlobalSettings, DbEmailTemplate, UpdateEmailTemplate, DbAccount, CreateAccount, UpdateAccount, DbAccountLinkedAccount};
use crate::models::signature::SignatureInfo;

// Structured query implementations for better organization
pub struct AccountQueries;
pub struct UserQueries;
pub struct TemplateQueries;
pub struct TemplateFolderQueries;
pub struct TemplateFieldQueries;
pub struct SubmitterQueries;
pub struct SubmissionFieldQueries;
pub struct GlobalSettingsQueries;
pub struct EmailTemplateQueries;

impl AccountQueries {
    /// Create a new account
    pub async fn create_account(pool: &PgPool, account_data: CreateAccount) -> Result<DbAccount, sqlx::Error> {
        let now = Utc::now();
        
        let row = sqlx::query(
            r#"
            INSERT INTO accounts (name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, slug, created_at, updated_at
            "#
        )
        .bind(&account_data.name)
        .bind(&account_data.slug)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(DbAccount {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            slug: row.try_get("slug")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// Get account by ID
    pub async fn get_account_by_id(pool: &PgPool, id: i64) -> Result<Option<DbAccount>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, slug, created_at, updated_at FROM accounts WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbAccount {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    /// Get account by slug
    pub async fn get_account_by_slug(pool: &PgPool, slug: &str) -> Result<Option<DbAccount>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, slug, created_at, updated_at FROM accounts WHERE slug = $1"
        )
        .bind(slug)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbAccount {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    /// Update account
    pub async fn update_account(pool: &PgPool, id: i64, update_data: UpdateAccount) -> Result<DbAccount, sqlx::Error> {
        let account = Self::get_account_by_id(pool, id).await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        let name = update_data.name.unwrap_or(account.name);

        let row = sqlx::query(
            r#"
            UPDATE accounts 
            SET name = $1, updated_at = $2
            WHERE id = $3
            RETURNING id, name, slug, created_at, updated_at
            "#
        )
        .bind(&name)
        .bind(Utc::now())
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(DbAccount {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            slug: row.try_get("slug")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// Get all users for an account (excluding archived unless specified)
    pub async fn get_account_users(pool: &PgPool, account_id: i64, include_archived: bool) -> Result<Vec<DbUser>, sqlx::Error> {
        let query = if include_archived {
            "SELECT id, name, email, password_hash, role, is_active, activation_token, account_id, archived_at, subscription_status, subscription_expires_at, free_usage_count, signature, initials, two_factor_secret, two_factor_enabled, created_at, updated_at FROM users WHERE account_id = $1 ORDER BY created_at DESC"
        } else {
            "SELECT id, name, email, password_hash, role, is_active, activation_token, account_id, archived_at, subscription_status, subscription_expires_at, free_usage_count, signature, initials, two_factor_secret, two_factor_enabled, created_at, updated_at FROM users WHERE account_id = $1 AND archived_at IS NULL ORDER BY created_at DESC"
        };

        let rows = sqlx::query(query)
            .bind(account_id)
            .fetch_all(pool)
            .await?;

        let mut users = Vec::new();
        for row in rows {
            users.push(DbUser {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                email: row.try_get("email")?,
                password_hash: row.try_get("password_hash")?,
                role: row.try_get("role")?,
                is_active: row.try_get("is_active")?,
                activation_token: row.try_get("activation_token")?,
                account_id: row.try_get("account_id")?,
                archived_at: row.try_get("archived_at")?,
                subscription_status: row.try_get("subscription_status")?,
                subscription_expires_at: row.try_get("subscription_expires_at")?,
                free_usage_count: row.try_get("free_usage_count")?,
                signature: row.try_get("signature")?,
                initials: row.try_get("initials")?,
                two_factor_secret: row.try_get("two_factor_secret")?,
                two_factor_enabled: row.try_get("two_factor_enabled")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(users)
    }

    /// Archive a user (soft delete)
    pub async fn archive_user(pool: &PgPool, user_id: i64, account_id: i64) -> Result<DbUser, sqlx::Error> {
        // Check this is not the last active user in the account
        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE account_id = $1 AND archived_at IS NULL"
        )
        .bind(account_id)
        .fetch_one(pool)
        .await?;

        if active_count <= 1 {
            return Err(sqlx::Error::Protocol("Cannot archive the last user in an account".to_string()));
        }

        let row = sqlx::query(
            r#"
            UPDATE users 
            SET archived_at = $1, updated_at = $2
            WHERE id = $3 AND account_id = $4
            RETURNING id, name, email, password_hash, role, is_active, activation_token, account_id, archived_at, subscription_status, subscription_expires_at, free_usage_count, signature, initials, two_factor_secret, two_factor_enabled, created_at, updated_at
            "#
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(user_id)
        .bind(account_id)
        .fetch_one(pool)
        .await?;

        Ok(DbUser {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            role: row.try_get("role")?,
            is_active: row.try_get("is_active")?,
            activation_token: row.try_get("activation_token")?,
            account_id: row.try_get("account_id")?,
            archived_at: row.try_get("archived_at")?,
            subscription_status: row.try_get("subscription_status")?,
            subscription_expires_at: row.try_get("subscription_expires_at")?,
            free_usage_count: row.try_get("free_usage_count")?,
            signature: row.try_get("signature")?,
            initials: row.try_get("initials")?,
            two_factor_secret: row.try_get("two_factor_secret")?,
            two_factor_enabled: row.try_get("two_factor_enabled")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// Unarchive a user
    pub async fn unarchive_user(pool: &PgPool, user_id: i64, account_id: i64) -> Result<DbUser, sqlx::Error> {
        let row = sqlx::query(
            r#"
            UPDATE users 
            SET archived_at = NULL, updated_at = $1
            WHERE id = $2 AND account_id = $3
            RETURNING id, name, email, password_hash, role, is_active, activation_token, account_id, archived_at, subscription_status, subscription_expires_at, free_usage_count, signature, initials, two_factor_secret, two_factor_enabled, created_at, updated_at
            "#
        )
        .bind(Utc::now())
        .bind(user_id)
        .bind(account_id)
        .fetch_one(pool)
        .await?;

        Ok(DbUser {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            role: row.try_get("role")?,
            is_active: row.try_get("is_active")?,
            activation_token: row.try_get("activation_token")?,
            account_id: row.try_get("account_id")?,
            archived_at: row.try_get("archived_at")?,
            subscription_status: row.try_get("subscription_status")?,
            subscription_expires_at: row.try_get("subscription_expires_at")?,
            free_usage_count: row.try_get("free_usage_count")?,
            signature: row.try_get("signature")?,
            initials: row.try_get("initials")?,
            two_factor_secret: row.try_get("two_factor_secret")?,
            two_factor_enabled: row.try_get("two_factor_enabled")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// Delete a user permanently
    pub async fn delete_user(pool: &PgPool, user_id: i64, account_id: i64) -> Result<(), sqlx::Error> {
        // Check this is not the last user in the account
        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE account_id = $1"
        )
        .bind(account_id)
        .fetch_one(pool)
        .await?;

        if active_count <= 1 {
            return Err(sqlx::Error::Protocol("Cannot delete the last user in an account".to_string()));
        }

        sqlx::query("DELETE FROM users WHERE id = $1 AND account_id = $2")
            .bind(user_id)
            .bind(account_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Link two accounts (for testing)
    pub async fn link_accounts(pool: &PgPool, account_id: i64, linked_account_id: i64) -> Result<DbAccountLinkedAccount, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO account_linked_accounts (account_id, linked_account_id, created_at)
            VALUES ($1, $2, $3)
            RETURNING id, account_id, linked_account_id, created_at
            "#
        )
        .bind(account_id)
        .bind(linked_account_id)
        .bind(Utc::now())
        .fetch_one(pool)
        .await?;

        Ok(DbAccountLinkedAccount {
            id: row.try_get("id")?,
            account_id: row.try_get("account_id")?,
            linked_account_id: row.try_get("linked_account_id")?,
            created_at: row.try_get("created_at")?,
        })
    }

    /// Get linked accounts
    pub async fn get_linked_accounts(pool: &PgPool, account_id: i64) -> Result<Vec<DbAccount>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT a.id, a.name, a.slug, a.created_at, a.updated_at
            FROM accounts a
            INNER JOIN account_linked_accounts ala ON a.id = ala.linked_account_id
            WHERE ala.account_id = $1
            "#
        )
        .bind(account_id)
        .fetch_all(pool)
        .await?;

        let mut accounts = Vec::new();
        for row in rows {
            accounts.push(DbAccount {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(accounts)
    }
}

impl UserQueries {
    pub async fn get_user_by_id(pool: &PgPool, id: i64) -> Result<Option<DbUser>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, email, password_hash, role, is_active, activation_token, account_id, archived_at, subscription_status, subscription_expires_at, free_usage_count, signature, initials, two_factor_secret, two_factor_enabled, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbUser {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                email: row.try_get("email")?,
                password_hash: row.try_get("password_hash")?,
                role: row.try_get("role")?,
                is_active: row.try_get("is_active")?,
                activation_token: row.try_get("activation_token")?,
                account_id: row.try_get("account_id")?,
                archived_at: row.try_get("archived_at")?,
                subscription_status: row.try_get("subscription_status")?,
                subscription_expires_at: row.try_get("subscription_expires_at")?,
                free_usage_count: row.try_get("free_usage_count")?,
                signature: row.try_get("signature")?,
                initials: row.try_get("initials")?,
                two_factor_secret: row.try_get("two_factor_secret")?,
                two_factor_enabled: row.try_get("two_factor_enabled")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn create_user(pool: &PgPool, user_data: CreateUser) -> Result<DbUser, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO users (name, email, password_hash, role, is_active, activation_token, account_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, name, email, password_hash, role, is_active, activation_token, account_id, archived_at, subscription_status, 
                     subscription_expires_at, free_usage_count, signature, initials, two_factor_secret, 
                     two_factor_enabled, created_at, updated_at
            "#
        )
        .bind(&user_data.name)
        .bind(&user_data.email)
        .bind(&user_data.password_hash)
        .bind(&user_data.role)
        .bind(user_data.is_active)
        .bind(&user_data.activation_token)
        .bind(&user_data.account_id)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(DbUser {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            role: row.try_get("role")?,
            is_active: row.try_get("is_active")?,
            activation_token: row.try_get("activation_token")?,
            account_id: row.try_get("account_id")?,
            archived_at: row.try_get("archived_at")?,
            subscription_status: row.try_get("subscription_status")?,
            subscription_expires_at: row.try_get("subscription_expires_at")?,
            free_usage_count: row.try_get("free_usage_count")?,
            signature: row.try_get("signature")?,
            initials: row.try_get("initials")?,
            two_factor_secret: row.try_get("two_factor_secret")?,
            two_factor_enabled: row.try_get("two_factor_enabled")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<DbUser>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, email, password_hash, role, is_active, activation_token, account_id, archived_at, subscription_status, subscription_expires_at, free_usage_count, signature, initials, two_factor_secret, two_factor_enabled, created_at, updated_at FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbUser {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                email: row.try_get("email")?,
                password_hash: row.try_get("password_hash")?,
                role: row.try_get("role")?,
                is_active: row.try_get("is_active")?,
                activation_token: row.try_get("activation_token")?,
                account_id: row.try_get("account_id")?,
                archived_at: row.try_get("archived_at")?,
                subscription_status: row.try_get("subscription_status")?,
                subscription_expires_at: row.try_get("subscription_expires_at")?,
                free_usage_count: row.try_get("free_usage_count")?,
                signature: row.try_get("signature")?,
                initials: row.try_get("initials")?,
                two_factor_secret: row.try_get("two_factor_secret")?,
                two_factor_enabled: row.try_get("two_factor_enabled")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn activate_user(pool: &PgPool, email: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET is_active = TRUE, activation_token = NULL WHERE email = $1"
        )
        .bind(email)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_user_password(pool: &PgPool, user_id: i64, new_password_hash: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET password_hash = $1, updated_at = $2 WHERE id = $3"
        )
        .bind(new_password_hash)
        .bind(Utc::now())
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_user_name(pool: &PgPool, user_id: i64, new_name: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET name = $1, updated_at = $2 WHERE id = $3"
        )
        .bind(new_name)
        .bind(Utc::now())
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_user_email(pool: &PgPool, user_id: i64, new_email: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET email = $1, updated_at = $2 WHERE id = $3"
        )
        .bind(new_email)
        .bind(Utc::now())
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_user_signature(pool: &PgPool, user_id: i64, signature: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET signature = $1, updated_at = $2 WHERE id = $3"
        )
        .bind(signature)
        .bind(Utc::now())
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_user_initials(pool: &PgPool, user_id: i64, initials: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET initials = $1, updated_at = $2 WHERE id = $3"
        )
        .bind(initials)
        .bind(Utc::now())
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

impl TemplateQueries {
    pub async fn create_template(pool: &PgPool, template_data: CreateTemplate) -> Result<DbTemplate, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO templates (name, slug, user_id, account_id, folder_id, documents, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at
            "#
        )
        .bind(&template_data.name)
        .bind(&template_data.slug)
        .bind(&template_data.user_id)
        .bind(&template_data.account_id)
        .bind(&template_data.folder_id)
        .bind(&template_data.documents)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(DbTemplate {
            id: row.get(0),
            name: row.get(1),
            slug: row.get(2),
            user_id: row.get(3),
            account_id: row.get(4),
            folder_id: row.get(5),
            // fields: None, // Removed - now stored in template_fields table
            documents: row.get(6),
            created_at: row.get(7),
            updated_at: row.get(8),
        })
    }

    pub async fn get_template_by_id(pool: &PgPool, id: i64) -> Result<Option<DbTemplate>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at FROM templates WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbTemplate {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                folder_id: row.try_get("folder_id")?,
                // fields: None, // Removed - now stored in template_fields table
                documents: row.try_get("documents")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_template_by_slug(pool: &PgPool, slug: &str) -> Result<Option<DbTemplate>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at FROM templates WHERE slug = $1"
        )
        .bind(slug)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbTemplate {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                folder_id: row.try_get("folder_id")?,
                // fields: None, // Removed - now stored in template_fields table
                documents: row.try_get("documents")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_all_templates(pool: &PgPool, user_id: i64) -> Result<Vec<DbTemplate>, sqlx::Error> {
        // Get user's account_id first
        let account_id_result = sqlx::query("SELECT account_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
        
        let account_id: Option<i64> = account_id_result.try_get("account_id")?;
        
        let query_str = if let Some(acc_id) = account_id {
            // User has account - show all templates in the account
            "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at FROM templates WHERE account_id = $1 AND folder_id IS NULL ORDER BY created_at DESC"
        } else {
            // User doesn't have account - show only their templates
            "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at FROM templates WHERE user_id = $1 AND folder_id IS NULL ORDER BY created_at DESC"
        };
        
        let rows = sqlx::query(query_str)
            .bind(account_id.unwrap_or(user_id))
            .fetch_all(pool)
            .await?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(DbTemplate {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                folder_id: row.try_get("folder_id")?,
                // fields: None, // Removed - now stored in template_fields table
                documents: row.try_get("documents")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(templates)
    }

    // Get templates accessible by team (invited users can see inviter's templates)
    pub async fn get_team_templates(pool: &PgPool, user_id: i64) -> Result<Vec<DbTemplate>, sqlx::Error> {
        // Get user's account_id
        let account_id_result = sqlx::query("SELECT account_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
        let account_id: Option<i64> = account_id_result.try_get("account_id")?;

        // If user has account_id, get all templates in that account
        // Otherwise, only get user's own templates
        let query_str = if account_id.is_some() {
            "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at 
             FROM templates 
             WHERE account_id = $1 AND folder_id IS NULL 
             ORDER BY created_at DESC"
        } else {
            "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at 
             FROM templates 
             WHERE user_id = $1 AND folder_id IS NULL 
             ORDER BY created_at DESC"
        };

        let rows = if let Some(acc_id) = account_id {
            sqlx::query(query_str)
                .bind(acc_id)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query(query_str)
                .bind(user_id)
                .fetch_all(pool)
                .await?
        };

        let mut templates = Vec::new();
        for row in rows {
            templates.push(DbTemplate {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                folder_id: row.try_get("folder_id")?,
                // fields: None, // Removed - now stored in template_fields table
                documents: row.try_get("documents")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(templates)
    }

    pub async fn update_template(pool: &PgPool, id: i64, name: Option<&str>) -> Result<Option<DbTemplate>, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            UPDATE templates
            SET name = COALESCE($2, name), updated_at = $3
            WHERE id = $1
            RETURNING id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at
            "#
        )
        .bind(id)
        .bind(name)
        .bind(now)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbTemplate {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                folder_id: row.try_get("folder_id")?,
                // fields: None, // Removed - now stored in template_fields table
                documents: row.try_get("documents")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn delete_template(pool: &PgPool, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM templates WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn clone_template(pool: &PgPool, original_id: i64, user_id: i64, new_name: &str, new_slug: &str) -> Result<Option<DbTemplate>, sqlx::Error> {
        // First get the original template
        if let Some(original) = Self::get_template_by_id(pool, original_id).await? {
            let now = Utc::now();
            let create_data = CreateTemplate {
                name: new_name.to_string(),
                slug: new_slug.to_string(),
                user_id: user_id,
                account_id: original.account_id, // Use same account
                folder_id: original.folder_id,
                // fields: None, // Removed - will be cloned separately via TemplateFieldQueries
                documents: original.documents,
            };

            Self::create_template(pool, create_data).await.map(Some)
        } else {
            Ok(None)
        }
    }
}

impl TemplateFolderQueries {
    pub async fn create_folder(pool: &PgPool, folder_data: CreateTemplateFolder) -> Result<DbTemplateFolder, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO template_folders (name, user_id, account_id, parent_folder_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, user_id, account_id, parent_folder_id, created_at, updated_at
            "#
        )
        .bind(&folder_data.name)
        .bind(folder_data.user_id)
        .bind(folder_data.account_id)
        .bind(folder_data.parent_folder_id)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(DbTemplateFolder {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            user_id: row.try_get("user_id")?,
            account_id: row.try_get("account_id")?,
            parent_folder_id: row.try_get("parent_folder_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn get_folder_by_id(pool: &PgPool, id: i64) -> Result<Option<DbTemplateFolder>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, name, user_id, account_id, parent_folder_id, created_at, updated_at FROM template_folders WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbTemplateFolder {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                parent_folder_id: row.try_get("parent_folder_id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_folders_by_user(pool: &PgPool, user_id: i64) -> Result<Vec<DbTemplateFolder>, sqlx::Error> {
        // Get user's account_id
        let account_id_result = sqlx::query("SELECT account_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
        let account_id: Option<i64> = account_id_result.try_get("account_id")?;
        
        let rows = if let Some(acc_id) = account_id {
            sqlx::query(
                "SELECT id, name, user_id, account_id, parent_folder_id, created_at, updated_at FROM template_folders WHERE account_id = $1 ORDER BY name ASC"
            )
            .bind(acc_id)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, name, user_id, account_id, parent_folder_id, created_at, updated_at FROM template_folders WHERE user_id = $1 ORDER BY name ASC"
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?
        };

        let mut folders = Vec::new();
        for row in rows {
            folders.push(DbTemplateFolder {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                parent_folder_id: row.try_get("parent_folder_id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(folders)
    }

    pub async fn get_folders_by_parent(pool: &PgPool, user_id: i64, parent_id: Option<i64>) -> Result<Vec<DbTemplateFolder>, sqlx::Error> {
        let rows = if let Some(parent_id) = parent_id {
            sqlx::query(
                "SELECT id, name, user_id, account_id, parent_folder_id, created_at, updated_at FROM template_folders WHERE user_id = $1 AND parent_folder_id = $2 ORDER BY name ASC"
            )
            .bind(user_id)
            .bind(parent_id)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, name, user_id, account_id, parent_folder_id, created_at, updated_at FROM template_folders WHERE user_id = $1 AND parent_folder_id IS NULL ORDER BY name ASC"
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?
        };

        let mut folders = Vec::new();
        for row in rows {
            folders.push(DbTemplateFolder {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                parent_folder_id: row.try_get("parent_folder_id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(folders)
    }

    pub async fn get_templates_in_folder(pool: &PgPool, user_id: i64, folder_id: Option<i64>) -> Result<Vec<DbTemplate>, sqlx::Error> {
        // Get user's account_id
        let account_id_result = sqlx::query("SELECT account_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
        let account_id: Option<i64> = account_id_result.try_get("account_id")?;
        
        let rows = if let Some(folder_id) = folder_id {
            if let Some(acc_id) = account_id {
                sqlx::query(
                    "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at FROM templates WHERE account_id = $1 AND folder_id = $2 ORDER BY created_at DESC"
                )
                .bind(acc_id)
                .bind(folder_id)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query(
                    "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at FROM templates WHERE user_id = $1 AND folder_id = $2 ORDER BY created_at DESC"
                )
                .bind(user_id)
                .bind(folder_id)
                .fetch_all(pool)
                .await?
            }
        } else {
            if let Some(acc_id) = account_id {
                sqlx::query(
                    "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at FROM templates WHERE account_id = $1 AND folder_id IS NULL ORDER BY created_at DESC"
                )
                .bind(acc_id)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query(
                    "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at FROM templates WHERE user_id = $1 AND folder_id IS NULL ORDER BY created_at DESC"
                )
                .bind(user_id)
                .fetch_all(pool)
                .await?
            }
        };

        let mut templates = Vec::new();
        for row in rows {
            templates.push(DbTemplate {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                folder_id: row.try_get("folder_id")?,
                documents: row.try_get("documents")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(templates)
    }

    pub async fn update_folder(pool: &PgPool, id: i64, name: Option<&str>, parent_folder_id: Option<Option<i64>>) -> Result<Option<DbTemplateFolder>, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            UPDATE template_folders
            SET name = COALESCE($2, name),
                parent_folder_id = COALESCE($3, parent_folder_id),
                updated_at = $4
            WHERE id = $1
            RETURNING id, name, user_id, account_id, parent_folder_id, created_at, updated_at
            "#
        )
        .bind(id)
        .bind(name)
        .bind(parent_folder_id)
        .bind(now)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbTemplateFolder {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                parent_folder_id: row.try_get("parent_folder_id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn delete_folder(pool: &PgPool, id: i64, folder_user_id: i64) -> Result<bool, sqlx::Error> {
        // First, move all templates in this folder to root (no folder)
        sqlx::query("UPDATE templates SET folder_id = NULL WHERE folder_id = $1 AND user_id = $2")
            .bind(id)
            .bind(folder_user_id)
            .execute(pool)
            .await?;

        // Move child folders to parent folder (or root if no parent)
        let parent_folder_id: Option<i64> = sqlx::query_scalar(
            "SELECT parent_folder_id FROM template_folders WHERE id = $1 AND user_id = $2"
        )
        .bind(id)
        .bind(folder_user_id)
        .fetch_optional(pool)
        .await?
        .flatten();

        sqlx::query("UPDATE template_folders SET parent_folder_id = $1 WHERE parent_folder_id = $2 AND user_id = $3")
            .bind(parent_folder_id)
            .bind(id)
            .bind(folder_user_id)
            .execute(pool)
            .await?;

        // Delete the folder
        let result = sqlx::query("DELETE FROM template_folders WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(folder_user_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn move_template_to_folder(pool: &PgPool, template_id: i64, folder_id: Option<i64>, user_id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE templates SET folder_id = $1 WHERE id = $2 AND user_id = $3"
        )
        .bind(folder_id)
        .bind(template_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_team_folders(pool: &PgPool, user_id: i64) -> Result<Vec<DbTemplateFolder>, sqlx::Error> {
        // Get user's account_id
        let account_id_result = sqlx::query("SELECT account_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
        let account_id: Option<i64> = account_id_result.try_get("account_id")?;

        // If user has account_id, get all folders in that account
        // Otherwise, only get user's own folders
        let query_str = if account_id.is_some() {
            "SELECT id, name, user_id, account_id, parent_folder_id, created_at, updated_at 
             FROM template_folders 
             WHERE account_id = $1 
             ORDER BY name ASC"
        } else {
            "SELECT id, name, user_id, account_id, parent_folder_id, created_at, updated_at 
             FROM template_folders 
             WHERE user_id = $1 
             ORDER BY name ASC"
        };

        let rows = if let Some(acc_id) = account_id {
            sqlx::query(query_str)
                .bind(acc_id)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query(query_str)
                .bind(user_id)
                .fetch_all(pool)
                .await?
        };

        let mut folders = Vec::new();
        for row in rows {
            folders.push(DbTemplateFolder {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                parent_folder_id: row.try_get("parent_folder_id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(folders)
    }

    // Get templates in a specific folder that are accessible by team members
    pub async fn get_team_templates_in_folder(pool: &PgPool, user_id: i64, folder_id: i64) -> Result<Vec<DbTemplate>, sqlx::Error> {
        // Get user's account_id
        let account_id_result = sqlx::query("SELECT account_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(pool)
            .await?;
        let account_id: Option<i64> = account_id_result.try_get("account_id")?;

        // If user has account_id, get all templates in that account and folder
        // Otherwise, only get user's own templates in that folder
        let query_str = if account_id.is_some() {
            "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at 
             FROM templates 
             WHERE account_id = $1 AND folder_id = $2
             ORDER BY created_at DESC"
        } else {
            "SELECT id, name, slug, user_id, account_id, folder_id, documents, created_at, updated_at 
             FROM templates 
             WHERE user_id = $1 AND folder_id = $2
             ORDER BY created_at DESC"
        };

        let rows = if let Some(acc_id) = account_id {
            sqlx::query(query_str)
                .bind(acc_id)
                .bind(folder_id)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query(query_str)
                .bind(user_id)
                .bind(folder_id)
                .fetch_all(pool)
                .await?
        };

        let mut templates = Vec::new();
        for row in rows {
            templates.push(DbTemplate {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                slug: row.try_get("slug")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                folder_id: row.try_get("folder_id")?,
                documents: row.try_get("documents")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(templates)
    }
}

impl TemplateFieldQueries {
    pub async fn create_template_field(pool: &PgPool, field_data: CreateTemplateField) -> Result<DbTemplateField, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO template_fields (
                template_id, name, field_type, required, display_order,
                position, options, metadata, partner, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, template_id, name, field_type, required, display_order,
                     position, options, metadata, partner, created_at, updated_at, deleted_at
            "#
        )
        .bind(field_data.template_id)
        .bind(&field_data.name)
        .bind(&field_data.field_type)
        .bind(field_data.required)
        .bind(field_data.display_order)
        .bind(&field_data.position)
        .bind(&field_data.options)
        .bind(&field_data.metadata)
        .bind(&field_data.partner)
        .bind(now)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(DbTemplateField {
            id: row.try_get("id")?,
            template_id: row.try_get("template_id")?,
            name: row.try_get("name")?,
            field_type: row.try_get("field_type")?,
            required: row.try_get("required")?,
            display_order: row.try_get("display_order")?,
            position: row.try_get("position")?,
            options: row.try_get("options")?,
            metadata: row.try_get("metadata")?,
            partner: row.try_get("partner")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            deleted_at: row.try_get("deleted_at")?,
        })
    }

    pub async fn get_template_fields(pool: &PgPool, template_id: i64) -> Result<Vec<DbTemplateField>, sqlx::Error> {
        sqlx::query_as::<_, DbTemplateField>(
            "SELECT * FROM template_fields WHERE template_id = $1 AND deleted_at IS NULL ORDER BY display_order"
        )
        .bind(template_id)
        .fetch_all(pool)
        .await
    }

    pub async fn get_all_template_fields(pool: &PgPool, template_id: i64) -> Result<Vec<DbTemplateField>, sqlx::Error> {
        sqlx::query_as::<_, DbTemplateField>(
            "SELECT * FROM template_fields WHERE template_id = $1 ORDER BY display_order"
        )
        .bind(template_id)
        .fetch_all(pool)
        .await
    }

    pub async fn get_template_field_by_id(pool: &PgPool, field_id: i64) -> Result<Option<DbTemplateField>, sqlx::Error> {
        sqlx::query_as::<_, DbTemplateField>(
            "SELECT * FROM template_fields WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(field_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn update_template_field(pool: &PgPool, field_id: i64, field_data: CreateTemplateField) -> Result<Option<DbTemplateField>, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            UPDATE template_fields SET
                name = $2, field_type = $3, required = $4, display_order = $5,
                position = $6, options = $7, metadata = $8, partner = $9, updated_at = $10
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING id, template_id, name, field_type, required, display_order,
                     position, options, metadata, partner, created_at, updated_at, deleted_at
            "#
        )
        .bind(field_id)
        .bind(&field_data.name)
        .bind(&field_data.field_type)
        .bind(field_data.required)
        .bind(field_data.display_order)
        .bind(&field_data.position)
        .bind(&field_data.options)
        .bind(&field_data.metadata)
        .bind(&field_data.partner)
        .bind(now)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbTemplateField {
                id: row.try_get("id")?,
                template_id: row.try_get("template_id")?,
                name: row.try_get("name")?,
                field_type: row.try_get("field_type")?,
                required: row.try_get("required")?,
                display_order: row.try_get("display_order")?,
                position: row.try_get("position")?,
                options: row.try_get("options")?,
                metadata: row.try_get("metadata")?,
                partner: row.try_get("partner")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                deleted_at: row.try_get("deleted_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn delete_template_field(pool: &PgPool, field_id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE template_fields SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(field_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn clone_template_fields(pool: &PgPool, from_template_id: i64, to_template_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO template_fields (
                template_id, name, field_type, required, display_order,
                position, options, metadata, partner, created_at, updated_at
            )
            SELECT
                $2 as template_id, name, field_type, required, display_order,
                position, options, metadata, partner, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
            FROM template_fields
            WHERE template_id = $1 AND deleted_at IS NULL
            "#
        )
        .bind(from_template_id)
        .bind(to_template_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_template_fields_with_positions(pool: &PgPool, template_id: i64) -> Result<Vec<DbTemplateField>, sqlx::Error> {
        sqlx::query_as::<_, DbTemplateField>(
            "SELECT * FROM template_fields 
             WHERE template_id = $1 AND position IS NOT NULL AND deleted_at IS NULL
             ORDER BY display_order"
        )
        .bind(template_id)
        .fetch_all(pool)
        .await
    }
}

impl SubmitterQueries {
    pub async fn create_submitter(pool: &PgPool, submitter_data: CreateSubmitter) -> Result<DbSubmitter, sqlx::Error> {
        let now = Utc::now();
        eprintln!("Creating submitter: template_id={}, user_id={}, name={}, email={}, token={}",
            submitter_data.template_id, submitter_data.user_id, submitter_data.name, submitter_data.email, submitter_data.token);
        let row = sqlx::query(
            "INSERT INTO submitters (template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, reminder_count, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             RETURNING id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone"
        )
        .bind(submitter_data.template_id)
        .bind(submitter_data.user_id)
        .bind(submitter_data.name)
        .bind(submitter_data.email)
        .bind(submitter_data.status)
        .bind(None as Option<DateTime<Utc>>)
        .bind(submitter_data.token)
        .bind(None as Option<serde_json::Value>) // bulk_signatures
        .bind(None as Option<String>) // ip_address
        .bind(None as Option<String>) // user_agent
        .bind(submitter_data.reminder_config) // reminder_config
        .bind(0) // reminder_count
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        eprintln!("Submitter created successfully: id={}", row.get::<i64, _>(0));
        Ok(DbSubmitter {
            id: row.get(0),
            template_id: row.get(1),
            user_id: row.get(2),
            name: row.get(3),
            email: row.get(4),
            status: row.get(5),
            signed_at: row.get(6),
            token: row.get(7),
            bulk_signatures: row.get(8),
            ip_address: row.get(9),
            user_agent: row.get(10),
            reminder_config: row.get(11),
            last_reminder_sent_at: row.get(12),
            reminder_count: row.get(13),
            created_at: row.get(14),
            updated_at: row.get(15),
            decline_reason: row.get(16),
            session_id: row.get(17),
            viewed_at: row.get(18),
            timezone: row.get(19),
            template_name: None,
        })
    }

    pub async fn get_submitters_by_template(pool: &PgPool, template_id: i64) -> Result<Vec<DbSubmitter>, sqlx::Error> {
        eprintln!("Getting submitters for template_id: {}", template_id);
        let rows = sqlx::query(
            "SELECT id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone
             FROM submitters WHERE template_id = $1 ORDER BY created_at "
        )
        .bind(template_id)
        .fetch_all(pool)
        .await?;

        eprintln!("Found {} submitters", rows.len());
        let mut submitters = Vec::new();
        for row in rows {
            submitters.push(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            });
        }
        Ok(submitters)
    }

    pub async fn get_submitters_by_user(pool: &PgPool, user_id: i64) -> Result<Vec<DbSubmitter>, sqlx::Error> {
        eprintln!("Getting submitters for user_id: {}", user_id);
        let rows = sqlx::query(
            "SELECT id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone
             FROM submitters WHERE user_id = $1 ORDER BY created_at "
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        eprintln!("Found {} submitters", rows.len());
        let mut submitters = Vec::new();
        for row in rows {
            submitters.push(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            });
        }
        Ok(submitters)
    }

    pub async fn get_submitter_by_token(pool: &PgPool, token: &str) -> Result<Option<DbSubmitter>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone
             FROM submitters WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_submitter(pool: &PgPool, id: i64, status: Option<&str>) -> Result<Option<DbSubmitter>, sqlx::Error> {
        let now = Utc::now();
        let signed_at = if status == Some("signed") { Some(now) } else { None };
        
        let row = sqlx::query(
            "UPDATE submitters SET status = COALESCE($1, status), signed_at = COALESCE($2, signed_at), updated_at = $3 
             WHERE id = $4 
             RETURNING id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone"
        )
        .bind(status)
        .bind(signed_at)
        .bind(now)
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_submitter_with_signatures(
        pool: &PgPool,
        id: i64,
        bulk_signatures: &serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        session_id: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<Option<DbSubmitter>, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            "UPDATE submitters SET bulk_signatures = $1, ip_address = $2, user_agent = $3, session_id = $4, timezone = $5, status = 'signed', signed_at = $6, updated_at = $6 
             WHERE id = $7 
             RETURNING id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone"
        )
        .bind(bulk_signatures)
        .bind(ip_address)
        .bind(user_agent)
        .bind(session_id)
        .bind(timezone)
        .bind(now)
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_submitter_with_decline_and_signatures(
        pool: &PgPool,
        id: i64,
        decline_reason: &str,
        bulk_signatures: &serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        session_id: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<Option<DbSubmitter>, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            "UPDATE submitters SET status = 'declined', decline_reason = $1, bulk_signatures = $2, ip_address = $3, user_agent = $4, session_id = $5, timezone = $6, updated_at = $7 
             WHERE id = $8 
             RETURNING id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone"
        )
        .bind(decline_reason)
        .bind(bulk_signatures)
        .bind(ip_address)
        .bind(user_agent)
        .bind(session_id)
        .bind(timezone)
        .bind(now)
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_submitter_by_id(pool: &PgPool, id: i64) -> Result<Option<DbSubmitter>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone
             FROM submitters WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            }))
        } else {
            Ok(None)
        }
    }

    // Get submitters that need reminder emails
    pub async fn get_pending_reminders(pool: &PgPool) -> Result<Vec<DbSubmitter>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT s.id, s.template_id, s.user_id, s.name, s.email, s.status, s.signed_at, s.token, s.bulk_signatures, s.ip_address, s.user_agent, s.reminder_config, s.last_reminder_sent_at, s.reminder_count, s.created_at, s.updated_at, s.decline_reason, s.session_id, s.viewed_at, s.timezone, t.name as template_name
            FROM submitters s
            LEFT JOIN templates t ON s.template_id = t.id
            WHERE s.status IN ('pending', 'sent', 'viewed')
              AND s.reminder_config IS NOT NULL
              AND s.reminder_count < 3
            ORDER BY s.created_at
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut submitters = Vec::new();
        for row in rows {
            submitters.push(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
                template_name: row.get(20),
            });
        }
        Ok(submitters)
    }

    // Update reminder status after sending
    pub async fn update_reminder_sent(pool: &PgPool, submitter_id: i64) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE submitters 
             SET last_reminder_sent_at = $1, 
                 reminder_count = reminder_count + 1,
                 updated_at = $1
             WHERE id = $2"
        )
        .bind(now)
        .bind(submitter_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete_submitter(pool: &PgPool, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM submitters WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // Get submitters accessible by team (invited users can see inviter's submitters)
    pub async fn get_team_submitters(pool: &PgPool, user_id: i64) -> Result<Vec<DbSubmitter>, sqlx::Error> {
        // Get the user's invitation info to find their team
        let team_query = sqlx::query(
            r#"
            SELECT invited_by_user_id FROM user_invitations 
            WHERE email = (SELECT email FROM users WHERE id = $1) AND is_used = TRUE
            "#
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        // Determine team members
        let team_member_ids = if let Some(row) = team_query {
            // User was invited - can see inviter's submitters and own submitters
            let invited_by: Option<i64> = row.try_get("invited_by_user_id")?;
            if let Some(inviter_id) = invited_by {
                // Get all users invited by the same inviter (team members)
                let team_rows = sqlx::query(
                    r#"
                    SELECT u.id FROM users u
                    INNER JOIN user_invitations ui ON u.email = ui.email
                    WHERE ui.invited_by_user_id = $1 AND ui.is_used = TRUE
                    UNION
                    SELECT $1 as id
                    "#
                )
                .bind(inviter_id)
                .fetch_all(pool)
                .await?;

                let mut ids: Vec<i64> = team_rows.iter()
                    .filter_map(|row| row.try_get::<i64, _>("id").ok())
                    .collect();
                ids.push(user_id); // Include current user
                ids
            } else {
                vec![user_id] // No inviter, only own submitters
            }
        } else {
            // User is admin/inviter - can see own submitters + invited users' submitters
            let invited_rows = sqlx::query(
                r#"
                SELECT u.id FROM users u
                INNER JOIN user_invitations ui ON u.email = ui.email
                WHERE ui.invited_by_user_id = $1 AND ui.is_used = TRUE
                "#
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?;

            let mut ids: Vec<i64> = invited_rows.iter()
                .filter_map(|row| row.try_get::<i64, _>("id").ok())
                .collect();
            ids.push(user_id); // Include current user (admin)
            ids
        };

        // Get submitters for all team members
        if team_member_ids.is_empty() {
            return Ok(vec![]);
        }

        let placeholders: Vec<String> = (1..=team_member_ids.len())
            .map(|i| format!("${}", i))
            .collect();
        let query_str = format!(
            "SELECT id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone
             FROM submitters 
             WHERE user_id IN ({}) 
             ORDER BY created_at DESC",
            placeholders.join(", ")
        );

        let mut query = sqlx::query(&query_str);
        for id in team_member_ids {
            query = query.bind(id);
        }

        let rows = query.fetch_all(pool).await?;

        let mut submitters = Vec::new();
        for row in rows {
            submitters.push(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            });
        }
        Ok(submitters)
    }

    pub async fn resubmit_submitter(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE submitters SET status = 'pending' WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_submitters_by_template_id(pool: &PgPool, template_id: i64) -> Result<Vec<DbSubmitter>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone
             FROM submitters WHERE template_id = $1"
        )
        .bind(template_id)
        .fetch_all(pool)
        .await?;

        let mut submitters = Vec::new();
        for row in rows {
            submitters.push(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            });
        }
        Ok(submitters)
    }
}

impl SubmissionFieldQueries {
    pub async fn create_submission_field(pool: &PgPool, field_data: CreateSubmissionField) -> Result<DbSubmissionField, sqlx::Error> {
        let now = Utc::now();
        let row = sqlx::query(
            "INSERT INTO submission_fields (submitter_id, template_field_id, name, field_type, required, display_order, position, options, metadata, partner, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING id, submitter_id, template_field_id, name, field_type, required, display_order, position, options, metadata, partner, created_at, updated_at"
        )
        .bind(field_data.submitter_id)
        .bind(field_data.template_field_id)
        .bind(field_data.name)
        .bind(field_data.field_type)
        .bind(field_data.required)
        .bind(field_data.display_order)
        .bind(field_data.position)
        .bind(field_data.options)
        .bind(field_data.metadata)
        .bind(field_data.partner)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(DbSubmissionField {
            id: row.get(0),
            submitter_id: row.get(1),
            template_field_id: row.get(2),
            name: row.get(3),
            field_type: row.get(4),
            required: row.get(5),
            display_order: row.get(6),
            position: row.get(7),
            options: row.get(8),
            metadata: row.get(9),
            partner: row.get(10),
            created_at: row.get(11),
            updated_at: row.get(12),
        })
    }

    pub async fn get_submission_fields_by_submitter_id(pool: &PgPool, submitter_id: i64) -> Result<Vec<DbSubmissionField>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, submitter_id, template_field_id, name, field_type, required, display_order, position, options, metadata, partner, created_at, updated_at
             FROM submission_fields WHERE submitter_id = $1 ORDER BY display_order"
        )
        .bind(submitter_id)
        .fetch_all(pool)
        .await?;

        let mut fields = Vec::new();
        for row in rows {
            fields.push(DbSubmissionField {
                id: row.get(0),
                submitter_id: row.get(1),
                template_field_id: row.get(2),
                name: row.get(3),
                field_type: row.get(4),
                required: row.get(5),
                display_order: row.get(6),
                position: row.get(7),
                options: row.get(8),
                metadata: row.get(9),
                partner: row.get(10),
                created_at: row.get(11),
                updated_at: row.get(12),
            });
        }
        Ok(fields)
    }
}

pub struct SignatureQueries;

impl SignatureQueries {






    // DEPRECATED: This function is not used with bulk signatures
    // pub async fn get_signature_history_by_field(
    //     pool: &PgPool,
    //     submitter_id: i64,
    //     field_name: &str,
    // ) -> Result<Vec<DbSignaturePosition>, sqlx::Error> {
    //     // Now we use bulk_signatures instead
    //     Ok(Vec::new())
    // }

    pub async fn get_submitter_with_signatures(
        pool: &PgPool,
        submitter_id: i64,
    ) -> Result<Option<DbSubmitter>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, template_id, user_id, name, email, status, signed_at, token, bulk_signatures, ip_address, user_agent, reminder_config, last_reminder_sent_at, reminder_count, created_at, updated_at, decline_reason, session_id, viewed_at, timezone
            FROM submitters
            WHERE id = $1 AND bulk_signatures IS NOT NULL
            "#
        )
        .bind(submitter_id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbSubmitter {
                id: row.get(0),
                template_id: row.get(1),
                user_id: row.get(2),
                name: row.get(3),
                email: row.get(4),
                status: row.get(5),
                signed_at: row.get(6),
                token: row.get(7),
                bulk_signatures: row.get(8),
                ip_address: row.get(9),
                user_agent: row.get(10),
                reminder_config: row.get(11),
                last_reminder_sent_at: row.get(12),
                reminder_count: row.get(13),
                created_at: row.get(14),
                updated_at: row.get(15),
                decline_reason: row.get(16),
                session_id: row.get(17),
                viewed_at: row.get(18),
                timezone: row.get(19),
            template_name: None,
            })),
            None => Ok(None),
        }
    }


    pub async fn get_signature_data_by_submitter(
        pool: &PgPool,
        submitter_id: i64,
    ) -> Result<Option<DbSignatureData>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, submitter_id, signature_image, signature_value, signed_at, ip_address, user_agent
            FROM signature_data
            WHERE submitter_id = $1
            ORDER BY signed_at DESC
            LIMIT 1
            "#
        )
        .bind(submitter_id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(DbSignatureData {
                id: row.try_get("id")?,
                submitter_id: row.try_get("submitter_id")?,
                signature_value: row.try_get("signature_value")?,
                signed_at: row.try_get("signed_at")?,
                ip_address: row.try_get("ip_address")?,
                user_agent: row.try_get("user_agent")?,
            }))
        } else {
            Ok(None)
        }
    }
}

// User Reminder Settings Queries
pub struct UserReminderSettingsQueries;

impl UserReminderSettingsQueries {
    // Get user reminder settings
    pub async fn get_by_user_id(pool: &PgPool, user_id: i64) -> Result<Option<super::models::DbUserReminderSettings>, sqlx::Error> {
        let row = sqlx::query_as::<_, super::models::DbUserReminderSettings>(
            "SELECT id, user_id, first_reminder_hours, second_reminder_hours, third_reminder_hours, receive_notification_on_completion, completion_notification_email, created_at, updated_at 
             FROM user_reminder_settings WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    // Create default reminder settings for new user
    pub async fn create(pool: &PgPool, settings_data: super::models::CreateUserReminderSettings) -> Result<super::models::DbUserReminderSettings, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query_as::<_, super::models::DbUserReminderSettings>(
            r#"
            INSERT INTO user_reminder_settings (user_id, first_reminder_hours, second_reminder_hours, third_reminder_hours, receive_notification_on_completion, completion_notification_email, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, first_reminder_hours, second_reminder_hours, third_reminder_hours, receive_notification_on_completion, completion_notification_email, created_at, updated_at
            "#
        )
        .bind(settings_data.user_id)
        .bind(settings_data.first_reminder_hours)
        .bind(settings_data.second_reminder_hours)
        .bind(settings_data.third_reminder_hours)
        .bind(settings_data.receive_notification_on_completion)
        .bind(settings_data.completion_notification_email)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    // Update user reminder settings
    pub async fn update(pool: &PgPool, user_id: i64, update_data: super::models::UpdateUserReminderSettings) -> Result<Option<super::models::DbUserReminderSettings>, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query_as::<_, super::models::DbUserReminderSettings>(
            r#"
            UPDATE user_reminder_settings 
            SET first_reminder_hours = COALESCE($1, first_reminder_hours),
                second_reminder_hours = COALESCE($2, second_reminder_hours),
                third_reminder_hours = COALESCE($3, third_reminder_hours),
                receive_notification_on_completion = $4,
                completion_notification_email = $5,
                updated_at = $6
            WHERE user_id = $7
            RETURNING id, user_id, first_reminder_hours, second_reminder_hours, third_reminder_hours, receive_notification_on_completion, completion_notification_email, created_at, updated_at
            "#
        )
        .bind(update_data.first_reminder_hours)
        .bind(update_data.second_reminder_hours)
        .bind(update_data.third_reminder_hours)
        .bind(update_data.receive_notification_on_completion)
        .bind(update_data.completion_notification_email)
        .bind(now)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    // Create default settings if not exists (helper for when creating submitters)
    pub async fn get_or_create_default(pool: &PgPool, user_id: i64) -> Result<super::models::DbUserReminderSettings, sqlx::Error> {
        // Try to get existing settings
        if let Some(settings) = Self::get_by_user_id(pool, user_id).await? {
            return Ok(settings);
        }

        // Create default settings (all NULL - user must configure)
        let default_settings = super::models::CreateUserReminderSettings {
            user_id,
            first_reminder_hours: Some(1),   // Default 1 minute for testing
            second_reminder_hours: Some(2),  // Default 2 minutes for testing
            third_reminder_hours: Some(3),   // Default 3 minutes for testing
            receive_notification_on_completion: None, // User must set
            completion_notification_email: None, // User must set
        };

        Self::create(pool, default_settings).await
    }
}

// Simplified subscription-related queries
pub struct SubscriptionQueries;

impl SubscriptionQueries {
    // Create payment record
    pub async fn create_payment_record(pool: &PgPool, payment_data: CreatePaymentRecord) -> Result<DbPaymentRecord, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query_as::<_, DbPaymentRecord>(
            r#"
            INSERT INTO payment_records (user_id, stripe_session_id, amount_cents, currency, status, metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(&payment_data.user_id)
        .bind(&payment_data.stripe_session_id)
        .bind(&payment_data.amount_cents)
        .bind(&payment_data.currency)
        .bind(&payment_data.status)
        .bind(&payment_data.metadata)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }


    // Update user subscription status sau khi thanh toán thành công
    pub async fn update_user_subscription_status(pool: &PgPool, user_id: i64, status: &str, expires_at: Option<DateTime<Utc>>) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE users SET subscription_status = $1, subscription_expires_at = $2, updated_at = $3 WHERE id = $4"
        )
        .bind(status)
        .bind(expires_at)
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    // Increment user free usage count
    pub async fn increment_user_usage(pool: &PgPool, user_id: i64) -> Result<i32, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE users SET free_usage_count = free_usage_count + 1, updated_at = NOW() WHERE id = $1 RETURNING free_usage_count"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        Ok(row.try_get("free_usage_count")?)
    }

    pub async fn increment_user_usage_by(pool: &PgPool, user_id: i64, count: i32) -> Result<i32, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE users SET free_usage_count = free_usage_count + $2, updated_at = NOW() WHERE id = $1 RETURNING free_usage_count"
        )
        .bind(user_id)
        .bind(count)
        .fetch_one(pool)
        .await?;

        Ok(row.try_get("free_usage_count")?)
    }

    // Get user subscription status
    pub async fn get_user_subscription_status(pool: &PgPool, user_id: i64) -> Result<Option<DbUser>, sqlx::Error> {
        let row = sqlx::query_as::<_, DbUser>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

}

pub struct OAuthTokenQueries;

impl OAuthTokenQueries {
    pub async fn get_oauth_token(pool: &PgPool, user_id: i64, provider: &str) -> Result<Option<super::models::DbOAuthToken>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, user_id, provider, access_token, refresh_token, expires_at, created_at, updated_at FROM oauth_tokens WHERE user_id = $1 AND provider = $2 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(user_id)
        .bind(provider)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(super::models::DbOAuthToken {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                provider: row.try_get("provider")?,
                access_token: row.try_get("access_token")?,
                refresh_token: row.try_get("refresh_token")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn create_oauth_token(pool: &PgPool, token_data: super::models::CreateOAuthToken) -> Result<super::models::DbOAuthToken, sqlx::Error> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO oauth_tokens (user_id, provider, access_token, refresh_token, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, user_id, provider, access_token, refresh_token, expires_at, created_at, updated_at
            "#
        )
        .bind(token_data.user_id)
        .bind(&token_data.provider)
        .bind(&token_data.access_token)
        .bind(&token_data.refresh_token)
        .bind(token_data.expires_at)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(super::models::DbOAuthToken {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            provider: row.try_get("provider")?,
            access_token: row.try_get("access_token")?,
            refresh_token: row.try_get("refresh_token")?,
            expires_at: row.try_get("expires_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }


    pub async fn update_oauth_token(pool: &PgPool, user_id: i64, provider: &str, access_token: &str, refresh_token: Option<&str>, expires_at: Option<DateTime<Utc>>) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE oauth_tokens SET access_token = $1, refresh_token = $2, expires_at = $3, updated_at = $4 WHERE user_id = $5 AND provider = $6"
        )
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .bind(now)
        .bind(user_id)
        .bind(provider)
        .execute(pool)
        .await?;

        Ok(())
    }
}

impl GlobalSettingsQueries {
    pub async fn get_global_settings(pool: &PgPool) -> Result<Option<DbGlobalSettings>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, user_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at FROM global_settings WHERE user_id IS NULL"
        )
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbGlobalSettings {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                account_id: None,
                company_name: row.try_get("company_name")?,
                timezone: row.try_get("timezone")?,
                locale: row.try_get("locale")?,
                logo_url: row.try_get("logo_url")?,
                force_2fa_with_authenticator_app: row.try_get("force_2fa_with_authenticator_app")?,
                add_signature_id_to_the_documents: row.try_get("add_signature_id_to_the_documents")?,
                require_signing_reason: row.try_get("require_signing_reason")?,
                allow_typed_text_signatures: row.try_get("allow_typed_text_signatures")?,
                allow_to_resubmit_completed_forms: row.try_get("allow_to_resubmit_completed_forms")?,
                allow_to_decline_documents: row.try_get("allow_to_decline_documents")?,
                remember_and_pre_fill_signatures: row.try_get("remember_and_pre_fill_signatures")?,
                require_authentication_for_file_download_links: row.try_get("require_authentication_for_file_download_links")?,
                combine_completed_documents_and_audit_log: row.try_get("combine_completed_documents_and_audit_log")?,
                expirable_file_download_links: row.try_get("expirable_file_download_links")?,
                enable_confetti: row.try_get("enable_confetti")?,
                completion_title: row.try_get("completion_title")?,
                completion_body: row.try_get("completion_body")?,
                redirect_title: row.try_get("redirect_title")?,
                redirect_url: row.try_get("redirect_url")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_user_settings(pool: &PgPool, user_id: i32) -> Result<Option<DbGlobalSettings>, sqlx::Error> {
        // First get user's account_id
        let user = UserQueries::get_user_by_id(pool, user_id as i64).await?;
        let account_id = match user {
            Some(u) => u.account_id,
            None => return Ok(None),
        };

        // Query settings by account_id (if has account) or user_id (if no account)
        let row = if let Some(acc_id) = account_id {
            sqlx::query(
                "SELECT id, user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at FROM global_settings WHERE account_id = $1"
            )
            .bind(acc_id)
            .fetch_optional(pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at FROM global_settings WHERE user_id = $1"
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await?
        };

        match row {
            Some(row) => Ok(Some(DbGlobalSettings {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                company_name: row.try_get("company_name")?,
                timezone: row.try_get("timezone")?,
                locale: row.try_get("locale")?,
                logo_url: row.try_get("logo_url")?,
                force_2fa_with_authenticator_app: row.try_get("force_2fa_with_authenticator_app")?,
                add_signature_id_to_the_documents: row.try_get("add_signature_id_to_the_documents")?,
                require_signing_reason: row.try_get("require_signing_reason")?,
                allow_typed_text_signatures: row.try_get("allow_typed_text_signatures")?,
                allow_to_resubmit_completed_forms: row.try_get("allow_to_resubmit_completed_forms")?,
                allow_to_decline_documents: row.try_get("allow_to_decline_documents")?,
                remember_and_pre_fill_signatures: row.try_get("remember_and_pre_fill_signatures")?,
                require_authentication_for_file_download_links: row.try_get("require_authentication_for_file_download_links")?,
                combine_completed_documents_and_audit_log: row.try_get("combine_completed_documents_and_audit_log")?,
                expirable_file_download_links: row.try_get("expirable_file_download_links")?,
                enable_confetti: row.try_get("enable_confetti")?,
                completion_title: row.try_get("completion_title")?,
                completion_body: row.try_get("completion_body")?,
                redirect_title: row.try_get("redirect_title")?,
                redirect_url: row.try_get("redirect_url")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn create_user_settings(pool: &PgPool, user_id: i32) -> Result<DbGlobalSettings, sqlx::Error> {
        let now = Utc::now();

        // Get user's account_id
        let user = UserQueries::get_user_by_id(pool, user_id as i64).await?;
        let account_id = match user {
            Some(u) => u.account_id,
            None => return Err(sqlx::Error::RowNotFound),
        };

        // Check if settings already exist for this account (if user has account)
        if let Some(acc_id) = account_id {
            let query_str = "SELECT id, user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at FROM global_settings WHERE account_id = $1";
            
            if let Some(existing) = sqlx::query(query_str)
                .bind(acc_id)
                .fetch_optional(pool)
                .await? {
                return Ok(DbGlobalSettings {
                    id: existing.try_get("id")?,
                    user_id: existing.try_get("user_id")?,
                    account_id: existing.try_get("account_id")?,
                    company_name: existing.try_get("company_name")?,
                    timezone: existing.try_get("timezone")?,
                    locale: existing.try_get("locale")?,
                    logo_url: existing.try_get("logo_url")?,
                    force_2fa_with_authenticator_app: existing.try_get("force_2fa_with_authenticator_app")?,
                    add_signature_id_to_the_documents: existing.try_get("add_signature_id_to_the_documents")?,
                    require_signing_reason: existing.try_get("require_signing_reason")?,
                    allow_typed_text_signatures: existing.try_get("allow_typed_text_signatures")?,
                    allow_to_resubmit_completed_forms: existing.try_get("allow_to_resubmit_completed_forms")?,
                    allow_to_decline_documents: existing.try_get("allow_to_decline_documents")?,
                    remember_and_pre_fill_signatures: existing.try_get("remember_and_pre_fill_signatures")?,
                    require_authentication_for_file_download_links: existing.try_get("require_authentication_for_file_download_links")?,
                    combine_completed_documents_and_audit_log: existing.try_get("combine_completed_documents_and_audit_log")?,
                    expirable_file_download_links: existing.try_get("expirable_file_download_links")?,
                    enable_confetti: existing.try_get("enable_confetti")?,
                    completion_title: existing.try_get("completion_title")?,
                    completion_body: existing.try_get("completion_body")?,
                    redirect_title: existing.try_get("redirect_title")?,
                    redirect_url: existing.try_get("redirect_url")?,
                    created_at: existing.try_get("created_at")?,
                    updated_at: existing.try_get("updated_at")?,
                });
            }
        } else {
            // Check if settings already exist for this user (if no account)
            let query_str = "SELECT id, user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at FROM global_settings WHERE user_id = $1";
            
            if let Some(existing) = sqlx::query(query_str)
                .bind(user_id)
                .fetch_optional(pool)
                .await? {
                return Ok(DbGlobalSettings {
                    id: existing.try_get("id")?,
                    user_id: existing.try_get("user_id")?,
                    account_id: existing.try_get("account_id")?,
                    company_name: existing.try_get("company_name")?,
                    timezone: existing.try_get("timezone")?,
                    locale: existing.try_get("locale")?,
                    logo_url: existing.try_get("logo_url")?,
                    force_2fa_with_authenticator_app: existing.try_get("force_2fa_with_authenticator_app")?,
                    add_signature_id_to_the_documents: existing.try_get("add_signature_id_to_the_documents")?,
                    require_signing_reason: existing.try_get("require_signing_reason")?,
                    allow_typed_text_signatures: existing.try_get("allow_typed_text_signatures")?,
                    allow_to_resubmit_completed_forms: existing.try_get("allow_to_resubmit_completed_forms")?,
                    allow_to_decline_documents: existing.try_get("allow_to_decline_documents")?,
                    remember_and_pre_fill_signatures: existing.try_get("remember_and_pre_fill_signatures")?,
                    require_authentication_for_file_download_links: existing.try_get("require_authentication_for_file_download_links")?,
                    combine_completed_documents_and_audit_log: existing.try_get("combine_completed_documents_and_audit_log")?,
                    expirable_file_download_links: existing.try_get("expirable_file_download_links")?,
                    enable_confetti: existing.try_get("enable_confetti")?,
                    completion_title: existing.try_get("completion_title")?,
                    completion_body: existing.try_get("completion_body")?,
                    redirect_title: existing.try_get("redirect_title")?,
                    redirect_url: existing.try_get("redirect_url")?,
                    created_at: existing.try_get("created_at")?,
                    updated_at: existing.try_get("updated_at")?,
                });
            }
        }

        // Create new settings
        let row = sqlx::query(
            r#"
            INSERT INTO global_settings (user_id, account_id, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, created_at, updated_at)
            VALUES ($1, $2, false, false, false, true, false, false, false, false, false, false, false, $3, $3)
            RETURNING id, user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, NULL as completion_title, NULL as completion_body, NULL as redirect_title, NULL as redirect_url, created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(account_id)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(DbGlobalSettings {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            account_id: row.try_get("account_id")?,
            company_name: row.try_get("company_name")?,
            timezone: row.try_get("timezone")?,
            locale: row.try_get("locale")?,
            logo_url: row.try_get("logo_url")?,
            force_2fa_with_authenticator_app: row.try_get("force_2fa_with_authenticator_app")?,
            add_signature_id_to_the_documents: row.try_get("add_signature_id_to_the_documents")?,
            require_signing_reason: row.try_get("require_signing_reason")?,
            allow_typed_text_signatures: row.try_get("allow_typed_text_signatures")?,
            allow_to_resubmit_completed_forms: row.try_get("allow_to_resubmit_completed_forms")?,
            allow_to_decline_documents: row.try_get("allow_to_decline_documents")?,
            remember_and_pre_fill_signatures: row.try_get("remember_and_pre_fill_signatures")?,
            require_authentication_for_file_download_links: row.try_get("require_authentication_for_file_download_links")?,
            combine_completed_documents_and_audit_log: row.try_get("combine_completed_documents_and_audit_log")?,
            expirable_file_download_links: row.try_get("expirable_file_download_links")?,
            enable_confetti: row.try_get("enable_confetti")?,
            completion_title: row.try_get("completion_title")?,
            completion_body: row.try_get("completion_body")?,
            redirect_title: row.try_get("redirect_title")?,
            redirect_url: row.try_get("redirect_url")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn update_global_settings(pool: &PgPool, settings: UpdateGlobalSettings) -> Result<DbGlobalSettings, sqlx::Error> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE global_settings
            SET company_name = $1, timezone = $2, locale = $3, logo_url = $4,
                force_2fa_with_authenticator_app = $5, add_signature_id_to_the_documents = $6, 
                require_signing_reason = $7, allow_typed_text_signatures = $8, 
                allow_to_resubmit_completed_forms = $9, allow_to_decline_documents = $10, 
                remember_and_pre_fill_signatures = $11, require_authentication_for_file_download_links = $12, 
                combine_completed_documents_and_audit_log = $13, expirable_file_download_links = $14,
                enable_confetti = $15,
                updated_at = $16
            WHERE user_id IS NULL
            "#
        )
        .bind(settings.company_name)
        .bind(settings.timezone)
        .bind(settings.locale)
        .bind(settings.logo_url)
        .bind(settings.force_2fa_with_authenticator_app)
        .bind(settings.add_signature_id_to_the_documents)
        .bind(settings.require_signing_reason)
        .bind(settings.allow_typed_text_signatures)
        .bind(settings.allow_to_resubmit_completed_forms)
        .bind(settings.allow_to_decline_documents)
        .bind(settings.remember_and_pre_fill_signatures)
        .bind(settings.require_authentication_for_file_download_links)
        .bind(settings.combine_completed_documents_and_audit_log)
        .bind(settings.expirable_file_download_links)
        .bind(settings.enable_confetti)
        .bind(now)
        .execute(pool)
        .await?;

        // Return the updated settings
        Self::get_global_settings(pool).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn update_user_settings(pool: &PgPool, user_id: i32, settings: UpdateGlobalSettings) -> Result<DbGlobalSettings, sqlx::Error> {
        let now = Utc::now();

        // Get user's account_id
        let user = UserQueries::get_user_by_id(pool, user_id as i64).await?;
        let account_id = match user {
            Some(u) => u.account_id,
            None => return Err(sqlx::Error::RowNotFound),
        };

        // First check if settings exist for this user_id or account_id
        let existing_settings = if let Some(acc_id) = account_id {
            sqlx::query_as::<_, (i32,)>("SELECT id FROM global_settings WHERE account_id = $1")
                .bind(acc_id)
                .fetch_optional(pool)
                .await?
        } else {
            // For users without account_id, check by user_id
            sqlx::query_as::<_, (i32,)>("SELECT id FROM global_settings WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(pool)
                .await?
        };

        if existing_settings.is_none() {
            // Create settings - for users with account_id, use ON CONFLICT on account_id
            // For users without account_id, just INSERT (no conflict possible except id)
            let row = if account_id.is_some() {
                sqlx::query(
                    r#"
                    INSERT INTO global_settings (user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
                    ON CONFLICT (account_id) DO UPDATE SET
                        company_name = COALESCE(EXCLUDED.company_name, global_settings.company_name),
                        timezone = COALESCE(EXCLUDED.timezone, global_settings.timezone),
                        locale = COALESCE(EXCLUDED.locale, global_settings.locale),
                        logo_url = EXCLUDED.logo_url,
                        force_2fa_with_authenticator_app = COALESCE(EXCLUDED.force_2fa_with_authenticator_app, global_settings.force_2fa_with_authenticator_app),
                        add_signature_id_to_the_documents = COALESCE(EXCLUDED.add_signature_id_to_the_documents, global_settings.add_signature_id_to_the_documents),
                        require_signing_reason = COALESCE(EXCLUDED.require_signing_reason, global_settings.require_signing_reason),
                        allow_typed_text_signatures = COALESCE(EXCLUDED.allow_typed_text_signatures, global_settings.allow_typed_text_signatures),
                        allow_to_resubmit_completed_forms = COALESCE(EXCLUDED.allow_to_resubmit_completed_forms, global_settings.allow_to_resubmit_completed_forms),
                        allow_to_decline_documents = COALESCE(EXCLUDED.allow_to_decline_documents, global_settings.allow_to_decline_documents),
                        remember_and_pre_fill_signatures = COALESCE(EXCLUDED.remember_and_pre_fill_signatures, global_settings.remember_and_pre_fill_signatures),
                        require_authentication_for_file_download_links = COALESCE(EXCLUDED.require_authentication_for_file_download_links, global_settings.require_authentication_for_file_download_links),
                        combine_completed_documents_and_audit_log = COALESCE(EXCLUDED.combine_completed_documents_and_audit_log, global_settings.combine_completed_documents_and_audit_log),
                        expirable_file_download_links = COALESCE(EXCLUDED.expirable_file_download_links, global_settings.expirable_file_download_links),
                        enable_confetti = COALESCE(EXCLUDED.enable_confetti, global_settings.enable_confetti),
                        completion_title = COALESCE(EXCLUDED.completion_title, global_settings.completion_title),
                        completion_body = COALESCE(EXCLUDED.completion_body, global_settings.completion_body),
                        redirect_title = COALESCE(EXCLUDED.redirect_title, global_settings.redirect_title),
                        redirect_url = COALESCE(EXCLUDED.redirect_url, global_settings.redirect_url),
                        updated_at = EXCLUDED.updated_at
                    RETURNING id, user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at
                    "#
                )
                .bind(user_id)
                .bind(account_id)
                .bind(settings.company_name.as_deref())
                .bind(settings.timezone.as_deref())
                .bind(settings.locale.as_deref())
                .bind(settings.logo_url.as_deref())
                .bind(settings.force_2fa_with_authenticator_app.unwrap_or(false))
                .bind(settings.add_signature_id_to_the_documents.unwrap_or(false))
                .bind(settings.require_signing_reason.unwrap_or(false))
                .bind(settings.allow_typed_text_signatures.unwrap_or(true))
                .bind(settings.allow_to_resubmit_completed_forms.unwrap_or(false))
                .bind(settings.allow_to_decline_documents.unwrap_or(false))
                .bind(settings.remember_and_pre_fill_signatures.unwrap_or(false))
                .bind(settings.require_authentication_for_file_download_links.unwrap_or(false))
                .bind(settings.combine_completed_documents_and_audit_log.unwrap_or(false))
                .bind(settings.expirable_file_download_links.unwrap_or(false))
                .bind(settings.enable_confetti.unwrap_or(false))
                .bind(settings.completion_title.as_deref())
                .bind(settings.completion_body.as_deref())
                .bind(settings.redirect_title.as_deref())
                .bind(settings.redirect_url.as_deref())
                .bind(now)
                .fetch_one(pool)
                .await?
            } else {
                // For users without account_id, just INSERT normally
                sqlx::query(
                    r#"
                    INSERT INTO global_settings (user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
                    RETURNING id, user_id, account_id, company_name, timezone, locale, logo_url, force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, remember_and_pre_fill_signatures, require_authentication_for_file_download_links, combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti, completion_title, completion_body, redirect_title, redirect_url, created_at, updated_at
                    "#
                )
                .bind(user_id)
                .bind(account_id)
                .bind(settings.company_name.as_deref())
                .bind(settings.timezone.as_deref())
                .bind(settings.locale.as_deref())
                .bind(settings.logo_url.as_deref())
                .bind(settings.force_2fa_with_authenticator_app.unwrap_or(false))
                .bind(settings.add_signature_id_to_the_documents.unwrap_or(false))
                .bind(settings.require_signing_reason.unwrap_or(false))
                .bind(settings.allow_typed_text_signatures.unwrap_or(true))
                .bind(settings.allow_to_resubmit_completed_forms.unwrap_or(false))
                .bind(settings.allow_to_decline_documents.unwrap_or(false))
                .bind(settings.remember_and_pre_fill_signatures.unwrap_or(false))
                .bind(settings.require_authentication_for_file_download_links.unwrap_or(false))
                .bind(settings.combine_completed_documents_and_audit_log.unwrap_or(false))
                .bind(settings.expirable_file_download_links.unwrap_or(false))
                .bind(settings.enable_confetti.unwrap_or(false))
                .bind(settings.completion_title.as_deref())
                .bind(settings.completion_body.as_deref())
                .bind(settings.redirect_title.as_deref())
                .bind(settings.redirect_url.as_deref())
                .bind(now)
                .fetch_one(pool)
                .await?
            };

            return Ok(DbGlobalSettings {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                account_id: row.try_get("account_id")?,
                company_name: row.try_get("company_name")?,
                timezone: row.try_get("timezone")?,
                locale: row.try_get("locale")?,
                logo_url: row.try_get("logo_url")?,
                force_2fa_with_authenticator_app: row.try_get("force_2fa_with_authenticator_app")?,
                add_signature_id_to_the_documents: row.try_get("add_signature_id_to_the_documents")?,
                require_signing_reason: row.try_get("require_signing_reason")?,
                allow_typed_text_signatures: row.try_get("allow_typed_text_signatures")?,
                allow_to_resubmit_completed_forms: row.try_get("allow_to_resubmit_completed_forms")?,
                allow_to_decline_documents: row.try_get("allow_to_decline_documents")?,
                remember_and_pre_fill_signatures: row.try_get("remember_and_pre_fill_signatures")?,
                require_authentication_for_file_download_links: row.try_get("require_authentication_for_file_download_links")?,
                combine_completed_documents_and_audit_log: row.try_get("combine_completed_documents_and_audit_log")?,
                expirable_file_download_links: row.try_get("expirable_file_download_links")?,
                enable_confetti: row.try_get("enable_confetti")?,
                completion_title: row.try_get("completion_title")?,
                completion_body: row.try_get("completion_body")?,
                redirect_title: row.try_get("redirect_title")?,
                redirect_url: row.try_get("redirect_url")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        // Update existing settings by account_id (if has account) or user_id (if no account)
        let update_query = if account_id.is_some() {
            r#"
            UPDATE global_settings
            SET company_name = COALESCE($1, company_name),
                timezone = COALESCE($2, timezone),
                locale = COALESCE($3, locale),
                logo_url = $4,
                force_2fa_with_authenticator_app = COALESCE($5, force_2fa_with_authenticator_app),
                add_signature_id_to_the_documents = COALESCE($6, add_signature_id_to_the_documents),
                require_signing_reason = COALESCE($7, require_signing_reason),
                allow_typed_text_signatures = COALESCE($8, allow_typed_text_signatures),
                allow_to_resubmit_completed_forms = COALESCE($9, allow_to_resubmit_completed_forms),
                allow_to_decline_documents = COALESCE($10, allow_to_decline_documents),
                remember_and_pre_fill_signatures = COALESCE($11, remember_and_pre_fill_signatures),
                require_authentication_for_file_download_links = COALESCE($12, require_authentication_for_file_download_links),
                combine_completed_documents_and_audit_log = COALESCE($13, combine_completed_documents_and_audit_log),
                expirable_file_download_links = COALESCE($14, expirable_file_download_links),
                enable_confetti = COALESCE($15, enable_confetti),
                completion_title = COALESCE($16, completion_title),
                completion_body = COALESCE($17, completion_body),
                redirect_title = COALESCE($18, redirect_title),
                redirect_url = COALESCE($19, redirect_url),
                updated_at = $20
            WHERE account_id = $21
            "#
        } else {
            r#"
            UPDATE global_settings
            SET company_name = COALESCE($1, company_name),
                timezone = COALESCE($2, timezone),
                locale = COALESCE($3, locale),
                logo_url = $4,
                force_2fa_with_authenticator_app = COALESCE($5, force_2fa_with_authenticator_app),
                add_signature_id_to_the_documents = COALESCE($6, add_signature_id_to_the_documents),
                require_signing_reason = COALESCE($7, require_signing_reason),
                allow_typed_text_signatures = COALESCE($8, allow_typed_text_signatures),
                allow_to_resubmit_completed_forms = COALESCE($9, allow_to_resubmit_completed_forms),
                allow_to_decline_documents = COALESCE($10, allow_to_decline_documents),
                remember_and_pre_fill_signatures = COALESCE($11, remember_and_pre_fill_signatures),
                require_authentication_for_file_download_links = COALESCE($12, require_authentication_for_file_download_links),
                combine_completed_documents_and_audit_log = COALESCE($13, combine_completed_documents_and_audit_log),
                expirable_file_download_links = COALESCE($14, expirable_file_download_links),
                enable_confetti = COALESCE($15, enable_confetti),
                completion_title = COALESCE($16, completion_title),
                completion_body = COALESCE($17, completion_body),
                redirect_title = COALESCE($18, redirect_title),
                redirect_url = COALESCE($19, redirect_url),
                updated_at = $20
            WHERE user_id = $21
            "#
        };
        
        let mut query = sqlx::query(update_query);
        
        query = query.bind(settings.company_name.as_deref());
        query = query.bind(settings.timezone.as_deref());
        query = query.bind(settings.locale.as_deref());
        query = query.bind(settings.logo_url.as_deref());
        query = query.bind(settings.force_2fa_with_authenticator_app);
        query = query.bind(settings.add_signature_id_to_the_documents);
        query = query.bind(settings.require_signing_reason);
        query = query.bind(settings.allow_typed_text_signatures);
        query = query.bind(settings.allow_to_resubmit_completed_forms);
        query = query.bind(settings.allow_to_decline_documents);
        query = query.bind(settings.remember_and_pre_fill_signatures);
        query = query.bind(settings.require_authentication_for_file_download_links);
        query = query.bind(settings.combine_completed_documents_and_audit_log);
        query = query.bind(settings.expirable_file_download_links);
        query = query.bind(settings.enable_confetti);
        query = query.bind(settings.completion_title.as_deref());
        query = query.bind(settings.completion_body.as_deref());
        query = query.bind(settings.redirect_title.as_deref());
        query = query.bind(settings.redirect_url.as_deref());
        query = query.bind(now);
        
        // Bind WHERE clause condition (account_id or user_id)
        if let Some(acc_id) = account_id {
            query = query.bind(acc_id);
        } else {
            query = query.bind(user_id);
        }
        
        query.execute(pool).await?;

        // Return the updated settings
        Self::get_user_settings(pool, user_id).await?.ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn create_default_global_settings(pool: &PgPool) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO global_settings (id, company_name, timezone, locale, 
                force_2fa_with_authenticator_app, add_signature_id_to_the_documents, require_signing_reason, 
                allow_typed_text_signatures, allow_to_resubmit_completed_forms, allow_to_decline_documents, 
                remember_and_pre_fill_signatures, require_authentication_for_file_download_links, 
                combine_completed_documents_and_audit_log, expirable_file_download_links, enable_confetti,
                created_at, updated_at)
            VALUES (1, 'DocuSeal', 'UTC', 'en-US', 
                FALSE, FALSE, FALSE, TRUE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE, FALSE,
                $1, $2)
            ON CONFLICT (id) DO NOTHING
            "#
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(())
    }
}

impl EmailTemplateQueries {
    pub async fn get_templates_by_user(pool: &PgPool, user_id: i64) -> Result<Vec<DbEmailTemplate>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log, created_at, updated_at FROM email_templates WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(DbEmailTemplate {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                template_type: row.try_get("template_type")?,
                subject: row.try_get("subject")?,
                body: row.try_get("body")?,
                body_format: row.try_get("body_format")?,
                is_default: row.try_get("is_default")?,
                attach_documents: row.try_get("attach_documents")?,
                attach_audit_log: row.try_get("attach_audit_log")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(templates)
    }

    pub async fn get_templates_by_type(pool: &PgPool, user_id: i64, template_type: &str) -> Result<Vec<DbEmailTemplate>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log, created_at, updated_at FROM email_templates WHERE user_id = $1 AND template_type = $2 ORDER BY is_default DESC, created_at DESC"
        )
        .bind(user_id)
        .bind(template_type)
        .fetch_all(pool)
        .await?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(DbEmailTemplate {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                template_type: row.try_get("template_type")?,
                subject: row.try_get("subject")?,
                body: row.try_get("body")?,
                body_format: row.try_get("body_format")?,
                is_default: row.try_get("is_default")?,
                attach_documents: row.try_get("attach_documents")?,
                attach_audit_log: row.try_get("attach_audit_log")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }
        Ok(templates)
    }

    pub async fn get_default_template_by_type(pool: &PgPool, user_id: i64, template_type: &str) -> Result<Option<DbEmailTemplate>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log, created_at, updated_at FROM email_templates WHERE user_id = $1 AND template_type = $2 AND is_default = true"
        )
        .bind(user_id)
        .bind(template_type)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbEmailTemplate {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                template_type: row.try_get("template_type")?,
                subject: row.try_get("subject")?,
                body: row.try_get("body")?,
                body_format: row.try_get("body_format")?,
                is_default: row.try_get("is_default")?,
                attach_documents: row.try_get("attach_documents")?,
                attach_audit_log: row.try_get("attach_audit_log")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_template_by_id(pool: &PgPool, id: i64, user_id: i64) -> Result<Option<DbEmailTemplate>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log, created_at, updated_at FROM email_templates WHERE id = $1 AND user_id = $2"
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(DbEmailTemplate {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                template_type: row.try_get("template_type")?,
                subject: row.try_get("subject")?,
                body: row.try_get("body")?,
                body_format: row.try_get("body_format")?,
                is_default: row.try_get("is_default")?,
                attach_documents: row.try_get("attach_documents")?,
                attach_audit_log: row.try_get("attach_audit_log")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn update_template(pool: &PgPool, id: i64, user_id: i64, update_data: UpdateEmailTemplate) -> Result<Option<DbEmailTemplate>, sqlx::Error> {
        let now = Utc::now();
        let mut query = "UPDATE email_templates SET updated_at = $1".to_string();
        let mut param_count = 1;
        let mut params: Vec<String> = vec!["$1".to_string()];

        if let Some(template_type) = &update_data.template_type {
            param_count += 1;
            query.push_str(&format!(", template_type = ${}", param_count));
            params.push(format!("${}", param_count));
        }
        if let Some(subject) = &update_data.subject {
            param_count += 1;
            query.push_str(&format!(", subject = ${}", param_count));
            params.push(format!("${}", param_count));
        }
        if let Some(body) = &update_data.body {
            param_count += 1;
            query.push_str(&format!(", body = ${}", param_count));
            params.push(format!("${}", param_count));
        }
        if let Some(body_format) = &update_data.body_format {
            param_count += 1;
            query.push_str(&format!(", body_format = ${}", param_count));
            params.push(format!("${}", param_count));
        }
        if let Some(is_default) = update_data.is_default {
            param_count += 1;
            query.push_str(&format!(", is_default = ${}", param_count));
            params.push(format!("${}", param_count));
        }
        if let Some(attach_documents) = update_data.attach_documents {
            param_count += 1;
            query.push_str(&format!(", attach_documents = ${}", param_count));
            params.push(format!("${}", param_count));
        }
        if let Some(attach_audit_log) = update_data.attach_audit_log {
            param_count += 1;
            query.push_str(&format!(", attach_audit_log = ${}", param_count));
            params.push(format!("${}", param_count));
        }

        query.push_str(&format!(" WHERE id = ${} AND user_id = ${} RETURNING id, user_id, template_type, subject, body, body_format, is_default, attach_documents, attach_audit_log, created_at, updated_at", param_count + 1, param_count + 2));

        let mut sql_query = sqlx::query(&query).bind(now);

        if let Some(template_type) = &update_data.template_type {
            sql_query = sql_query.bind(template_type);
        }
        if let Some(subject) = &update_data.subject {
            sql_query = sql_query.bind(subject);
        }
        if let Some(body) = &update_data.body {
            sql_query = sql_query.bind(body);
        }
        if let Some(body_format) = &update_data.body_format {
            sql_query = sql_query.bind(body_format);
        }
        if let Some(is_default) = update_data.is_default {
            sql_query = sql_query.bind(is_default);
        }
        if let Some(attach_documents) = update_data.attach_documents {
            sql_query = sql_query.bind(attach_documents);
        }
        if let Some(attach_audit_log) = update_data.attach_audit_log {
            sql_query = sql_query.bind(attach_audit_log);
        }
        sql_query = sql_query.bind(id).bind(user_id);

        let row = sql_query.fetch_optional(pool).await?;

        match row {
            Some(row) => Ok(Some(DbEmailTemplate {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                template_type: row.try_get("template_type")?,
                subject: row.try_get("subject")?,
                body: row.try_get("body")?,
                body_format: row.try_get("body_format")?,
                is_default: row.try_get("is_default")?,
                attach_documents: row.try_get("attach_documents")?,
                attach_audit_log: row.try_get("attach_audit_log")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }
}
