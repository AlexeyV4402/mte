use std::time::{Duration, Instant};

use dashmap::DashMap;
use game_lib::content::types::PlayerIdType;
use game_lib::network::types::VerifyTokenType;

pub struct AuthRegistry {
    tokens: DashMap<VerifyTokenType, (PlayerIdType, Instant)>,
}

impl AuthRegistry {
    pub fn new() -> Self {
        Self {
            tokens: DashMap::new(),
        }
    }

    pub fn add_token(&self, token: VerifyTokenType, player_id: PlayerIdType) {
        self.tokens.insert(token, (player_id, Instant::now()));
    }

    pub fn verify_and_remove(&self, token: VerifyTokenType, timeout_secs: u64) -> PlayerIdType {
        let timeout = Duration::from_secs(timeout_secs);

        let removed = self.tokens.remove_if(&token, |_, (_, _)| true);

        match removed {
            Some((_, (id, created))) => {
                if created.elapsed() < timeout {
                    id
                } else {
                    0
                }
            }
            None => 0,
        }
    }

    pub fn cleanup_expired(&self, timeout_secs: u64) {
        let timeout = Duration::from_secs(timeout_secs);
        self.tokens
            .retain(|_, (_, created)| created.elapsed() < timeout);
    }
}
