use std::{marker::PhantomData, str::FromStr};

use httpmock::{Method::POST, MockServer};
use redis::aio::MultiplexedConnection;
use serde_json::json;
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

use super::*;
use crate::nakama::NakamaClient;

#[tokio::test]
async fn test_join_queue() {
    let container = create_redis(6379).await;
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(6379).await.unwrap();
    let client = redis_client(host.to_string(), port).await;
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
    init_regions(conn.clone()).await;

    let server = MockServer::start_async().await;
    let server_port = server.address().port();
    let mock = server
        .mock_async(|when, then| {
            when.method(POST)
                .path("/v2/console/api/endpoints/rpc/healthcheck")
                .scheme("http")
                .any_request();
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"body": "{\"success\": true}", "error_message": "error"}));
        })
        .await;

    let http = reqwest::Client::new();
    let nakama_client = auth_client(server_port);
    let nakama_client = Arc::new(nakama_client);
    let http_client = Arc::new(http);
    let matchmaking_server = MatchmakingServer {
        redis: conn.clone(),
        http_client,
        nakama_client,
    };

    let player_data = Player {
        player_id: "01997433-3000-7b4b-8712-9253d26a68c8".to_string(),
        loadout_config: "".to_string(),
        region: "CAN".to_string(),
        ping: 20,
        difficulty: 1,
        join_mode: 2,
        party_mode: 0,
        party_member_id: Vec::new(),
    };
    let mut req = Request::new(player_data.clone());
    add_auth(&mut req);
    let response = matchmaking_server.join_queue(req).await.unwrap();

    mock.assert_async().await;

    let saved_player_encoded: Option<Vec<u8>> = conn
        .get(Uuid::from_str("01997433-3000-7b4b-8712-9253d26a68c8").unwrap())
        .await
        .unwrap();
    let decoded_player: QueuedPlayer = bitcode::decode(&saved_player_encoded.unwrap()).unwrap();

    let zqueued = conn
        .zrange::<String, Vec<Option<Vec<u8>>>>(player_queue_key(&decoded_player), 0, -1)
        .await
        .unwrap()[0]
        .clone()
        .unwrap();
    let zmatch = conn
        .zrange::<String, Vec<Option<Vec<u8>>>>(create_match_queue_key(&player_data.region), 0, 1)
        .await
        .unwrap();

    container.pause().await.unwrap();
    let decode_queued: QueuedPlayer = bitcode::decode(&zqueued).unwrap();
    assert_eq!(decode_queued, decoded_player);
    // Only player is not Host
    assert!(zmatch.is_empty());

    assert_eq!(
        decoded_player.player_id,
        Uuid::from_str(player_data.player_id.as_str()).unwrap()
    );

    let response = response.into_inner();
    assert_eq!(response.player_id, player_data.player_id);
    assert_eq!(response.status, "waiting in queue");
}

async fn redis_client(host: String, port: u16) -> redis::Client {
    redis::Client::open(format!("redis://{host}:{port}")).unwrap()
}

async fn create_redis(port: u16) -> ContainerAsync<GenericImage> {
    GenericImage::new("redis", "8.2.1-bookworm")
        .with_exposed_port(port.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .with_network("bridge")
        .with_env_var("REDIS_PASSWORD", "super-secret-password")
        .with_env_var("REDIS_USER", "redis_mms_admin")
        .start()
        .await
        .expect("Failed to start Redis")
}

pub fn auth_client(port: u16) -> NakamaClient<Authenticated> {
    NakamaClient {
        username: "username".to_string(),
        password: "password".to_string(),
        token: Some("super_random_token".to_string()),
        url: format!("http://127.0.0.1:{port}"),
        server_key_name: "defaultkey".to_string(),
        server_key_value: "server_key".to_string(),
        encryption_key: "encryption_key".to_string(),
        _state: PhantomData::<Authenticated>,
    }
}

async fn init_regions(conn: MultiplexedConnection) {
    let regions = &[
        "CAN".to_string(),
        "US".to_string(),
        "SOUTH_AMERICA".to_string(),
    ];

    crate::regions::set_regions(conn, regions).await.unwrap();
}

fn add_auth(req: &mut Request<Player>) {
    req.extensions_mut().insert(auth::UserId {
        player_id: "01997433-3000-7b4b-8712-9253d26a68c8".to_string(),
    });
}
