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
