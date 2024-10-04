use std::num::TryFromIntError;

use prisma_client_rust::QueryError;
use tonic::Status;

pub struct Error {}

impl From<TryFromIntError> for Error {
    fn from(err: TryFromIntError) -> Self {
        tracing::error!("Could not convert int. Error: {:?}", err);
        Self {}
    }
}
impl From<QueryError> for Error {
    fn from(err: QueryError) -> Self {
        tracing::error!("{:?}", err);
        Self {}
    }
}

impl From<Error> for Status {
    fn from(_: Error) -> Self {
        Self::internal("")
    }
}
