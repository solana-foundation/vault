import {
    ASYNC_VAULT_ERROR__ARITHMETIC_ERROR,
    ASYNC_VAULT_ERROR__FEE_BPS_EXCEEDED,
    ASYNC_VAULT_ERROR__INVALID_ASSET_MINT,
    ASYNC_VAULT_ERROR__INVALID_FEE_RECIPIENT,
    ASYNC_VAULT_ERROR__INVALID_REQUEST,
    ASYNC_VAULT_ERROR__INVALID_REQUEST_TYPE,
    ASYNC_VAULT_ERROR__INVALID_SHARE_MINT,
    ASYNC_VAULT_ERROR__MINTS_SHOULD_BE_DIFFERENT,
    ASYNC_VAULT_ERROR__NO_PENDING_AUTHORITY,
    ASYNC_VAULT_ERROR__PAUSED_VAULT,
    ASYNC_VAULT_ERROR__REDEMPTION_AMOUNT_BELOW_MINIMUM,
    ASYNC_VAULT_ERROR__REDEMPTION_QUEUE_OUT_OF_ORDER,
    ASYNC_VAULT_ERROR__REDEMPTIONS_PAUSED,
    ASYNC_VAULT_ERROR__REQUEST_NOT_CLAIMABLE,
    ASYNC_VAULT_ERROR__REQUEST_NOT_PENDING,
    ASYNC_VAULT_ERROR__SHARE_MINT_SUPPLY_SHOULD_BE_ZERO,
    ASYNC_VAULT_ERROR__SUBSCRIPTION_AMOUNT_BELOW_MINIMUM,
    ASYNC_VAULT_ERROR__SUBSCRIPTION_QUEUE_OUT_OF_ORDER,
    ASYNC_VAULT_ERROR__SUBSCRIPTIONS_PAUSED,
    ASYNC_VAULT_ERROR__UNAUTHORIZED_SIGNER,
    ASYNC_VAULT_ERROR__UNINITIALIZED_VAULT,
    ASYNC_VAULT_ERROR__VAULT_ALREADY_INITIALIZED,
} from '@solana/vault';
import {
    isSolanaError,
    SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM,
    SOLANA_ERROR__JSON_RPC__SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE,
    unwrapSimulationError,
} from '@solana/kit';

const VAULT_ERROR_MESSAGES: Record<number, string> = {
    [ASYNC_VAULT_ERROR__UNAUTHORIZED_SIGNER]: 'Unauthorized signer',
    [ASYNC_VAULT_ERROR__UNINITIALIZED_VAULT]: 'Vault is not initialized',
    [ASYNC_VAULT_ERROR__PAUSED_VAULT]: 'Vault is paused',
    [ASYNC_VAULT_ERROR__VAULT_ALREADY_INITIALIZED]: 'Vault is already initialized',
    [ASYNC_VAULT_ERROR__FEE_BPS_EXCEEDED]: 'Fee basis points exceed maximum',
    [ASYNC_VAULT_ERROR__ARITHMETIC_ERROR]: 'Arithmetic error',
    [ASYNC_VAULT_ERROR__MINTS_SHOULD_BE_DIFFERENT]: 'Asset and share mints must be different',
    [ASYNC_VAULT_ERROR__SHARE_MINT_SUPPLY_SHOULD_BE_ZERO]: 'Share mint supply must be zero before create',
    [ASYNC_VAULT_ERROR__NO_PENDING_AUTHORITY]: 'No pending authority invitation',
    [ASYNC_VAULT_ERROR__INVALID_FEE_RECIPIENT]: 'Fee recipient account is invalid',
    [ASYNC_VAULT_ERROR__REQUEST_NOT_PENDING]: 'Request is not in a pending state',
    [ASYNC_VAULT_ERROR__REQUEST_NOT_CLAIMABLE]: 'Request is not claimable yet',
    [ASYNC_VAULT_ERROR__INVALID_REQUEST]: 'Request address is not valid',
    [ASYNC_VAULT_ERROR__INVALID_REQUEST_TYPE]: 'Invalid request type for this instruction',
    [ASYNC_VAULT_ERROR__INVALID_ASSET_MINT]: 'Asset mint is not valid',
    [ASYNC_VAULT_ERROR__INVALID_SHARE_MINT]: 'Share mint is not valid',
    [ASYNC_VAULT_ERROR__SUBSCRIPTIONS_PAUSED]: 'Subscriptions are paused',
    [ASYNC_VAULT_ERROR__REDEMPTIONS_PAUSED]: 'Redemptions are paused',
    [ASYNC_VAULT_ERROR__SUBSCRIPTION_AMOUNT_BELOW_MINIMUM]: 'Deposit is below the minimum subscription threshold',
    [ASYNC_VAULT_ERROR__REDEMPTION_AMOUNT_BELOW_MINIMUM]: 'Redemption is below the minimum redemption threshold',
    [ASYNC_VAULT_ERROR__SUBSCRIPTION_QUEUE_OUT_OF_ORDER]: 'Deposit request is not next in the subscription queue',
    [ASYNC_VAULT_ERROR__REDEMPTION_QUEUE_OUT_OF_ORDER]: 'Redeem request is not next in the redemption queue',
};

const FALLBACK_TX_FAILED_MESSAGE = 'Transaction failed';
const MAX_LOG_LINES = 12;

export interface TransactionErrorDetails {
    readonly logs: readonly string[];
    readonly message: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null;
}

function getErrorMessage(error: unknown): string {
    if (typeof error === 'string') return error;
    if (error instanceof Error) return error.message;
    if (isRecord(error) && typeof error.message === 'string') return error.message;
    return '';
}

function getErrorCause(error: unknown): unknown {
    if (error instanceof Error) return error.cause;
    if (isRecord(error)) return error.cause;
    return undefined;
}

function tryDecodePayload(payload: string): string | null {
    if (typeof globalThis.atob !== 'function') return null;
    try {
        return globalThis.atob(payload);
    } catch {
        return null;
    }
}

function parseCustomProgramCodeFromString(message: string): number | null {
    const customErrorMatch = message.match(/custom program error:\s*(#\d+|0x[0-9a-fA-F]+|\d+)/i);
    if (customErrorMatch) {
        const value = customErrorMatch[1].trim();
        if (value.startsWith('#')) {
            const parsed = Number.parseInt(value.slice(1), 10);
            return Number.isNaN(parsed) ? null : parsed;
        }
        if (value.toLowerCase().startsWith('0x')) {
            const parsed = Number.parseInt(value.slice(2), 16);
            return Number.isNaN(parsed) ? null : parsed;
        }
        const parsed = Number.parseInt(value, 10);
        return Number.isNaN(parsed) ? null : parsed;
    }

    const decodePayloadMatch = message.match(/@solana\/errors decode --\s+-?\d+\s+'([^']+)'/);
    if (decodePayloadMatch) {
        const decodedPayload = tryDecodePayload(decodePayloadMatch[1]);
        if (decodedPayload) {
            const params = new URLSearchParams(decodedPayload);
            const code = params.get('code');
            if (code) {
                const parsed = Number.parseInt(code, 10);
                if (!Number.isNaN(parsed)) return parsed;
            }
        }
    }

    return null;
}

function parseInstructionErrorCode(value: unknown): number | null {
    if (!Array.isArray(value) || value.length < 2) return null;
    const instructionError = value[1];
    if (isRecord(instructionError) && typeof instructionError.Custom === 'number') return instructionError.Custom;
    return null;
}

function parseCustomProgramCode(error: unknown, visited = new Set<object>()): number | null {
    if (isRecord(error)) {
        if (visited.has(error)) return null;
        visited.add(error);
    }

    const simulationCause = unwrapSimulationError(error);
    if (simulationCause !== error) {
        const simulationCode = parseCustomProgramCode(simulationCause, visited);
        if (simulationCode !== null) return simulationCode;
    }

    if (isSolanaError(error, SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM)) {
        return error.context.code;
    }

    if (isRecord(error)) {
        const context = error.context;
        if (isRecord(context) && typeof context.code === 'number') return context.code;

        const directInstructionErrorCode = parseInstructionErrorCode(error.InstructionError);
        if (directInstructionErrorCode !== null) return directInstructionErrorCode;

        const err = error.err;
        if (isRecord(err)) {
            const errInstructionErrorCode = parseInstructionErrorCode(err.InstructionError);
            if (errInstructionErrorCode !== null) return errInstructionErrorCode;
        }

        const data = error.data;
        if (isRecord(data)) {
            const dataInstructionErrorCode = parseCustomProgramCode(data, visited);
            if (dataInstructionErrorCode !== null) return dataInstructionErrorCode;
        }
    }

    const message = getErrorMessage(error);
    const parsedMessageCode = message ? parseCustomProgramCodeFromString(message) : null;
    if (parsedMessageCode !== null) return parsedMessageCode;

    const cause = getErrorCause(error);
    return cause === undefined ? null : parseCustomProgramCode(cause, visited);
}

function getVaultProgramErrorMessage(code: number | null): string | null {
    if (code === null) return null;
    return VAULT_ERROR_MESSAGES[code] ?? null;
}

function normalizeLogs(value: unknown): readonly string[] {
    if (!Array.isArray(value)) return [];
    return value.filter((line): line is string => typeof line === 'string' && line.trim().length > 0);
}

function collectLogs(error: unknown, visited = new Set<object>()): readonly string[] {
    const logs: string[] = [];

    if (isRecord(error)) {
        if (visited.has(error)) return [];
        visited.add(error);

        if (isSolanaError(error, SOLANA_ERROR__JSON_RPC__SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE)) {
            logs.push(...normalizeLogs(error.context.logs));
        }

        logs.push(...normalizeLogs(error.logs));

        const context = error.context;
        if (isRecord(context)) logs.push(...normalizeLogs(context.logs));

        const data = error.data;
        if (isRecord(data)) logs.push(...normalizeLogs(data.logs));
    }

    const simulationCause = unwrapSimulationError(error);
    if (simulationCause !== error) logs.push(...collectLogs(simulationCause, visited));

    const cause = getErrorCause(error);
    if (cause !== undefined) logs.push(...collectLogs(cause, visited));

    return [...new Set(logs)];
}

function isPreflightFailure(error: unknown, visited = new Set<object>()): boolean {
    if (isRecord(error)) {
        if (visited.has(error)) return false;
        visited.add(error);
    }

    if (isSolanaError(error, SOLANA_ERROR__JSON_RPC__SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE)) {
        return true;
    }

    const cause = getErrorCause(error);
    return cause === undefined ? false : isPreflightFailure(cause, visited);
}

export function getTransactionErrorDetails(error: unknown): TransactionErrorDetails {
    const message = getErrorMessage(error).trim();
    const vaultMessage = getVaultProgramErrorMessage(parseCustomProgramCode(error));

    if (vaultMessage) {
        return { logs: collectLogs(error), message: `${FALLBACK_TX_FAILED_MESSAGE}: ${vaultMessage}` };
    }

    if (/user rejected|rejected the request|declined|cancelled/i.test(message)) {
        return { logs: [], message: 'Transaction was rejected in wallet' };
    }

    if (isPreflightFailure(error)) {
        return {
            logs: collectLogs(error),
            message: `${FALLBACK_TX_FAILED_MESSAGE}: transaction simulation failed`,
        };
    }

    if (
        message === FALLBACK_TX_FAILED_MESSAGE ||
        message.startsWith(`${FALLBACK_TX_FAILED_MESSAGE}:`) ||
        message === 'Transaction was rejected in wallet'
    ) {
        return { logs: collectLogs(error), message };
    }

    return {
        logs: collectLogs(error),
        message: message ? `${FALLBACK_TX_FAILED_MESSAGE}: ${message}` : FALLBACK_TX_FAILED_MESSAGE,
    };
}

export function formatTransactionError(error: unknown): string {
    return getTransactionErrorDetails(error).message;
}

export function formatTransactionErrorWithLogs(error: unknown): string {
    const { logs, message } = getTransactionErrorDetails(error);
    if (logs.length === 0) return message;

    const visibleLogs = logs.slice(-MAX_LOG_LINES);
    const omittedCount = logs.length - visibleLogs.length;
    const omittedLine = omittedCount > 0 ? [`... ${omittedCount} earlier log lines omitted`] : [];

    return [message, '', 'Logs:', ...omittedLine, ...visibleLogs].join('\n');
}
