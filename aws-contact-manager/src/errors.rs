use thiserror::Error;

// ============================================================================
// Boxed error alias — replaces the direct smithy SdkError<E> dependency
// ============================================================================

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

// ============================================================================
// Custom Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum AppError {
    #[error("AWS Organizations error: {0}")]
    Organizations(#[from] OrganizationsError),

    #[error("AWS STS error: {0}")]
    Sts(#[from] StsError),

    #[error("AWS Account error: {0}")]
    Account(#[from] AccountError),

    #[error("AWS S3 error: {0}")]
    S3(#[from] S3Error),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("User input error: {0}")]
    UserInput(String),

    #[error("Unknown alternate contact type: {0}")]
    UnknownContactType(String),
}

#[derive(Error, Debug)]
pub enum OrganizationsError {
    #[error("Failed to list accounts: {message}")]
    ListAccounts {
        message: String,
        #[source]
        source: Option<BoxError>,
    },

    #[error("Access denied to Organizations API. Ensure you have the required permissions.")]
    AccessDenied,

    #[error("Organizations service unavailable. Please retry later.")]
    ServiceUnavailable,
}

#[derive(Error, Debug)]
pub enum StsError {
    #[error("Failed to get caller identity: {message}")]
    GetCallerIdentity {
        message: String,
        #[source]
        source: Option<BoxError>,
    },

    #[error("No account ID found in caller identity response")]
    NoAccountId,
}

#[derive(Error, Debug)]
pub enum AccountError {
    #[error("Failed to get alternate contact for account {account_id}, type {contact_type}: {message}")]
    GetAlternateContact {
        account_id: String,
        contact_type: String,
        message: String,
        #[source]
        source: Option<BoxError>,
    },

    #[error("Failed to update alternate contact for account {account_id}, type {contact_type}: {message}")]
    PutAlternateContact {
        account_id: String,
        contact_type: String,
        message: String,
        #[source]
        source: Option<BoxError>,
    },

    #[error("Failed to delete alternate contact for account {account_id}, type {contact_type}: {message}")]
    DeleteAlternateContact {
        account_id: String,
        contact_type: String,
        message: String,
        #[source]
        source: Option<BoxError>,
    },

    #[error("Alternate contact not found for account {account_id}, type {contact_type}")]
    ResourceNotFound {
        account_id: String,
        contact_type: String,
    },

    #[error("Access denied to account {account_id}. Check trusted access and permissions.")]
    AccessDenied { account_id: String },

    #[error("Too many requests. Please slow down and retry.")]
    TooManyRequests,
}

#[derive(Error, Debug)]
pub enum S3Error {
    #[error("Failed to upload to S3 bucket '{bucket}', key '{key}': {message}")]
    PutObject {
        bucket: String,
        key: String,
        message: String,
        #[source]
        source: Option<BoxError>,
    },

    #[error("S3 bucket '{bucket}' not found")]
    BucketNotFound { bucket: String },

    #[error("Access denied to S3 bucket '{bucket}'")]
    AccessDenied { bucket: String },
    
    #[error("S3 bucket '{bucket}' does not exist")]
    NoSuchBucket { bucket: String },
}

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid AWS Account ID '{account_id}': must be exactly 12 digits")]
    InvalidAccountId { account_id: String },

    #[error("Account ID '{account_id}' does not belong to your AWS Organization")]
    AccountNotInOrganization { account_id: String },

    #[error("No accounts provided")]
    NoAccountsProvided,
}

// ============================================================================
// Result Type Alias
// ============================================================================

pub type AppResult<T> = Result<T, AppError>;

// ============================================================================
// Error Classification Helpers
// ============================================================================
//
// These operate on BoxError via debug-string inspection, keeping the same
// classification logic as before without requiring smithy types directly.

pub fn error_is_access_denied(err: &BoxError) -> bool {
    let s = format!("{:?}", err);
    s.contains("AccessDenied") || s.contains("UnauthorizedAccess")
}

pub fn error_is_throttling(err: &BoxError) -> bool {
    let s = format!("{:?}", err);
    s.contains("TooManyRequests") || s.contains("Throttling")
}

pub fn error_is_service_unavailable(err: &BoxError) -> bool {
    let s = format!("{:?}", err);
    s.contains("ServiceUnavailable")
        || s.contains("InternalError")
        || s.contains("TimeoutError")
        || s.contains("DispatchFailure")
}

pub fn error_is_not_found(err: &BoxError) -> bool {
    let s = format!("{:?}", err);
    s.contains("ResourceNotFoundException") || s.contains("ResourceNotFound")
}