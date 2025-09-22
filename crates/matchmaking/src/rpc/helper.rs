use std::fmt::Debug;

use chrono::{DateTime, Local};
use tonic::Status;
use tracing::error;

use crate::rpc::server::GAME_START;

pub trait IntoTonicError<T> {
    fn to_tonic_error(
        self,
        error_msg: impl Into<String>,
        func: Box<dyn Fn(String) -> Status>,
    ) -> Result<T, Status>;
}

impl<T, E: Debug> IntoTonicError<T> for Result<T, E> {
    fn to_tonic_error(
        self,
        error_msg: impl Into<String>,
        func: Box<dyn Fn(String) -> Status>,
    ) -> Result<T, Status> {
        self.inspect_err(|err| error!("{err:?}"))
            .map_err(|_| func(error_msg.into()))
    }
}

pub fn time_since(dt: &DateTime<Local>) -> Result<i64, tonic::Status> {
    Ok(dt.naive_utc()
    .signed_duration_since(
        GAME_START
            .and_then(|dt| dt.and_hms_opt(0, 0, 0))
            .ok_or_else(|| tonic::Status::internal("Failed define time of player join"))?,
    )
    .num_seconds())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now() {
        let dt = Local::now();
        let time = time_since(&dt).unwrap();
        assert!(time > 22810515)
    }
}