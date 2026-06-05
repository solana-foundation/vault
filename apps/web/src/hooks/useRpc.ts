import { createSolanaRpc, createSolanaRpcSubscriptions } from '@solana/kit';
import { useMemo } from 'react';

import { useClusterConfig } from '@/hooks/use-cluster-config';

function wsUrlFromHttp(httpUrl: string): string {
    if (httpUrl.startsWith('/')) {
        // Localnet goes through the Vite proxy; PubSub is served on a separate port,
        // routed via the dedicated `/rpc-ws` proxy entry.
        const protocol = window.location.protocol === 'https:' ? 'wss://' : 'ws://';
        return `${protocol}${window.location.host}/rpc-ws`;
    }
    return httpUrl.replace(/^https?:\/\//, match => (match === 'https://' ? 'wss://' : 'ws://'));
}

export function useRpc() {
    const { url } = useClusterConfig();
    return useMemo(() => createSolanaRpc(url), [url]);
}

export function useRpcSubscriptions() {
    const { url } = useClusterConfig();
    return useMemo(() => createSolanaRpcSubscriptions(wsUrlFromHttp(url)), [url]);
}
