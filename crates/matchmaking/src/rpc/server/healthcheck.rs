use std::pin::Pin;

use tokio_stream::Stream;
use tonic::Request;

use crate::rpc::matchmaking::{
    HealthCheckRequest, HealthCheckResponse, matchmaking_service_server::SERVICE_NAME,
};

pub(crate) type ResponseStream =
    Pin<Box<dyn Stream<Item = Result<HealthCheckResponse, tonic::Status>> + Send>>;

pub enum ServingStatus {
    NotFound,
    Serving,
    NotServing,
    ServiceUnknown,
    DEPRECATED,
}

impl From<ServingStatus> for i32 {
    fn from(value: ServingStatus) -> Self {
        match value {
            ServingStatus::NotFound => 0,
            ServingStatus::Serving => 1,
            ServingStatus::NotServing => 2,
            ServingStatus::ServiceUnknown => 3,
            ServingStatus::DEPRECATED => 4,
        }
    }
}

impl From<ServingStatus> for HealthCheckResponse {
    fn from(value: ServingStatus) -> Self {
        Self {
            status: value.into(),
        }
    }
}

pub fn healthy(request: Request<HealthCheckRequest>) -> HealthCheckResponse {
    if request.get_ref().service != SERVICE_NAME && request.get_ref().service != "matchmaking" {
        ServingStatus::NotFound.into()
    } else {
        use std::process::Command;

        let status = Command::new("echo").arg("healthy").status();
        match status {
            Ok(_) => ServingStatus::Serving.into(),
            Err(_) => ServingStatus::NotServing.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matchmaking_is_healthy() {
        let health = healthy(Request::new(HealthCheckRequest {
            service: "matchmaking".to_string(),
        }));
        assert_eq!(health.status, 1);

        let health = healthy(Request::new(HealthCheckRequest {
            service: SERVICE_NAME.to_string(),
        }));
        assert_eq!(health.status, 1);
    }

    #[test]
    fn other_service_is_notfound() {
        let health = healthy(Request::new(HealthCheckRequest {
            service: "random".to_string(),
        }));
        assert_eq!(health.status, 0);
    }
}
