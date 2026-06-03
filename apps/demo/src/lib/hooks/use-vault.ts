'use client';

import * as React from 'react';
import { useConnection } from '@solana/wallet-adapter-react';
import { type Connection, PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';

import { decodeVaultData, parseExtensions, type ParsedExtension } from '@/lib/extensions';
import { PROGRAM_ID } from '@/lib/env';
import { fetchMint, fetchTokenAccountBalance, type MintInfo, type TokenProgramKind } from '@/lib/token';
import { deriveVaultPdas, type VaultPdas } from '@/lib/program';
import { getRequestDecoder, REQUEST_DISCRIMINATOR, type Request } from '@solana/vault';

export interface VaultState {
    shareMint: PublicKey;
    pdas: VaultPdas;
    base: ReturnType<typeof decodeVaultData>;
    extensions: ParsedExtension[];
    rawData: Uint8Array;
    assetMint: MintInfo;
    shareMintInfo: MintInfo;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
    reserveBalance: bigint | null;
    pendingBalance: bigint | null;
}

export interface VaultRequest {
    address: PublicKey;
    type: 'deposit' | 'redeem';
    state: 'pending' | 'claimable' | 'canceled';
    owner: PublicKey;
    amount: bigint;
    price: bigint;
    createdAt: bigint;
    operator: PublicKey | null;
    raw: Request;
}

async function fetchVaultRequests(connection: Connection, vault: PublicKey): Promise<VaultRequest[]> {
    const accounts = await connection.getProgramAccounts(PROGRAM_ID, {
        filters: [
            { memcmp: { offset: 0, bytes: bs58.encode(REQUEST_DISCRIMINATOR) } },
            { memcmp: { offset: 8, bytes: vault.toBase58() } },
        ],
    });
    const decoder = getRequestDecoder();
    return accounts
        .map(({ pubkey, account }) => {
            const r = decoder.decode(new Uint8Array(account.data));
            return {
                address: pubkey,
                type: r.requestType === 0 ? ('deposit' as const) : ('redeem' as const),
                state:
                    r.requestState === 0
                        ? ('pending' as const)
                        : r.requestState === 1
                          ? ('claimable' as const)
                          : ('canceled' as const),
                owner: new PublicKey(r.owner),
                amount: r.amount,
                price: r.price,
                createdAt: r.createdAt,
                operator: r.operator.__option === 'Some' ? new PublicKey(r.operator.value) : null,
                raw: r,
            };
        })
        .sort((a, b) => Number(a.createdAt - b.createdAt));
}

export function useVault(shareMintAddress: string | null | undefined) {
    const { connection } = useConnection();
    const [state, setState] = React.useState<VaultState | null>(null);
    const [requests, setRequests] = React.useState<VaultRequest[] | null>(null);
    const [error, setError] = React.useState<Error | null>(null);
    const [loading, setLoading] = React.useState(false);
    const [refreshKey, setRefreshKey] = React.useState(0);

    const refresh = React.useCallback(() => setRefreshKey(k => k + 1), []);

    React.useEffect(() => {
        if (!shareMintAddress) {
            setState(null);
            setRequests(null);
            return;
        }
        let cancelled = false;
        const run = async () => {
            setLoading(true);
            setError(null);
            try {
                const shareMint = new PublicKey(shareMintAddress);
                const pdas = await deriveVaultPdas(shareMint);
                const vaultAcc = await connection.getAccountInfo(pdas.vault);
                if (!vaultAcc) throw new Error(`Vault not found for share mint ${shareMintAddress}`);
                const data = new Uint8Array(vaultAcc.data);
                const base = decodeVaultData(data);
                const extensions = parseExtensions(data);
                const [assetMint, shareMintInfo, reserveBalance, pendingBalance, vaultRequests] = await Promise.all([
                    fetchMint(connection, new PublicKey(base.assetMint)),
                    fetchMint(connection, shareMint),
                    fetchTokenAccountBalance(connection, pdas.reserve),
                    fetchTokenAccountBalance(connection, pdas.pendingVault),
                    fetchVaultRequests(connection, pdas.vault),
                ]);
                if (cancelled) return;
                setState({
                    shareMint,
                    pdas,
                    base,
                    extensions,
                    rawData: data,
                    assetMint,
                    shareMintInfo,
                    assetTokenProgram: assetMint.tokenProgram,
                    shareTokenProgram: shareMintInfo.tokenProgram,
                    reserveBalance,
                    pendingBalance,
                });
                setRequests(vaultRequests);
            } catch (err) {
                if (!cancelled) setError(err as Error);
            } finally {
                if (!cancelled) setLoading(false);
            }
        };
        void run();
        return () => {
            cancelled = true;
        };
    }, [connection, shareMintAddress, refreshKey]);

    return { state, requests, loading, error, refresh };
}

export function useTokenBalance(address: string | null | undefined) {
    const { connection } = useConnection();
    const [balance, setBalance] = React.useState<bigint | null>(null);
    const [refreshKey, setRefreshKey] = React.useState(0);
    const refresh = React.useCallback(() => setRefreshKey(k => k + 1), []);

    React.useEffect(() => {
        if (!address) {
            setBalance(null);
            return;
        }
        let cancelled = false;
        void (async () => {
            try {
                const b = await fetchTokenAccountBalance(connection, new PublicKey(address));
                if (!cancelled) setBalance(b);
            } catch {
                if (!cancelled) setBalance(null);
            }
        })();
        return () => {
            cancelled = true;
        };
    }, [address, connection, refreshKey]);
    return { balance, refresh };
}
