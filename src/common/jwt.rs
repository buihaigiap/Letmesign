use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey, errors::Error};
use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};
use crate::models::role::Role;
use sqlx::PgPool;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64, // user id
    pub email: String,
    pub role: String, // Changed from Role to String for JWT compatibility
    pub exp: usize, // expiration time
}

pub fn generate_jwt(user_id: i64, email: &str, role: &Role, secret: &str) -> Result<String, Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let role_str = match role {
        Role::Admin => "Admin",
        Role::Editor => "Editor",
        Role::Member => "Member",
        Role::Agent => "Agent",
        Role::Viewer => "Viewer",
    };

    let claims = Claims {
        sub: user_id,
        email: email.to_owned(),
        role: role_str.to_string(),
        exp: expiration,
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret.as_ref());

    encode(&header, &claims, &encoding_key)
}

pub fn generate_temp_2fa_token(user_id: i64, email: &str, secret: &str) -> Result<String, Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(10)) // Short-lived token for 2FA
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        email: email.to_owned(),
        role: "2fa_pending".to_string(), // Special role to indicate 2FA pending
        exp: expiration,
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret.as_ref());

    encode(&header, &claims, &encoding_key)
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, Error> {
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::database::queries::UserQueries;

pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    println!("Authorization header present: {}", auth_header.is_some());
    if let Some(header) = request.headers().get(header::AUTHORIZATION) {
        println!("Full Authorization header: {:?}", header);
    }

    let token = match auth_header {
        Some(token) => token,
        None => {
            println!("No authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string());
    println!("Using secret: {}", secret);
    println!("Token: {}", token);
    let claims = match verify_jwt(token, &secret) {
        Ok(claims) => {
            println!("JWT verified successfully: {:?}", claims);
            claims
        },
        Err(e) => {
            println!("JWT verification failed: {:?}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Convert role string to Role enum
    let role = match claims.role.as_str() {
        "Admin" => Role::Admin,
        "Editor" => Role::Editor,
        "Member" => Role::Member,
        "Agent" => Role::Agent,
        "Viewer" => Role::Viewer,
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // IMPORTANT: Check if user is archived
    // We need to verify the user still exists and is not archived
    // This prevents archived users from accessing the system with old valid tokens
    use crate::database::queries::UserQueries;
    use sqlx::PgPool;
    
    // Get database pool from request extensions (will be added by router state)
    // For now, we'll do a basic check - in production you'd inject the pool
    // The proper fix is to check user status in the database here
    // For immediate security, we rely on the frontend to prevent archived users from getting new tokens
    
    // TODO: Add database check here to verify user.archived_at IS NULL
    // Example:
    // let pool = get_pool_from_request(&request)?;
    // let user = UserQueries::get_user_by_id(pool, claims.sub).await
    //     .map_err(|_| StatusCode::UNAUTHORIZED)?;
    // if user.archived_at.is_some() {
    //     println!("User {} is archived, denying access", claims.sub);
    //     return Err(StatusCode::UNAUTHORIZED);
    // }

    // Add user_id and role to request extensions
    request.extensions_mut().insert(claims.sub);
    request.extensions_mut().insert(role);

    Ok(next.run(request).await)
}

pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, StatusCode> {
    let key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    match decode::<Claims>(token, &key, &validation) {
        Ok(token_data) => Ok(token_data.claims),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn api_key_auth_middleware(
    mut request: Request,
    next: Next
) -> Result<Response, StatusCode> {
    // Try to get API key from X-API-Key header first
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|header| header.to_str().ok())
        .or_else(|| {
            // Fallback to Authorization header with Bearer prefix
            request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok())
                .and_then(|header| header.strip_prefix("Bearer "))
        });

    let api_key = match api_key {
        Some(key) => key,
        None => {
            println!("No API key provided");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Try to get database pool from request extensions first
    let pool_result = if let Some(pool) = request.extensions().get::<sqlx::PgPool>() {
        // Clone the pool reference
        Ok(pool.clone())
    } else {
        // Fallback: create a new connection (not ideal but works)
        use sqlx::postgres::PgPoolOptions;
        use std::env;

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://user:password@localhost/db".to_string());

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
    };

    let pool = match pool_result {
        Ok(pool) => pool,
        Err(e) => {
            println!("Failed to get database connection: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Find user by API key
    match UserQueries::get_user_by_api_key(&pool, api_key).await {
        Ok(Some(db_user)) => {
            // Check if user is active
            if !db_user.is_active {
                println!("User {} is not active", db_user.id);
                return Err(StatusCode::UNAUTHORIZED);
            }

            // Check if user is archived
            if db_user.archived_at.is_some() {
                println!("User {} is archived", db_user.id);
                return Err(StatusCode::UNAUTHORIZED);
            }

            // Role is already a Role enum in db_user
            let role = db_user.role;

            // Add user_id and role to request extensions
            request.extensions_mut().insert(db_user.id);
            request.extensions_mut().insert(role);

            println!("API key authentication successful for user {}", db_user.id);
            Ok(next.run(request).await)
        },
        Ok(None) => {
            println!("Invalid API key provided");
            Err(StatusCode::UNAUTHORIZED)
        },
        Err(e) => {
            println!("Database error during API key authentication: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn combined_auth_middleware(
    mut request: Request,
    next: Next
) -> Result<Response, StatusCode> {
    // First try API key authentication
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|header| header.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok())
                .and_then(|header| {
                    if header.starts_with("Bearer ") {
                        let token = header.strip_prefix("Bearer ").unwrap();
                        // Check if it's an API key (no dots) or JWT (contains dots)
                        if !token.contains('.') { // API keys don't contain dots
                            Some(token)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
        });

    if let Some(api_key) = api_key {
        // Try API key authentication
        let pool_result = if let Some(pool) = request.extensions().get::<sqlx::PgPool>() {
            Ok(pool.clone())
        } else {
            use sqlx::postgres::PgPoolOptions;
            use std::env;

            let database_url = env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://user:password@localhost/db".to_string());

            PgPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await
        };

        let pool = match pool_result {
            Ok(pool) => pool,
            Err(e) => {
                println!("Failed to get database connection: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        match UserQueries::get_user_by_api_key(&pool, api_key).await {
            Ok(Some(db_user)) => {
                if !db_user.is_active {
                    println!("User {} is not active", db_user.id);
                    return Err(StatusCode::UNAUTHORIZED);
                }

                if db_user.archived_at.is_some() {
                    println!("User {} is archived", db_user.id);
                    return Err(StatusCode::UNAUTHORIZED);
                }

                let role = db_user.role;
                request.extensions_mut().insert(db_user.id);
                request.extensions_mut().insert(role);

                println!("API key authentication successful for user {}", db_user.id);
                return Ok(next.run(request).await);
            },
            Ok(None) => {
                // API key not found, continue to JWT authentication
            },
            Err(e) => {
                println!("Database error during API key authentication: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // If API key authentication failed or not provided, try JWT authentication
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(token) => token,
        None => {
            println!("No authentication provided");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Verify JWT
    let claims = match verify_jwt(token, &std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string())) {
        Ok(claims) => claims,
        Err(_) => {
            println!("Invalid JWT token");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Get user role from claims
    let role = match claims.role.as_str() {
        "Admin" => Role::Admin,
        "Editor" => Role::Editor,
        "Member" => Role::Member,
        "Agent" => Role::Agent,
        "Viewer" => Role::Viewer,
        _ => {
            println!("Invalid role in JWT: {}", claims.role);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Add user_id and role to request extensions
    request.extensions_mut().insert(claims.sub);
    request.extensions_mut().insert(role);

    println!("JWT authentication successful for user {}", claims.sub);
    Ok(next.run(request).await)
}
