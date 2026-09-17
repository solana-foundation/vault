import { CLUSTER_STORAGE_KEY } from './config';

const CLUSTER_PARAM: Record<string, string> = {
    'solana:devnet': 'devnet',
    'solana:localnet': 'custom',
    'solana:mainnet': '',
    'solana:testnet': 'testnet',
};

function currentClusterId(): string {
    if (typeof window === 'undefined') return 'solana:devnet';
    const stored = window.localStorage.getItem(CLUSTER_STORAGE_KEY);
    return stored && stored in CLUSTER_PARAM ? stored : 'solana:devnet';
}

export function explorerLink(value: string, kind: 'address' | 'tx' = 'address'): string {
    const path = kind === 'tx' ? 'tx' : 'address';
    const clusterParam = CLUSTER_PARAM[currentClusterId()];
    const url = `https://explorer.solana.com/${path}/${encodeURIComponent(value)}`;
    return clusterParam ? `${url}?cluster=${clusterParam}` : url;
}
