use axum::{
    async_trait,
    body::Body,
    extract::{FromRequest, Multipart, Request},
    http::StatusCode,
};
use taralli_primitives::PartialRequest;

/// A custom extractor that retains both the compressed and decompressed versions.
pub struct SubmittedRequest {
    pub partial_request: PartialRequest,
    pub proving_system_information_bytes: Vec<u8>,
}

#[async_trait]
impl<S> FromRequest<S> for SubmittedRequest
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| (e.status(), e.body_text()))?;

        let mut partial_request: Option<PartialRequest> = None;
        let mut proving_system_information_bytes: Option<Vec<u8>> = None;
        while let Some(part) = multipart
            .next_field()
            .await
            .map_err(|e| (e.status(), e.body_text()))?
        {
            match part.name() {
                Some("metadata") => {
                    let bytes = part
                        .bytes()
                        .await
                        .map_err(|e| (e.status(), e.body_text()))?;
                    partial_request = Some(serde_json::from_slice(&bytes).map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "Invalid JSON in partial request".to_string(),
                        )
                    })?);
                }
                Some("proving_system_information") => {
                    let bytes = part.bytes().await.map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "Error reading proving system information as binary".to_string(),
                        )
                    })?;
                    proving_system_information_bytes = Some(bytes.to_vec());
                }
                Some(_s) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Field not recognized on submission".to_string(),
                    ));
                }
                None => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Missing name for some submitted field".to_string(),
                    ))
                }
            }
        }

        // The `ok_or()` clauses below should never trigger, any error should've been filtered above.
        // Nonetheless, I'm opting for this rather than std::mem::MaybeUninit for the sake of making sure we're not returning something empty.
        Ok(SubmittedRequest {
            partial_request: partial_request.ok_or((
                StatusCode::BAD_REQUEST,
                "Missing partial request data".to_string(),
            ))?,
            proving_system_information_bytes: proving_system_information_bytes.ok_or((
                StatusCode::BAD_REQUEST,
                "Missing proving system information as binary".to_string(),
            ))?,
        })
    }
}
