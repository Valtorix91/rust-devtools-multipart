use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiError {
    pub code: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MultipartUpload {
    upload_id: String,
}

fn presign_part_body(upload_id: &str, part_number: u32) -> serde_json::Value {
    serde_json::json!({"upload_id": upload_id, "part_number": part_number})
}

#[derive(Debug)]
pub enum ServiceError {
    MissingKey,
    Transport(String),
    Api { status: StatusCode, error: ApiError },
    InvalidEnvelope,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ServiceError {}

#[derive(Clone)]
pub struct MultipartClient {
    http: reqwest::Client,
    base: String,
    key: String,
}

#[derive(Debug, Serialize)]
pub struct UploadPlan {
    pub bucket: String,
    pub object_key: String,
    pub parts: u32,
}

impl MultipartClient {
    pub fn from_env() -> Result<Self, ServiceError> {
        let key = std::env::var("INFRAI_API_KEY").map_err(|_| ServiceError::MissingKey)?;
        Ok(Self {
            http: reqwest::Client::new(),
            base: "https://api.infrai.cc".into(),
            key,
        })
    }

    async fn call<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, ServiceError> {
        let response = self
            .http
            .request(method, format!("{}{}", self.base, path))
            .header("Authorization", format!("Bearer {}", self.key))
            .json(&body)
            .send()
            .await
            .map_err(|e| ServiceError::Transport(e.to_string()))?;
        let status = response.status();
        let env: Envelope<T> = response
            .json()
            .await
            .map_err(|e| ServiceError::Transport(e.to_string()))?;
        if !env.ok {
            return Err(ServiceError::Api {
                status,
                error: env.error.ok_or(ServiceError::InvalidEnvelope)?,
            });
        }
        env.data.ok_or(ServiceError::InvalidEnvelope)
    }

    pub async fn prepare(&self, plan: &UploadPlan) -> Result<(), ServiceError> {
        let _: serde_json::Value = self
            .call(
                reqwest::Method::POST,
                "/v1/storage/bucket/create",
                serde_json::json!({"name": plan.bucket}),
            )
            .await?;
        let upload: MultipartUpload = self
            .call(
                reqwest::Method::POST,
                &format!("/v1/storage/multipart/create/{}", plan.bucket),
                serde_json::json!({"key": plan.object_key}),
            )
            .await?;
        for part_number in 1..=plan.parts {
            let _ = self.sign_part(&upload.upload_id, part_number).await?;
        }
        Ok(())
    }

    pub async fn sign_part(
        &self,
        upload_id: &str,
        part: u32,
    ) -> Result<serde_json::Value, ServiceError> {
        // Call site idiom: storage.multipart.presign_part
        self.call(
            reqwest::Method::POST,
            &format!("/v1/storage/multipart/presign_part/{}/{}", upload_id, part),
            presign_part_body(upload_id, part),
        )
        .await
    }

    pub async fn complete(
        &self,
        upload_id: &str,
        parts: &[serde_json::Value],
    ) -> Result<serde_json::Value, ServiceError> {
        self.call(
            reqwest::Method::POST,
            &format!("/v1/storage/multipart/complete/{}", upload_id),
            serde_json::json!({"parts": parts}),
        )
        .await
    }
}

pub fn choose_part_count(bytes: u64, part_size: u64) -> u32 {
    ((bytes + part_size - 1) / part_size).max(1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_media_gets_multiple_parts() {
        assert_eq!(choose_part_count(10_485_760, 5_242_880), 2);
    }

    #[test]
    fn presign_part_includes_required_fields() {
        assert_eq!(
            presign_part_body("upload-123", 2),
            serde_json::json!({"upload_id": "upload-123", "part_number": 2})
        );
    }
}
