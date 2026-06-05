import { CLUSTER_STORAGE_KEY } from './config';

function currentClusterId(): string {
    if (typeof window === 'undefined') return 'solana:devnet';
    return window.localStorage.getItem(CLUSTER_STORAGE_KEY) ?? 'solana:devnet';
}

export function explorerLink(value: string, kind: 'address' | 'tx' = 'address'): string {
    const id = currentClusterId();
    const path = kind === 'tx' ? 'tx' : 'address';
    if (id === 'solana:mainnet') return `https://explorer.solana.com/${path}/${value}`;
    const clusterParam = id === 'solana:localnet' ? 'custom' : id.replace('solana:', '');
    return `https://explorer.solana.com/${path}/${value}?cluster=${clusterParam}`;
}
