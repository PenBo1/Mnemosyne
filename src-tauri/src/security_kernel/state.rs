//! ═══════════════════════════════════════════════════════════════════════════
//! state - 安全内核状态模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use super::audit::EventHandler;
use super::kernel::SecurityKernel;

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

    /// 创建 SecurityKernel 并订阅外部注入的审计处理器。
    ///
    /// 处理器实现位于 infrastructure 层（数据库持久化），
    /// 由装配层注入，避免 security_kernel 反向依赖 infrastructure。
    pub fn with_audit_handler(handler: Box<dyn EventHandler>) -> Self {
        let kernel = SecurityKernel::new();
        kernel.audit_bus().subscribe(handler);
        // C16: 订阅处理器后启动后台派发 task,
        // 避免 emit 同步调用处理器的 SQLite 写入阻塞 tokio worker。
        kernel.audit_bus().start_dispatch_task();
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
        &self.kernel
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
        &self.kernel
    }
}