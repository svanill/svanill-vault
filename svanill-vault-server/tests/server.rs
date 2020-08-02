#[cfg(test)]
use actix_web::{test, App};
use svanill_vault_server::auth::tokens_cache::TokensCache;
use svanill_vault_server::errors::ApiError;
use svanill_vault_server::http::handlers::config_handlers;

#[actix_rt::test]
async fn test_not_found() {
    let tokens_cache = TokensCache::default();
    let mut app =
        test::init_service(App::new().data(tokens_cache).configure(config_handlers)).await;
    let req = test::TestRequest::post().uri("/not-exist").to_request();
    let resp = test::call_service(&mut app, req).await;

    //let resp: ApiError = test::read_response_json(&mut app, req).await;

    //assert_eq!(404, resp.http_status);
    //assert_eq!(404, resp.error.code);
}
