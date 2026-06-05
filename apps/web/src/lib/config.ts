import type { Address } from '@solana/kit';

const viteEnv = import.meta.env as unknown as {
    readonly VITE_PROGRAM_ID?: string;
};

export const PROGRAM_ID_STRING: string = viteEnv.VITE_PROGRAM_ID ?? '7M6pdteAnZmj9SEyzjsqUEqfcc4jqhpgLFF9dULDq1iP';
export const PROGRAM_ADDRESS = PROGRAM_ID_STRING as Address;

export const CLUSTER_STORAGE_KEY = 'vault-cluster';
