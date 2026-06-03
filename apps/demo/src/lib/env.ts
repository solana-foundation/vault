import { PublicKey } from '@solana/web3.js';

export type Cluster = 'devnet' | 'mainnet-beta' | 'testnet' | 'localnet';

const DEFAULT_PROGRAM_ID = '7M6pdteAnZmj9SEyzjsqUEqfcc4jqhpgLFF9dULDq1iP';
const DEFAULT_RPC = 'https://api.devnet.solana.com';
const DEFAULT_CLUSTER: Cluster = 'devnet';

export const CLUSTER: Cluster = (process.env.NEXT_PUBLIC_CLUSTER as Cluster | undefined) ?? DEFAULT_CLUSTER;
export const RPC_URL: string = process.env.NEXT_PUBLIC_RPC_URL ?? DEFAULT_RPC;
export const PROGRAM_ID_STRING: string = process.env.NEXT_PUBLIC_PROGRAM_ID ?? DEFAULT_PROGRAM_ID;
export const PROGRAM_ID = new PublicKey(PROGRAM_ID_STRING);

export const EXPLORER_BASE = 'https://explorer.solana.com';
export function explorerLink(addressOrSig: string, kind: 'address' | 'tx' = 'address'): string {
    const cluster = CLUSTER === 'mainnet-beta' ? '' : `?cluster=${CLUSTER === 'localnet' ? 'custom' : CLUSTER}`;
    const path = kind === 'address' ? 'address' : 'tx';
    return `${EXPLORER_BASE}/${path}/${addressOrSig}${cluster}`;
}
