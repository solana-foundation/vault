import { type ClusterId, CLUSTER_STORAGE_KEY, isClusterId } from './config';

const CLUSTER_PARAM: Record<ClusterId, string> = {
    'solana:devnet': 'devnet',
    'solana:localnet': 'custom',
    'solana:mainnet': '',
    'solana:testnet': 'testnet',
};

function currentClusterId(): ClusterId {
    if (typeof window === 'undefined') return 'solana:devnet';
    const stored = window.localStorage.getItem(CLUSTER_STORAGE_KEY);
    return isClusterId(stored) ? stored : 'solana:devnet';
}

export function explorerLink(value: string, kind: 'address' | 'tx' = 'address'): string {
    const path = kind === 'tx' ? 'tx' : 'address';
    const clusterParam = CLUSTER_PARAM[currentClusterId()];
    const url = `https://explorer.solana.com/${path}/${encodeURIComponent(value)}`;
    return clusterParam ? `${url}?cluster=${clusterParam}` : url;
}
