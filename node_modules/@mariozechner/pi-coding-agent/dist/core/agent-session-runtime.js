import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { emitSessionShutdownEvent } from "./extensions/runner.js";
import { assertSessionCwdExists } from "./session-cwd.js";
import { SessionManager } from "./session-manager.js";
function extractUserMessageText(content) {
    if (typeof content === "string") {
        return content;
    }
    return content
        .filter((part) => part.type === "text" && typeof part.text === "string")
        .map((part) => part.text)
        .join("");
}
/**
 * Owns the current AgentSession plus its cwd-bound services.
 *
 * Session replacement methods tear down the current runtime first, then create
 * and apply the next runtime. If creation fails, the error is propagated to the
 * caller. The caller is responsible for user-facing error handling.
 */
export class AgentSessionRuntime {
    _session;
    _services;
    createRuntime;
    _diagnostics;
    _modelFallbackMessage;
    constructor(_session, _services, createRuntime, _diagnostics = [], _modelFallbackMessage) {
        this._session = _session;
        this._services = _services;
        this.createRuntime = createRuntime;
        this._diagnostics = _diagnostics;
        this._modelFallbackMessage = _modelFallbackMessage;
    }
    get services() {
        return this._services;
    }
    get session() {
        return this._session;
    }
    get cwd() {
        return this._services.cwd;
    }
    get diagnostics() {
        return this._diagnostics;
    }
    get modelFallbackMessage() {
        return this._modelFallbackMessage;
    }
    async emitBeforeSwitch(reason, targetSessionFile) {
        const runner = this.session.extensionRunner;
        if (!runner?.hasHandlers("session_before_switch")) {
            return { cancelled: false };
        }
        const result = await runner.emit({
            type: "session_before_switch",
            reason,
            targetSessionFile,
        });
        return { cancelled: result?.cancel === true };
    }
    async emitBeforeFork(entryId) {
        const runner = this.session.extensionRunner;
        if (!runner?.hasHandlers("session_before_fork")) {
            return { cancelled: false };
        }
        const result = await runner.emit({
            type: "session_before_fork",
            entryId,
        });
        return { cancelled: result?.cancel === true };
    }
    async teardownCurrent() {
        await emitSessionShutdownEvent(this.session.extensionRunner);
        this.session.dispose();
    }
    apply(result) {
        if (process.cwd() !== result.services.cwd) {
            process.chdir(result.services.cwd);
        }
        this._session = result.session;
        this._services = result.services;
        this._diagnostics = result.diagnostics;
        this._modelFallbackMessage = result.modelFallbackMessage;
    }
    async switchSession(sessionPath, cwdOverride) {
        const beforeResult = await this.emitBeforeSwitch("resume", sessionPath);
        if (beforeResult.cancelled) {
            return beforeResult;
        }
        const previousSessionFile = this.session.sessionFile;
        const sessionManager = SessionManager.open(sessionPath, undefined, cwdOverride);
        assertSessionCwdExists(sessionManager, this.cwd);
        await this.teardownCurrent();
        this.apply(await this.createRuntime({
            cwd: sessionManager.getCwd(),
            agentDir: this.services.agentDir,
            sessionManager,
            sessionStartEvent: { type: "session_start", reason: "resume", previousSessionFile },
        }));
        return { cancelled: false };
    }
    async newSession(options) {
        const beforeResult = await this.emitBeforeSwitch("new");
        if (beforeResult.cancelled) {
            return beforeResult;
        }
        const previousSessionFile = this.session.sessionFile;
        const sessionDir = this.session.sessionManager.getSessionDir();
        const sessionManager = SessionManager.create(this.cwd, sessionDir);
        if (options?.parentSession) {
            sessionManager.newSession({ parentSession: options.parentSession });
        }
        await this.teardownCurrent();
        this.apply(await this.createRuntime({
            cwd: this.cwd,
            agentDir: this.services.agentDir,
            sessionManager,
            sessionStartEvent: { type: "session_start", reason: "new", previousSessionFile },
        }));
        if (options?.setup) {
            await options.setup(this.session.sessionManager);
            this.session.agent.state.messages = this.session.sessionManager.buildSessionContext().messages;
        }
        return { cancelled: false };
    }
    async fork(entryId) {
        const beforeResult = await this.emitBeforeFork(entryId);
        if (beforeResult.cancelled) {
            return { cancelled: true };
        }
        const selectedEntry = this.session.sessionManager.getEntry(entryId);
        if (!selectedEntry || selectedEntry.type !== "message" || selectedEntry.message.role !== "user") {
            throw new Error("Invalid entry ID for forking");
        }
        const previousSessionFile = this.session.sessionFile;
        const selectedText = extractUserMessageText(selectedEntry.message.content);
        if (this.session.sessionManager.isPersisted()) {
            const currentSessionFile = this.session.sessionFile;
            if (!currentSessionFile) {
                throw new Error("Persisted session is missing a session file");
            }
            const sessionDir = this.session.sessionManager.getSessionDir();
            if (!selectedEntry.parentId) {
                const sessionManager = SessionManager.create(this.cwd, sessionDir);
                sessionManager.newSession({ parentSession: currentSessionFile });
                await this.teardownCurrent();
                this.apply(await this.createRuntime({
                    cwd: this.cwd,
                    agentDir: this.services.agentDir,
                    sessionManager,
                    sessionStartEvent: { type: "session_start", reason: "fork", previousSessionFile },
                }));
                return { cancelled: false, selectedText };
            }
            const sourceManager = SessionManager.open(currentSessionFile, sessionDir);
            const forkedSessionPath = sourceManager.createBranchedSession(selectedEntry.parentId);
            if (!forkedSessionPath) {
                throw new Error("Failed to create forked session");
            }
            const sessionManager = SessionManager.open(forkedSessionPath, sessionDir);
            await this.teardownCurrent();
            this.apply(await this.createRuntime({
                cwd: sessionManager.getCwd(),
                agentDir: this.services.agentDir,
                sessionManager,
                sessionStartEvent: { type: "session_start", reason: "fork", previousSessionFile },
            }));
            return { cancelled: false, selectedText };
        }
        const sessionManager = this.session.sessionManager;
        if (!selectedEntry.parentId) {
            sessionManager.newSession({ parentSession: this.session.sessionFile });
        }
        else {
            sessionManager.createBranchedSession(selectedEntry.parentId);
        }
        await this.teardownCurrent();
        this.apply(await this.createRuntime({
            cwd: this.cwd,
            agentDir: this.services.agentDir,
            sessionManager,
            sessionStartEvent: { type: "session_start", reason: "fork", previousSessionFile },
        }));
        return { cancelled: false, selectedText };
    }
    async importFromJsonl(inputPath, cwdOverride) {
        const resolvedPath = resolve(inputPath);
        if (!existsSync(resolvedPath)) {
            throw new Error(`File not found: ${resolvedPath}`);
        }
        const sessionDir = this.session.sessionManager.getSessionDir();
        if (!existsSync(sessionDir)) {
            mkdirSync(sessionDir, { recursive: true });
        }
        const destinationPath = join(sessionDir, basename(resolvedPath));
        const beforeResult = await this.emitBeforeSwitch("resume", destinationPath);
        if (beforeResult.cancelled) {
            return beforeResult;
        }
        const previousSessionFile = this.session.sessionFile;
        if (resolve(destinationPath) !== resolvedPath) {
            copyFileSync(resolvedPath, destinationPath);
        }
        const sessionManager = SessionManager.open(destinationPath, sessionDir, cwdOverride);
        assertSessionCwdExists(sessionManager, this.cwd);
        await this.teardownCurrent();
        this.apply(await this.createRuntime({
            cwd: sessionManager.getCwd(),
            agentDir: this.services.agentDir,
            sessionManager,
            sessionStartEvent: { type: "session_start", reason: "resume", previousSessionFile },
        }));
        return { cancelled: false };
    }
    async dispose() {
        await emitSessionShutdownEvent(this.session.extensionRunner);
        this.session.dispose();
    }
}
/**
 * Create the initial runtime from a runtime factory and initial session target.
 *
 * The same factory is stored on the returned AgentSessionRuntime and reused for
 * later /new, /resume, /fork, and import flows.
 */
export async function createAgentSessionRuntime(createRuntime, options) {
    assertSessionCwdExists(options.sessionManager, options.cwd);
    const result = await createRuntime(options);
    if (process.cwd() !== result.services.cwd) {
        process.chdir(result.services.cwd);
    }
    return new AgentSessionRuntime(result.session, result.services, createRuntime, result.diagnostics, result.modelFallbackMessage);
}
export { createAgentSessionFromServices, createAgentSessionServices, } from "./agent-session-services.js";
//# sourceMappingURL=agent-session-runtime.js.map