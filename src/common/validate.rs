use tonic::Status;
use uuid::Uuid;

pub fn validate_uuid(value: &String) -> Result<Uuid, Status> {
    value
        .parse::<Uuid>()
        .map_err(|_| Status::invalid_argument(""))
}
