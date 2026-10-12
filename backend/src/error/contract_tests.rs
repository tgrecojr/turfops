//! Every handler error answers JSON `{"error"}` with the status its kind calls for.

use super::*;

async fn render(err: TurfOpsError) -> (StatusCode, serde_json::Value) {
    let response = err.into_response();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).expect("a JSON body"))
}

fn http_error() -> reqwest::Error {
    reqwest::Client::new()
        .get("not a url")
        .build()
        .expect_err("an unparseable URL")
}

// @spec DEPLOY-HTTP-005
#[tokio::test]
async fn each_error_kind_has_its_status_and_a_json_error_body() {
    let cases = [
        (TurfOpsError::NotFound("No such plant.".into()), 404),
        (
            TurfOpsError::InvalidData("Rate must be above 0.".into()),
            400,
        ),
        (TurfOpsError::not_configured(Source::Forecast), 503),
        (TurfOpsError::unavailable(Source::WeatherLake, "io"), 503),
        (TurfOpsError::Conflict("Already on the shelf.".into()), 409),
        (TurfOpsError::Config("bad".into()), 500),
        (TurfOpsError::Internal("profile row without id".into()), 500),
        (TurfOpsError::Database(sqlx::Error::RowNotFound), 500),
        (TurfOpsError::Http(http_error()), 500),
        (TurfOpsError::Io(std::io::Error::other("disk")), 500),
        (
            TurfOpsError::Json(serde_json::from_str::<serde_json::Value>("{").unwrap_err()),
            500,
        ),
    ];
    for (err, expected) in cases {
        let kind = format!("{err:?}");
        let (status, body) = render(err).await;
        assert_eq!(status.as_u16(), expected, "{kind}");
        let object = body.as_object().expect("an object");
        assert_eq!(object.len(), 1, "{kind}: only `error`: {body}");
        assert!(object["error"].is_string(), "{kind}: {body}");
    }

    let (_, body) = render(TurfOpsError::NotFound("No such plant.".into())).await;
    assert_eq!(body["error"], "No such plant.", "a 4xx carries its message");
    let (_, body) = render(TurfOpsError::Io(std::io::Error::other("disk"))).await;
    assert_eq!(body["error"], SERVER_ERROR_MESSAGE);
}
