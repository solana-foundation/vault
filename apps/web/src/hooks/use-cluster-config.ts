import { useCluster } from '@solana/connector/react';

export interface ClusterWithUrl {
    id: string;
    label: string;
    url: string;
}

export function useClusterConfig(): ClusterWithUrl {
    const { cluster } = useCluster();
    if (!cluster) return { id: 'solana:localnet', label: 'Localnet', url: '/rpc' };
    return { id: cluster.id, label: cluster.label, url: cluster.url };
}
