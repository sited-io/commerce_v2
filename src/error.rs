use std::num::TryFromIntError;

use prisma_client_rust::QueryError;
use stripe::StripeError;
use tonic::Status;

#[derive(Debug, Default)]
pub struct Error {
    msg: String,
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Self { msg }
    }
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        value.to_string().into()
    }
}

impl From<TryFromIntError> for Error {
    fn from(value: TryFromIntError) -> Self {
        tracing::error!("Could not convert int. Error: {:?}", value);
        value.to_string().into()
    }
}
impl From<QueryError> for Error {
    fn from(value: QueryError) -> Self {
        tracing::error!("{:?}", value);
        value.to_string().into()
    }
}
impl From<Status> for Error {
    fn from(value: Status) -> Self {
        tracing::error!("{:?}", value);
        value.to_string().into()
    }
}

impl From<StripeError> for Error {
    fn from(value: StripeError) -> Self {
        tracing::error!("{:?}", value);
        value.to_string().into()
    }
}

impl From<Error> for Status {
    fn from(_: Error) -> Self {
        Self::internal("")
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
