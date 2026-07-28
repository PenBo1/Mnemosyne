/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 安全策略服务 - 提供工作区安全策略与审批流程管理
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { ipc } from "./index";

// ── 类型定义 ────────────────────────────────────────────────────────────────

export type TrustLevel = "unknown" | "trusted" | "enterprise" | "readonly" | "dangerous";

export type RiskLevel = "low" | "medium" | "high" | "critical";

export type PolicyDecision = "allow" | "require_approval" | "deny";

export type PolicySource = "temporary_override" | "user_override" | "workspace_override" | "global_policy";

export interface WorkspaceId {
  uuid: string;
}

export interface Workspace {
  id: WorkspaceId;
  root: string;
  trustLevel: TrustLevel;
  capabilities: string[];
  openedAt: string;
}

export interface ApprovalId {
  uuid: string;
}

export interface ApprovalToken {
  id: ApprovalId;
  workspace: WorkspaceId;
  actionHash: string;
  expire: string;
  riskLevel: RiskLevel;
  approvedBy: string | null;
  createdAt: string;
}

export interface ApprovalRequest {
  token: ApprovalToken;
  operation: Operation;
  workspace: WorkspaceId;
  reason: string;
}

export interface ApprovalResult {
  tokenId: ApprovalId;
  approved: boolean;
  approvedBy: string | null;
  message: string;
}

export interface PolicyEvaluation {
  decision: PolicyDecision;
  risk: RiskLevel;
  source: PolicySource;
  reason: string;
}

export type FsScope = "workspace" | "workspace_readonly" | "app_data" | "cache" | "temp" | "resources" | "templates" | "plugins" | "logs";

export type FsOperation = "read" | "write" | "delete" | "list" | "create_dir";

export interface NetworkEndpoint {
  host: string;
  port: number | null;
  protocol: "http" | "https" | "custom";
}

export type GitOperation = "status" | "commit" | "push" | "pull" | "fetch" | "diff" | "log" | "reset" | "branch" | "checkout" | "merge" | "rebase" | "stash" | "tag" | "remote" | "clone";

export type CargoOperation = "build" | "check" | "test" | "run" | "clippy" | "fmt" | "doc" | "clean" | "publish" | "install" | "update" | "add" | "remove";

export interface ShellScope {
  type: "git" | "python" | "node" | "cargo";
  operations?: GitOperation[] | CargoOperation[];
  scripts?: string[];
}

export interface NetworkScope {
  type: "provider" | "mcp" | "plugin";
  endpoint: NetworkEndpoint;
  allowLocalhost?: boolean;
}

export interface Operation {
  type: "filesystem" | "shell" | "network";
  scope: FsScope | ShellScope | NetworkScope;
  operation?: FsOperation;
  path?: string;
  command?: string;
  args?: string[];
  endpoint?: string;
  method?: string;
}

export interface ResourceUsage {
  workspaceId: WorkspaceId;
  fileCount: number;
  totalSizeBytes: number;
  operationCount: number;
  lastActivity: string;
}

export interface WorkspaceOverride {
  workspaceId: WorkspaceId;
  decision: PolicyDecision;
  reason: string;
  expireAt: string | null;
}

// ── 工作区管理 ────────────────────────────────────────────────────────────────

export async function workspaceOpen(path: string, trustLevel: TrustLevel): Promise<string> {
  return await ipc<string>("workspace_open", { path, trustLevel });
}

export async function workspaceClose(workspaceId: string): Promise<void> {
  return await ipc<void>("workspace_close", { workspaceId });
}

export async function workspaceGet(workspaceId: string): Promise<Workspace> {
  return await ipc<Workspace>("workspace_get", { workspaceId });
}

export async function workspaceGetActive(): Promise<Workspace | null> {
  return await ipc<Workspace | null>("workspace_get_active");
}

export async function workspaceSetActive(workspaceId: string): Promise<void> {
  return await ipc<void>("workspace_set_active", { workspaceId });
}

export async function workspaceList(): Promise<Workspace[]> {
  return await ipc<Workspace[]>("workspace_list");
}

export async function workspaceSetTrustLevel(workspaceId: string, trustLevel: TrustLevel): Promise<void> {
  return await ipc<void>("workspace_set_trust_level", { workspaceId, trustLevel });
}

// ── 策略评估 ────────────────────────────────────────────────────────────────

export async function policyEvaluate(operation: Operation, workspaceId: string): Promise<PolicyEvaluation> {
  return await ipc<PolicyEvaluation>("policy_evaluate", { operation, workspaceId });
}

export async function policySetWorkspaceOverride(workspaceId: string, decision: PolicyDecision, reason: string, ttlSeconds?: number): Promise<WorkspaceOverride> {
  return await ipc<WorkspaceOverride>("policy_set_workspace_override", { workspaceId, decision, reason, ttlSeconds });
}

export async function policyClearWorkspaceOverride(workspaceId: string): Promise<void> {
  return await ipc<void>("policy_clear_workspace_override", { workspaceId });
}

// ── 审批流程 ────────────────────────────────────────────────────────────────

export async function approvalCreate(operation: Operation, workspaceId: string, ttlSeconds?: number): Promise<ApprovalToken> {
  return await ipc<ApprovalToken>("approval_create", { operation, workspaceId, ttlSeconds });
}

export async function approvalValidate(tokenId: string, operation: Operation, workspaceId: string): Promise<boolean> {
  return await ipc<boolean>("approval_validate", { tokenId, operation, workspaceId });
}

export async function approvalConsume(tokenId: string): Promise<ApprovalResult> {
  return await ipc<ApprovalResult>("approval_consume", { tokenId });
}

export async function approvalReject(tokenId: string, reason?: string): Promise<ApprovalResult> {
  return await ipc<ApprovalResult>("approval_reject", { tokenId, reason });
}

// ── 资源监控 ────────────────────────────────────────────────────────────────

export async function resourceGetUsage(workspaceId: string): Promise<ResourceUsage> {
  return await ipc<ResourceUsage>("resource_get_usage", { workspaceId });
}

// ── 操作类型判断 ────────────────────────────────────────────────────────────────

export function isOperationFilesystem(op: Operation): boolean {
  return op.type === "filesystem";
}

export function isOperationShell(op: Operation): boolean {
  return op.type === "shell";
}

export function isOperationNetwork(op: Operation): boolean {
  return op.type === "network";
}

// ── 操作构造器 ────────────────────────────────────────────────────────────────

export function createFsOperation(scope: FsScope, operation: FsOperation, path: string): Operation {
  return { type: "filesystem", scope, operation, path };
}

export function createShellOperation(scope: ShellScope, command: string, args?: string[]): Operation {
  return { type: "shell", scope, command, args };
}

export function createNetworkOperation(scope: NetworkScope, endpoint: string, method: string): Operation {
  return { type: "network", scope, endpoint, method };
}