import type { AgentSession } from "./agent-session.js";
import type { AgentSessionRuntimeDiagnostic, AgentSessionServices } from "./agent-session-services.js";
import type { SessionStartEvent } from "./extensions/index.js";
import type { CreateAgentSessionResult } from "./sdk.js";
import { SessionManager } from "./session-manager.js";
/**
 * Result returned by runtime creation.
 *
 * The caller gets the created session, its cwd-bound services, and all
 * diagnostics collected during setup.
 */
export interface CreateAgentSessionRuntimeResult extends CreateAgentSessionResult {
    services: AgentSessionServices;
    diagnostics: AgentSessionRuntimeDiagnostic[];
}
/**
 * Creates a full runtime for a target cwd and session manager.
 *
 * The factory closes over process-global fixed inputs, recreates cwd-bound
 * services for the effective cwd, resolves session options against those
 * services, and finally creates the AgentSession.
 */
export type CreateAgentSessionRuntimeFactory = (options: {
    cwd: string;
    agentDir: string;
    sessionManager: SessionManager;
    sessionStartEvent?: SessionStartEvent;
}) => Promise<CreateAgentSessionRuntimeResult>;
/**
 * Owns the current AgentSession plus its cwd-bound services.
 *
 * Session replacement methods tear down the current runtime first, then create
 * and apply the next runtime. If creation fails, the error is propagated to the
 * caller. The caller is responsible for user-facing error handling.
 */
export declare class AgentSessionRuntime {
    private _session;
    private _services;
    private readonly createRuntime;
    private _diagnostics;
    private _modelFallbackMessage?;
    constructor(_session: AgentSession, _services: AgentSessionServices, createRuntime: CreateAgentSessionRuntimeFactory, _diagnostics?: AgentSessionRuntimeDiagnostic[], _modelFallbackMessage?: string | undefined);
    get services(): AgentSessionServices;
    get session(): AgentSession;
    get cwd(): string;
    get diagnostics(): readonly AgentSessionRuntimeDiagnostic[];
    get modelFallbackMessage(): string | undefined;
    private emitBeforeSwitch;
    private emitBeforeFork;
    private teardownCurrent;
    private apply;
    switchSession(sessionPath: string, cwdOverride?: string): Promise<{
        cancelled: boolean;
    }>;
    newSession(options?: {
        parentSession?: string;
        setup?: (sessionManager: SessionManager) => Promise<void>;
    }): Promise<{
        cancelled: boolean;
    }>;
    fork(entryId: string): Promise<{
        cancelled: boolean;
        selectedText?: string;
    }>;
    importFromJsonl(inputPath: string, cwdOverride?: string): Promise<{
        cancelled: boolean;
    }>;
    dispose(): Promise<void>;
}
/**
 * Create the initial runtime from a runtime factory and initial session target.
 *
 * The same factory is stored on the returned AgentSessionRuntime and reused for
 * later /new, /resume, /fork, and import flows.
 */
export declare function createAgentSessionRuntime(createRuntime: CreateAgentSessionRuntimeFactory, options: {
    cwd: string;
    agentDir: string;
    sessionManager: SessionManager;
    sessionStartEvent?: SessionStartEvent;
}): Promise<AgentSessionRuntime>;
export { type AgentSessionRuntimeDiagnostic, type AgentSessionServices, type CreateAgentSessionFromServicesOptions, type CreateAgentSessionServicesOptions, createAgentSessionFromServices, createAgentSessionServices, } from "./agent-session-services.js";
//# sourceMappingURL=agent-session-runtime.d.ts.map