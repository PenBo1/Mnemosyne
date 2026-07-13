/** 沙箱运行状态 */
export interface SandboxStatus {
  policy_name: string;
  security_level: string;
  fs_status: {
    work_dir: string;
    rule_count: number;
    allowed_patterns: string[];
    denied_patterns: string[];
  };
  exec_rule_count: number;
  net_status: {
    rule_count: number;
    allowed_hosts: string[];
    denied_hosts: string[];
  };
  timeout_secs: number;
}

/** 沙箱策略定义 */
export interface SandboxPolicy {
  name: string;
  description: string;
  level: string;
  fs_rules: Array<{ pattern: string; action: string; description?: string }>;
  exec_rules: Array<{ pattern: string; action: string; description?: string }>;
  net_rules: Array<{ host: string; action: string; description?: string }>;
  max_exec_timeout_secs: number;
  max_output_bytes: number;
  env_blacklist: string[];
  resource_limits: {
    max_memory_mb: number;
    max_cpu_secs: number;
    max_file_size_mb: number;
    max_open_files: number;
    max_nesting_depth: number;
  };
}
