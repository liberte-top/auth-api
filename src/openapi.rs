use utoipa::OpenApi;

use crate::{
    handler,
    handler::{
        accounts::{AccountResponse, CreateAccount, UpdateAccount},
        auth::password::{
            ErrorResponse, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse,
            VerifyEmailRequest, VerifyEmailResponse,
        },
        health::Health,
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handler::health::health,
        handler::accounts::create_account,
        handler::accounts::get_account,
        handler::accounts::update_account,
        handler::accounts::delete_account,
        handler::auth::password::register,
        handler::auth::password::login,
        handler::auth::password::logout,
        handler::auth::password::verify_email
    ),
    components(schemas(
        Health,
        CreateAccount,
        UpdateAccount,
        AccountResponse,
        RegisterRequest,
        RegisterResponse,
        LoginRequest,
        LoginResponse,
        VerifyEmailRequest,
        VerifyEmailResponse,
        ErrorResponse
    )),
    tags(
        (name = "health", description = "Health check"),
        (name = "accounts", description = "Accounts"),
        (name = "auth", description = "Authentication")
    )
)]
pub struct ApiDoc;
