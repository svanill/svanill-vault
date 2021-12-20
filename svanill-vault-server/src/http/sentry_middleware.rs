use actix_web::dev::ServiceRequest;
use sentry::protocol::{Event, Request};

/// Drop this file for sentry-actix once they support actix-web >= 4

pub fn enrich_sentry_event_with_request_metadata(req: &ServiceRequest) {
    let (tx, sentry_req) = sentry_request_from_http(req);

    sentry::configure_scope(|scope| {
        scope.set_transaction(tx.as_deref());

        scope.add_event_processor(Box::new(move |mut event: Event<'static>| {
            if event.request.is_none() {
                event.request = Some(sentry_req.clone());
            }
            Some(event)
        }))
    });
}

/// Build a Sentry request struct from the HTTP request
fn sentry_request_from_http(request: &ServiceRequest) -> (Option<String>, Request) {
    let transaction = if let Some(name) = request.match_name() {
        Some(String::from(name))
    } else {
        request.match_pattern()
    };

    let sentry_req = Request {
        url: format!(
            "{}://{}{}",
            request.connection_info().scheme(),
            request.connection_info().host(),
            request.uri()
        )
        .parse()
        .ok(),
        method: Some(request.method().to_string()),
        headers: request
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect(),
        ..Default::default()
    };

    (transaction, sentry_req)
}
