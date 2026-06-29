use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse<T> {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> IpcResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { status: super::status::OK, code: "OK".into(), message: "Success".into(), data: Some(data) }
    }

    pub fn created(data: T) -> Self {
        Self { status: super::status::CREATED, code: "CREATED".into(), message: "Resource created".into(), data: Some(data) }
    }

    pub fn updated(data: T) -> Self {
        Self { status: super::status::UPDATED, code: "UPDATED".into(), message: "Resource updated".into(), data: Some(data) }
    }

    pub fn deleted(data: T) -> Self {
        Self { status: super::status::DELETED, code: "DELETED".into(), message: "Resource deleted".into(), data: Some(data) }
    }

    pub fn accepted(data: T) -> Self {
        Self { status: super::status::ACCEPTED, code: "ACCEPTED".into(), message: "Request accepted".into(), data: Some(data) }
    }
}

impl IpcResponse<()> {
    pub fn no_content() -> Self {
        Self { status: super::status::NO_CONTENT, code: "NO_CONTENT".into(), message: "Success".into(), data: None }
    }

    pub fn ok_void() -> Self {
        Self::no_content()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_ok() {
        let resp = IpcResponse::ok("hello");
        assert_eq!(resp.status, 0);
        assert_eq!(resp.code, "OK");
        assert_eq!(resp.data.as_deref(), Some("hello"));
    }

    #[test]
    fn test_ipc_created() {
        let resp = IpcResponse::created(42);
        assert_eq!(resp.status, 1);
        assert_eq!(resp.code, "CREATED");
        assert_eq!(resp.data, Some(42));
    }

    #[test]
    fn test_ipc_no_content() {
        let resp = IpcResponse::<()>::no_content();
        assert_eq!(resp.status, 4);
        assert_eq!(resp.code, "NO_CONTENT");
        assert!(resp.data.is_none());
    }

    #[test]
    fn test_ipc_serialization() {
        let resp = IpcResponse::ok(vec![1, 2, 3]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":0"));
        assert!(json.contains("\"data\":[1,2,3]"));
    }
}
