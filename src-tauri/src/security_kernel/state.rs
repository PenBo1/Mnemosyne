use std::sync::Arc;

use super::audit::DbAuditHandler;
use super::kernel::SecurityKernel;
use crate::infrastructure::db::connection::Database;

#[derive(Clone)]
pub struct SecurityKernelState {
    kernel: Arc<SecurityKernel>,
}

impl SecurityKernelState {
    pub fn new() -> Self {
        Self {
            kernel: Arc::new(SecurityKernel::new()),
        }
    }

    /// 创建 SecurityKernel 并订阅 DbAuditHandler,把审计事件持久化到 SQLite。
    pub fn with_db(db: Database) -> Self {
        let kernel = SecurityKernel::new();
        kernel.audit_bus().subscribe(Box::new(DbAuditHandler::new(db)));
        Self {
            kernel: Arc::new(kernel),
        }
    }

    pub fn with_kernel(kernel: SecurityKernel) -> Self {
        Self {
            kernel: Arc::new(kernel),
        }
    }

    pub fn kernel(&self) -> &SecurityKernel {
        &*self.kernel
    }

    pub fn into_inner(self) -> Arc<SecurityKernel> {
        self.kernel
    }
}

impl Default for SecurityKernelState {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for SecurityKernelState {
    type Target = SecurityKernel;

    fn deref(&self) -> &Self::Target {
        &*self.kernel
    }
}