use crate::auth::auth_token::AuthToken;
use lru_cache::LruCache;
use std::time::{Duration, Instant};

pub struct TokensCache {
    timeout: Duration,
    cache: LruCache<AuthToken, (String, Instant)>,
}

impl TokensCache {
    pub fn new(capacity: usize, timeout: Duration) -> TokensCache {
        TokensCache {
            timeout,
            cache: LruCache::new(capacity),
        }
    }
    pub fn insert(&mut self, token: AuthToken, username: String) {
        self.cache.insert(token, (username, Instant::now()));
    }

    /// Check if the token is present and not stale, then return the username associated.
    pub fn get_username(&mut self, token: &AuthToken) -> Option<String> {
        if let Some((username, refreshed_at)) = self.cache.get_mut(token) {
            if refreshed_at.elapsed() < self.timeout {
                return Some(username.clone());
            }
        }
        None
    }
}
