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
