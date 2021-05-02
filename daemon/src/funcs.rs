use spacetraders::client::{self, HttpClient};
use spacetraders::errors::GameStatusError;

pub async fn is_api_in_maintenance_mode(http_client: HttpClient) -> bool {
    let game_status = client::get_game_status(http_client.clone()).await;

    if game_status.is_err() {
        let game_status_error = game_status.err().unwrap();

        return matches!(game_status_error, GameStatusError::ServiceUnavailable)
    }

    false
}
