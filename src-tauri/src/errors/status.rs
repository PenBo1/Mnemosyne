// ═══════════════════════════════════════════════════════════════
// Mnemosyne 项目状态码
// ═══════════════════════════════════════════════════════════════
// 设计原则：
//   - 按类别分前缀，每类 100 个码位 (x00-x99)
//   - 0xx = 成功/通用
//   - 1xx = 验证/输入错误
//   - 2xx = AI/LLM 错误
//   - 3xx = 文件系统错误
//   - 4xx = 数据库错误
//   - 5xx = 网络错误
//   - 6xx = 业务逻辑错误
//   - 7xx = 沙箱/安全错误
//   - 8xx = 任务/作业错误
//   - 9xx = 系统/内部错误
// ═══════════════════════════════════════════════════════════════

// ── 0xx: 成功 ──────────────────────────────────────────────
pub const OK: u16 = 0;
pub const CREATED: u16 = 1;
pub const UPDATED: u16 = 2;
pub const DELETED: u16 = 3;
pub const NO_CONTENT: u16 = 4;
pub const ACCEPTED: u16 = 5;

// ── 1xx: 验证/输入错误 ─────────────────────────────────────
pub const INVALID_INPUT: u16 = 100;
pub const MISSING_FIELD: u16 = 101;
pub const INVALID_FORMAT: u16 = 102;
pub const INVALID_PATH: u16 = 103;
pub const PATH_TRAVERSAL: u16 = 104;
pub const VALUE_OUT_OF_RANGE: u16 = 105;
pub const DUPLICATE_ENTRY: u16 = 106;
pub const INVALID_STATE: u16 = 107;
pub const UNPROCESSABLE: u16 = 108;
pub const PAYLOAD_TOO_LARGE: u16 = 109;
pub const INVALID_ENCODING: u16 = 110;
pub const INVALID_TYPE: u16 = 111;
pub const INVALID_LENGTH: u16 = 112;
pub const EMPTY_VALUE: u16 = 113;
pub const WHITESPACE_ONLY: u16 = 114;
pub const INVALID_CHARACTERS: u16 = 115;
pub const INVALID_ENUM_VALUE: u16 = 116;
pub const CIRCULAR_REFERENCE: u16 = 117;
pub const DEPENDENCY_MISSING: u16 = 118;

// ── 2xx: AI/LLM 错误 ──────────────────────────────────────
pub const PROVIDER_NOT_FOUND: u16 = 200;
pub const PROVIDER_UNAVAILABLE: u16 = 201;
pub const PROVIDER_TIMEOUT: u16 = 202;
pub const PROVIDER_RATE_LIMITED: u16 = 203;
pub const MODEL_NOT_FOUND: u16 = 210;
pub const MODEL_UNAVAILABLE: u16 = 211;
pub const MODEL_OVERLOADED: u16 = 212;
pub const MODEL_CONTEXT_EXCEEDED: u16 = 213;
pub const MODEL_OUTPUT_TRUNCATED: u16 = 214;
pub const API_KEY_INVALID: u16 = 220;
pub const API_KEY_EXPIRED: u16 = 221;
pub const API_KEY_REVOKED: u16 = 222;
pub const API_QUOTA_EXCEEDED: u16 = 230;
pub const API_CREDIT_EXHAUSTED: u16 = 231;
pub const API_DAILY_LIMIT: u16 = 232;
pub const TOKEN_LIMIT_INPUT: u16 = 240;
pub const TOKEN_LIMIT_OUTPUT: u16 = 241;
pub const TOKEN_LIMIT_TOTAL: u16 = 242;
pub const STREAM_DISCONNECTED: u16 = 250;
pub const STREAM_ERROR: u16 = 251;
pub const STREAM_TIMEOUT: u16 = 252;
pub const AGENT_NOT_RUNNING: u16 = 260;
pub const AGENT_BUSY: u16 = 261;
pub const AGENT_CANCELLED: u16 = 262;
pub const AGENT_TOOL_DENIED: u16 = 263;
pub const AGENT_MAX_TURNS: u16 = 264;
pub const PROMPT_TOO_LONG: u16 = 270;
pub const CONTENT_FILTERED: u16 = 271;
pub const SAFETY_REJECTION: u16 = 272;

// ── 3xx: 文件系统错误 ──────────────────────────────────────
pub const FILE_NOT_FOUND: u16 = 300;
pub const FILE_ALREADY_EXISTS: u16 = 301;
pub const FILE_READ_ERROR: u16 = 302;
pub const FILE_WRITE_ERROR: u16 = 303;
pub const FILE_DELETE_ERROR: u16 = 304;
pub const FILE_PERMISSION_DENIED: u16 = 305;
pub const FILE_LOCKED: u16 = 306;
pub const FILE_CORRUPT: u16 = 307;
pub const FILE_TOO_LARGE: u16 = 308;
pub const FILE_EMPTY: u16 = 309;
pub const DIRECTORY_NOT_FOUND: u16 = 310;
pub const DIRECTORY_NOT_EMPTY: u16 = 311;
pub const DIRECTORY_CREATE_FAILED: u16 = 312;
pub const DIRECTORY_DELETE_FAILED: u16 = 313;
pub const SYMLINK_ERROR: u16 = 320;
pub const WATCH_ERROR: u16 = 330;
pub const DISK_FULL: u16 = 340;
pub const DISK_READ_ONLY: u16 = 341;
pub const INODE_EXHAUSTED: u16 = 342;

// ── 4xx: 数据库错误 ────────────────────────────────────────
pub const DB_CONNECTION_FAILED: u16 = 400;
pub const DB_CONNECTION_LOST: u16 = 401;
pub const DB_QUERY_FAILED: u16 = 410;
pub const DB_QUERY_SYNTAX: u16 = 411;
pub const DB_CONSTRAINT_VIOLATION: u16 = 420;
pub const DB_UNIQUE_VIOLATION: u16 = 421;
pub const DB_FOREIGN_KEY_VIOLATION: u16 = 422;
pub const DB_NOT_NULL_VIOLATION: u16 = 423;
pub const DB_MIGRATION_FAILED: u16 = 430;
pub const DB_MIGRATION_PENDING: u16 = 431;
pub const DB_MIGRATION_CONFLICT: u16 = 432;
pub const DB_BUSY: u16 = 440;
pub const DB_LOCKED: u16 = 441;
pub const DB_TIMEOUT: u16 = 442;
pub const DB_CORRUPTION: u16 = 450;
pub const DB_INTEGRITY_ERROR: u16 = 451;
pub const DB_BACKUP_FAILED: u16 = 460;
pub const DB_RESTORE_FAILED: u16 = 461;

// ── 5xx: 网络错误 ──────────────────────────────────────────
pub const NETWORK_TIMEOUT: u16 = 500;
pub const NETWORK_UNREACHABLE: u16 = 501;
pub const DNS_RESOLUTION_FAILED: u16 = 510;
pub const DNS_TIMEOUT: u16 = 511;
pub const CONNECTION_REFUSED: u16 = 520;
pub const CONNECTION_RESET: u16 = 521;
pub const CONNECTION_CLOSED: u16 = 522;
pub const TLS_HANDSHAKE_FAILED: u16 = 530;
pub const TLS_CERT_EXPIRED: u16 = 531;
pub const TLS_CERT_UNTRUSTED: u16 = 532;
pub const HTTP_ERROR: u16 = 540;
pub const HTTP_TIMEOUT: u16 = 541;
pub const HTTP_REDIRECT_LOOP: u16 = 542;
pub const PROXY_ERROR: u16 = 550;
pub const PROXY_AUTH_REQUIRED: u16 = 551;

// ── 6xx: 业务逻辑错误 ──────────────────────────────────────
pub const NOVEL_NOT_FOUND: u16 = 600;
pub const NOVEL_ALREADY_EXISTS: u16 = 601;
pub const NOVEL_READONLY: u16 = 602;
pub const CHAPTER_NOT_FOUND: u16 = 610;
pub const CHAPTER_OUT_OF_RANGE: u16 = 611;
pub const CHAPTER_EMPTY: u16 = 612;
pub const SESSION_NOT_FOUND: u16 = 620;
pub const SESSION_EXPIRED: u16 = 621;
pub const SESSION_BUSY: u16 = 622;
pub const WORKSPACE_NOT_FOUND: u16 = 630;
pub const WORKSPACE_ALREADY_OPEN: u16 = 631;
pub const WORKSPACE_CORRUPT: u16 = 632;
pub const SKILL_NOT_FOUND: u16 = 640;
pub const SKILL_INVALID: u16 = 641;
pub const SKILL_DEPENDENCY_MISSING: u16 = 642;
pub const PROMPT_NOT_FOUND: u16 = 650;
pub const PROMPT_INVALID: u16 = 651;
pub const AGENT_CONFIG_NOT_FOUND: u16 = 660;
pub const AGENT_CONFIG_INVALID: u16 = 661;
pub const PIPELINE_BUSY: u16 = 670;
pub const PIPELINE_NOT_READY: u16 = 671;
pub const PIPELINE_STEP_FAILED: u16 = 672;
pub const MEMORY_NOT_FOUND: u16 = 680;
pub const MEMORY_DUPLICATE: u16 = 681;

// ── 7xx: 沙箱/安全错误 ─────────────────────────────────────
pub const SANDBOX_VIOLATION: u16 = 700;
pub const SANDBOX_TIMEOUT: u16 = 701;
pub const SANDBOX_BLOCKED: u16 = 702;
pub const SANDBOX_RESOURCE_EXCEEDED: u16 = 703;
pub const PERMISSION_DENIED: u16 = 710;
pub const AUTH_FAILED: u16 = 720;
pub const AUTH_TOKEN_EXPIRED: u16 = 721;
pub const AUTH_TOKEN_INVALID: u16 = 722;
pub const CSRF_FAILED: u16 = 730;
pub const INJECTION_DETECTED: u16 = 740;
pub const MALICIOUS_INPUT: u16 = 741;

// ── 8xx: 任务/作业错误 ─────────────────────────────────────
pub const TASK_NOT_FOUND: u16 = 800;
pub const TASK_CANCELLED: u16 = 801;
pub const TASK_FAILED: u16 = 802;
pub const TASK_TIMEOUT: u16 = 803;
pub const TASK_ALREADY_RUNNING: u16 = 804;
pub const TASK_DEPENDENCY_FAILED: u16 = 805;
pub const JOB_QUEUE_FULL: u16 = 810;
pub const JOB_QUEUE_TIMEOUT: u16 = 811;
pub const PROGRESS_UNKNOWN: u16 = 820;

// ── 9xx: 系统/内部错误 ─────────────────────────────────────
pub const INTERNAL_ERROR: u16 = 900;
pub const NOT_IMPLEMENTED: u16 = 901;
pub const UNAVAILABLE: u16 = 902;
pub const CONFIG_ERROR: u16 = 910;
pub const CONFIG_MISSING: u16 = 911;
pub const CONFIG_INVALID: u16 = 912;
pub const PLUGIN_ERROR: u16 = 920;
pub const PLUGIN_NOT_FOUND: u16 = 921;
pub const PLUGIN_VERSION_MISMATCH: u16 = 922;
pub const EVENT_EMIT_FAILED: u16 = 930;
pub const CHANNEL_CLOSED: u16 = 931;
pub const MEMORY_EXHAUSTED: u16 = 940;
pub const CPU_OVERLOAD: u16 = 941;
pub const UNKNOWN_ERROR: u16 = 999;
