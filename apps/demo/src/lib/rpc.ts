import { Connection, type Commitment } from '@solana/web3.js';
import { createSolanaRpc, createSolanaRpcSubscriptions } from '@solana/kit';

import { RPC_URL } from './env';

export const COMMITMENT: Commitment = 'confirmed';

let _connection: Connection | null = null;
export function getConnection(): Connection {
    _connection ??= new Connection(RPC_URL, { commitment: COMMITMENT });
    return _connection;
}

let _kitRpc: ReturnType<typeof createSolanaRpc> | null = null;
export function getKitRpc(): ReturnType<typeof createSolanaRpc> {
    _kitRpc ??= createSolanaRpc(RPC_URL);
    return _kitRpc;
}

let _kitSubs: ReturnType<typeof createSolanaRpcSubscriptions> | null = null;
export function getKitSubscriptions(): ReturnType<typeof createSolanaRpcSubscriptions> {
    if (!_kitSubs) {
        const wsUrl = RPC_URL.replace(/^http/, 'ws');
        _kitSubs = createSolanaRpcSubscriptions(wsUrl);
    }
    return _kitSubs;
}
