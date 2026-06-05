import type { Address } from '@solana/kit';

const viteEnv = import.meta.env as unknown as {
    readonly VITE_PROGRAM_ID?: string;
};

export const PROGRAM_ID_STRING: string = viteEnv.VITE_PROGRAM_ID ?? '2kUpRoU8oGpstygkk3ZE51upGSq9UpkjNoEUiiQ88MMY';
export const PROGRAM_ADDRESS = PROGRAM_ID_STRING as Address;

export const CLUSTER_STORAGE_KEY = 'vault-cluster';
