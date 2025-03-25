use axum::{
    async_trait,
    body::Body,
    extract::{FromRequest, Multipart, Request},
    http::StatusCode,
};
use taralli_primitives::compression_utils::intents::{PartialComputeOffer, PartialComputeRequest};

/// A custom extracted type that contains both all Request data of `Request<I: ProvingSystemInformation>`.
/// Although we use a vector of bytes to reprent the compressed proving system.
pub struct ExtractedRequest {
    pub partial_request: PartialComputeRequest,
    pub system_bytes: Vec<u8>,
}

#[async_trait]
impl<S> FromRequest<S> for ExtractedRequest
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| (e.status(), e.body_text()))?;

        let mut partial_request: Option<PartialComputeRequest> = None;
        let mut system_bytes: Option<Vec<u8>> = None;
        while let Some(part) = multipart
            .next_field()
            .await
            .map_err(|e| (e.status(), e.body_text()))?
        {
            match part.name() {
                Some("partial_request") => {
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
                Some("system_bytes") => {
                    let bytes = part.bytes().await.map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "Error reading system information as binary".to_string(),
                        )
                    })?;
                    system_bytes = Some(bytes.to_vec());
                }
                Some(s) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("Field not recognized on submission {s}"),
                    ));
                }
                None => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Missing name for some multipart submitted field".to_string(),
                    ))
                }
            }
        }

        // The `ok_or()` clauses below should never trigger, any error should've been filtered above.
        // Nonetheless, I'm opting for this rather than std::mem::MaybeUninit for the sake of making sure we're not returning something empty.
        Ok(ExtractedRequest {
            partial_request: partial_request.ok_or((
                StatusCode::BAD_REQUEST,
                "Missing partial request data".to_string(),
            ))?,
            system_bytes: system_bytes.ok_or((
                StatusCode::BAD_REQUEST,
                "Missing system information as binary".to_string(),
            ))?,
        })
    }
}

pub struct ExtractedOffer {
    pub partial_offer: PartialComputeOffer,
    pub system_bytes: Vec<u8>,
}

#[async_trait]
impl<S> FromRequest<S> for ExtractedOffer
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| (e.status(), e.body_text()))?;

        let mut partial_offer: Option<PartialComputeOffer> = None;
        let mut system_bytes: Option<Vec<u8>> = None;
        while let Some(part) = multipart
            .next_field()
            .await
            .map_err(|e| (e.status(), e.body_text()))?
        {
            match part.name() {
                Some("partial_offer") => {
                    let bytes = part
                        .bytes()
                        .await
                        .map_err(|e| (e.status(), e.body_text()))?;
                    partial_offer = Some(serde_json::from_slice(&bytes).map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "Invalid JSON in partial offer".to_string(),
                        )
                    })?);
                }
                Some("system_bytes") => {
                    let bytes = part.bytes().await.map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "Error reading system information as binary".to_string(),
                        )
                    })?;
                    system_bytes = Some(bytes.to_vec());
                }
                Some(s) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("Field not recognized on submission {s}"),
                    ));
                }
                None => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Missing name for some multipart submitted field".to_string(),
                    ))
                }
            }
        }

        // The `ok_or()` clauses below should never trigger, any error should've been filtered above.
        // Nonetheless, I'm opting for this rather than std::mem::MaybeUninit for the sake of making sure we're not returning something empty.
        Ok(ExtractedOffer {
            partial_offer: partial_offer.ok_or((
                StatusCode::BAD_REQUEST,
                "Missing partial request data".to_string(),
            ))?,
            system_bytes: system_bytes.ok_or((
                StatusCode::BAD_REQUEST,
                "Missing system information as binary".to_string(),
            ))?,
        })
    }
}
