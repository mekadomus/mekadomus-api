use axum::{
    body::Body,
    http,
    http::{Method, Request, StatusCode},
};
use chrono::{Duration, NaiveDateTime, Utc};
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use http_body_util::BodyExt;
use mockall::predicate::{always, eq};
use serde_json::{json, Value};
use std::sync::Arc;
use test_log::test;
use tower::util::ServiceExt;

use mekadomus_api::{
    api::user::{
        LogInUserInput, NewPasswordInput, SignUpUserInput, User, UserAuthProvider::Password,
    },
    error::app_error::{
        AppError, AppErrorCode, ErrorData, ErrorResponse, ValidationIssue::TooFrequent,
    },
    helper::{mail::MockMailHelper, user::MockUserHelper},
    middleware::auth::DefaultAuthorizer,
    storage::{postgres::PostgresStorage, UserStorage},
};

use crate::helper::app::{create_app, create_app_basic, create_app_user_helper};

#[test(tokio::test)]
async fn sign_up_user_weak_password() {
    let password = "12345678";
    let captcha = "heyyou";
    let hashed_password = "hashed-123";

    // Mock UserHelper
    let mut user_helper_mock = MockUserHelper::new();
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(password))
        .return_const(true);
    user_helper_mock
        .expect_is_bot()
        .with(eq("my_secret"), eq(captcha))
        .return_const(false);
    user_helper_mock
        .expect_hash()
        .with(eq(password))
        .returning(|_| Ok(hashed_password.to_string()));

    let app = create_app_user_helper(Arc::new(user_helper_mock)).await;

    let input = SignUpUserInput {
        email: "my.user@you.know".to_string(),
        name: "Someone last".to_string(),
        password: password.to_string(),
        captcha: captcha.to_string(),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "code": "ValidationError", "data": { "ValidationInfo": [ { "field": "password", "issue": "TooWeak" } ] }, "message": "Request data is invalid" })
    );
}

#[test(tokio::test)]
async fn sign_up_failed_captcha() {
    let password = "12345678";
    let captcha = "heyyou";

    // Mock UserHelper
    let mut user_helper_mock = MockUserHelper::new();
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(password))
        .return_const(false);
    user_helper_mock
        .expect_is_bot()
        .with(eq("my_secret"), eq(captcha))
        .return_const(true);

    let app = create_app_user_helper(Arc::new(user_helper_mock)).await;

    let input = SignUpUserInput {
        email: "my.user@you.know".to_string(),
        name: "Someone last".to_string(),
        password: password.to_string(),
        captcha: captcha.to_string(),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "code": "ValidationError", "data": { "ValidationInfo": [ { "field": "captcha", "issue": "Invalid" } ] }, "message": "Request data is invalid" })
    );
}

#[test(tokio::test)]
async fn sign_up_failed_hash() {
    let password = "12345678";
    let captcha = "heyyou";

    // Mock UserHelper
    let mut user_helper_mock = MockUserHelper::new();
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(password))
        .return_const(false);
    user_helper_mock
        .expect_is_bot()
        .with(eq("my_secret"), eq(captcha))
        .return_const(false);
    user_helper_mock
        .expect_hash()
        .with(eq(password))
        .returning(|_| Err(AppError::ServerError));

    let app = create_app_user_helper(Arc::new(user_helper_mock)).await;

    let input = SignUpUserInput {
        email: "my.user@you.know".to_string(),
        name: "Someone last".to_string(),
        password: password.to_string(),
        captcha: captcha.to_string(),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "code": "InternalError", "data": "", "message": "We made a mistake. Sorry" })
    );
}

#[test(tokio::test)]
async fn sign_up_failed_empty_name() {
    let password = "12345678";
    let captcha = "heyyou";
    let hashed_password = "hashed-123";

    // Mock UserHelper
    let mut user_helper_mock = MockUserHelper::new();
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(password))
        .return_const(false);
    user_helper_mock
        .expect_is_bot()
        .with(eq("my_secret"), eq(captcha))
        .return_const(false);
    user_helper_mock
        .expect_hash()
        .with(eq(password))
        .returning(|_| Ok(hashed_password.to_string()));

    let app = create_app_user_helper(Arc::new(user_helper_mock)).await;

    let input = SignUpUserInput {
        email: "my.user@you.know".to_string(),
        name: "    ".to_string(),
        password: password.to_string(),
        captcha: captcha.to_string(),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "code": "ValidationError", "data": { "ValidationInfo": [ { "field": "name", "issue": "Required" } ] }, "message": "Request data is invalid" })
    );
}

#[test(tokio::test)]
async fn sign_up_failed_invalid_email() {
    let password = "12345678";
    let captcha = "heyyou";
    let hashed_password = "hashed-123";

    // Mock UserHelper
    let mut user_helper_mock = MockUserHelper::new();
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(password))
        .return_const(false);
    user_helper_mock
        .expect_is_bot()
        .with(eq("my_secret"), eq(captcha))
        .return_const(false);
    user_helper_mock
        .expect_hash()
        .with(eq(password))
        .returning(|_| Ok(hashed_password.to_string()));

    let app = create_app_user_helper(Arc::new(user_helper_mock)).await;

    let input = SignUpUserInput {
        email: "my.useryou.know".to_string(),
        name: "Gallo Claudio".to_string(),
        password: password.to_string(),
        captcha: captcha.to_string(),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "code": "ValidationError", "data": { "ValidationInfo": [ { "field": "email", "issue": "Invalid" } ] }, "message": "Request data is invalid" })
    );
}

#[test(tokio::test)]
async fn sign_up_user_duplicate() {
    let password = "0987654321";
    let captcha = "hellothere";
    let hashed_password = "hashed-123";

    // Mock UserHelper
    let mut user_helper_mock = MockUserHelper::new();
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(password))
        .return_const(false);
    user_helper_mock
        .expect_is_bot()
        .with(eq("my_secret"), eq(captcha))
        .return_const(false);
    user_helper_mock
        .expect_hash()
        .with(eq(password))
        .returning(|_| Ok(hashed_password.to_string()));

    let mut mail_helper_mock = MockMailHelper::new();
    mail_helper_mock
        .expect_sign_up_verification()
        .with(always(), always(), always())
        .return_const(true);

    let app = create_app(
        Arc::new(DefaultAuthorizer {}),
        Arc::new(mail_helper_mock),
        Arc::new(user_helper_mock),
    )
    .await;

    let input = SignUpUserInput {
        email: "other.user@you.know".to_string(),
        name: "Someone last".to_string(),
        password: password.to_string(),
        captcha: captcha.to_string(),
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "code": "InternalError", "data": "", "message": "We made a mistake. Sorry" })
    );
}

#[test(tokio::test)]
async fn sign_up_user_email_failure() {
    let password = "12345678";
    let captcha = "heyyou";
    let hashed_password = "hashed-123";

    // Mock UserHelper
    let mut user_helper_mock = MockUserHelper::new();
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(password))
        .return_const(false);
    user_helper_mock
        .expect_is_bot()
        .with(eq("my_secret"), eq(captcha))
        .return_const(false);
    user_helper_mock
        .expect_hash()
        .with(eq(password))
        .returning(|_| Ok(hashed_password.to_string()));

    let mut mail_helper_mock = MockMailHelper::new();
    mail_helper_mock
        .expect_sign_up_verification()
        .with(always(), always(), always())
        .return_const(false);

    let app = create_app(
        Arc::new(DefaultAuthorizer {}),
        Arc::new(mail_helper_mock),
        Arc::new(user_helper_mock),
    )
    .await;

    let input = SignUpUserInput {
        email: "email.failure@you.know".to_string(),
        name: "Someone last".to_string(),
        password: password.to_string(),
        captcha: captcha.to_string(),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "code": "InternalError", "data": "", "message": "We made a mistake. Sorry" })
    );

    // Verify user was not saved
    let id = format!("{}+{}", &input.email, Password);
    let storage = PostgresStorage::new("postgresql://user:password@postgres/mekadomus").await;
    assert!(storage.user_by_id(&id).await.unwrap().is_none());
}

#[test(tokio::test)]
async fn email_verification_wrong_token() {
    let app = create_app_basic().await; // Make simple one

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/v1/email-verification?token=wrongtoken")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test(tokio::test)]
async fn sign_up_to_log_in_to_log_out_happy_path() {
    // Sign up a user
    let password = "12345678";
    let captcha = "heyyou";
    let hashed_password = "hashed-123";

    // Mock UserHelper
    let mut user_helper_mock = MockUserHelper::new();
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(password))
        .return_const(false);
    user_helper_mock
        .expect_is_bot()
        .with(eq("my_secret"), eq(captcha))
        .return_const(false);
    user_helper_mock
        .expect_hash()
        .with(eq(password))
        .returning(|_| Ok(hashed_password.to_string()));
    user_helper_mock
        .expect_verify_hash()
        .with(eq(password), eq(hashed_password))
        .returning(|_, _| Ok(true));

    let mut mail_helper_mock = MockMailHelper::new();
    mail_helper_mock
        .expect_sign_up_verification()
        .with(always(), always(), always())
        .return_const(true);

    let app = create_app(
        Arc::new(DefaultAuthorizer {}),
        Arc::new(mail_helper_mock),
        Arc::new(user_helper_mock),
    )
    .await;

    let input = SignUpUserInput {
        email: "my.user@you.know".to_string(),
        name: "Someone last".to_string(),
        password: password.to_string(),
        captcha: captcha.to_string(),
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/sign-up")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body.get("id").unwrap().as_str().unwrap(),
        "my.user@you.know+password"
    );
    assert_eq!(body.get("provider").unwrap().as_str().unwrap(), "password");
    assert_eq!(body.get("email").unwrap().as_str().unwrap(), input.email);
    assert!(body.get("password").unwrap().as_str().is_none()); // Password shouldn't be returned
    let actual_date = NaiveDateTime::parse_from_str(
        body.get("recorded_at").unwrap().as_str().unwrap(),
        "%Y-%m-%dT%H:%M:%S%.f",
    );
    assert!(Utc::now().naive_utc() > actual_date.expect("Bad date"));
    // E-mail hasn't been verified
    assert!(body.get("email_verified_at").unwrap().as_str().is_none());

    // Get the token for the user
    let storage =
        Arc::new(PostgresStorage::new("postgresql://user:password@postgres/mekadomus").await);
    let id = format!("{}+{}", &input.email, Password);
    let token = match storage.email_verification_by_id(&id).await {
        Ok(v) => v.unwrap().token,
        Err(_) => panic!("Failed to get e-mail verification"),
    };

    // Verify token for the user
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/v1/email-verification?token={}", token))
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let input = LogInUserInput {
        email: "my.user@you.know".to_string(),
        password: password.to_string(),
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/log-in")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body.get("user_id").unwrap().as_str().unwrap(),
        "my.user@you.know+password"
    );
    let token = body.get("token").unwrap().as_str().unwrap();
    let expires_at = NaiveDateTime::parse_from_str(
        body.get("expires_at").unwrap().as_str().unwrap(),
        "%Y-%m-%dT%H:%M:%S%.f",
    )
    .expect("Bad date");
    assert!(Utc::now().naive_utc() < expires_at);
    assert!((Utc::now().naive_utc() + Duration::days(30)) > expires_at);

    // Make a request that requires the token
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/v1/me")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .header(AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body.get("id").unwrap().as_str().unwrap(),
        "my.user@you.know+password"
    );
    assert_eq!(body.get("provider").unwrap().as_str().unwrap(), "password");
    assert_eq!(body.get("email").unwrap().as_str().unwrap(), input.email);
    assert!(body.get("password").unwrap().as_str().is_none()); // Password shouldn't be returned
    let actual_date = NaiveDateTime::parse_from_str(
        body.get("recorded_at").unwrap().as_str().unwrap(),
        "%Y-%m-%dT%H:%M:%S%.f",
    );
    assert!(Utc::now().naive_utc() > actual_date.expect("Bad date"));
    assert!(body.get("email_verified_at").unwrap().as_str().is_some());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/log-out")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .header(AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[test(tokio::test)]
async fn me_no_auth_token() {
    let app = create_app_basic().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::GET)
                .uri("/v1/me")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let resp: ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp.code, AppErrorCode::Unauthorized);
}

#[test(tokio::test)]
async fn recover_password_failed_invalid_email() {
    let app = create_app_basic().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::GET)
                .uri("/v1/recover-password?email=my.user")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let resp: ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp.code, AppErrorCode::ValidationError);
    match resp.data {
        ErrorData::ValidationInfo(v) => {
            assert_eq!(v[0].field, "email");
        }
        _ => {
            panic!("Invalid error response");
        }
    }
}

#[test(tokio::test)]
async fn recover_password_success_if_user_not_found() {
    let app = create_app_basic().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(http::Method::GET)
                .uri("/v1/recover-password?email=non.existing@us.er")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[test(tokio::test)]
async fn recover_password_success() {
    let email = "existing@us.er";
    let user = User {
        id: format!("{}+password", email),
        provider: Password,
        name: "Carlos".to_string(),
        email: email.to_string(),
        password: Some("hello".to_string()),
        email_verified_at: Some(Utc::now().naive_utc()),
        recorded_at: Utc::now().naive_utc(),
    };
    let storage =
        Arc::new(PostgresStorage::new("postgresql://user:password@postgres/mekadomus").await);
    let _ = storage.insert_user(&user).await;

    let mut mail_helper_mock = MockMailHelper::new();
    mail_helper_mock
        .expect_password_recovery()
        .with(always(), always(), always())
        .return_const(true);

    let mut user_helper_mock = MockUserHelper::new();
    let new_password = "Chicharron15?";
    let hashed_password = "imhash";
    user_helper_mock
        .expect_password_is_weak()
        .with(eq(new_password))
        .return_const(false);
    user_helper_mock
        .expect_hash()
        .with(eq(new_password))
        .returning(|_| Ok(hashed_password.to_string()));

    let app = create_app(
        Arc::new(DefaultAuthorizer {}),
        Arc::new(mail_helper_mock),
        Arc::new(user_helper_mock),
    )
    .await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::GET)
                .uri(format!("/v1/recover-password?email={}", email))
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Don't allow another recovery attempt
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::GET)
                .uri(format!("/v1/recover-password?email={}", email))
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let resp: ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp.code, AppErrorCode::ValidationError);
    match resp.data {
        ErrorData::ValidationInfo(v) => {
            assert_eq!(v[0].issue, TooFrequent);
        }
        _ => {
            panic!("Invalid error response");
        }
    }

    // Set a new password using the password_recovery token
    let pr = storage.password_recovery_by_user(&user.id).await.unwrap();
    let input = NewPasswordInput {
        token: pr.unwrap().token,
        password: new_password.to_string(),
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/v1/new-password")
                .header(CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_string(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    assert!(storage
        .password_recovery_by_user(&user.id)
        .await
        .unwrap()
        .is_none());
    let new_user = storage.user_by_id(&user.id).await.unwrap().unwrap();
    assert_eq!(hashed_password, new_user.password.unwrap())
}
