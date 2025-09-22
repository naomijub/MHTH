use crc::{CRC_16_CDMA2000, Crc};
use tracing::debug;

use crate::nakama::{Error, SALTING_KEY};

pub(super) fn get_password(env_password: &str) -> String {
    let crc = Crc::<u16>::new(&CRC_16_CDMA2000);
    let mut digest = crc.digest();
    digest.update(env_password.as_bytes());
    digest.update(SALTING_KEY.as_bytes());
    let crc = digest.finalize();

    format!("{}{}{:X}", env_password, SALTING_KEY, crc)
}

pub(super) fn get_env_user() -> String {
    match std::env::var("NAKAMA_USERNAME") {
        Ok(url) => url,
        Err(_) => {
            debug!(".env `NAKAMA_USERNAME` not found. Using default.");
            "mhth_nakama_client".to_string()
        }
    }
}

pub(super) fn get_env_password() -> Result<String, Error> {
    match std::env::var("NAKAMA_PASSWORD") {
        Ok(pswd) => Ok(pswd),
        Err(_) => Err(Error::PasswordEnvNotSet),
    }
}

pub(super) fn get_env_endpoint() -> String {
    let port = std::env::var("NAKAMA_CONSOLE_PORT").unwrap_or_else(|_| "7351".to_string());
    match std::env::var("NAKAMA_HOST") {
        Ok(url) => format!("http://{url}:{port}"),
        Err(_) => {
            debug!(".env `NAKAMA_HOST` not found. Using default.");
            "http://127.0.0.1:7350".to_string()
        }
    }
}

pub(super) fn get_env_server_key_name() -> String {
    match std::env::var("NAKAMA_SERVER_KEY_NAME") {
        Ok(url) => url,
        Err(_) => {
            debug!(".env `NAKAMA_SERVER_KEY_NAME` not found. Using default.");
            "defaultkey".to_string()
        }
    }
}

pub(super) fn get_env_server_key_value() -> String {
    match std::env::var("NAKAMA_SERVER_KEY") {
        Ok(url) => url,
        Err(_) => {
            debug!(".env `NAKAMA_SERVER_KEY` not found. Using default.");
            "abcde123".to_string()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::get_password;

    #[test]
    fn salt_password() {
        let my_unsalted = "unsaltedPassword";
        let salted = get_password(my_unsalted);

        assert_eq!(salted, "unsaltedPasswordfL@.P47H$P!fmcdcF460");
    }
}
