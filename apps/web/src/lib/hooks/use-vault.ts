import { useCallback, useEffect, useState } from 'react';
import {
    fetchEncodedAccount,
    getBase58Decoder,
    getBase64Encoder,
    type Address,
    type Base58EncodedBytes,
} from '@solana/kit';
import { getRequestDecoder, REQUEST_DISCRIMINATOR, type Request } from '@solana/vault';

import { PROGRAM_ADDRESS } from '@/lib/config';
import { decodeVaultData, parseExtensions, type ParsedExtension } from '@/lib/extensions';
import { deriveVaultPdas, type VaultPdas } from '@/lib/program';
import {
    fetchMintInfo,
    fetchTokenAccountBalance,
    type MintInfo,
    type TokenProgramKind,
    type VaultRpc,
} from '@/lib/token';
import { useRpc } from '@/hooks/useRpc';

export interface VaultState {
    shareMint: Address;
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
    address: Address;
    type: 'deposit' | 'redeem';
    state: 'pending' | 'claimable' | 'canceled';
    owner: Address;
    amount: bigint;
    price: bigint;
    createdAt: bigint;
    operator: Address | null;
    raw: Request;
}

const base64Encoder = getBase64Encoder();
const base58Decoder = getBase58Decoder();

async function fetchVaultRequests(rpc: VaultRpc, vault: Address): Promise<VaultRequest[]> {
    const discriminator = base58Decoder.decode(REQUEST_DISCRIMINATOR) as Base58EncodedBytes;
    const accounts = await rpc
        .getProgramAccounts(PROGRAM_ADDRESS, {
            encoding: 'base64',
            filters: [
                { memcmp: { bytes: discriminator, encoding: 'base58', offset: 0n } },
                { memcmp: { bytes: vault as unknown as Base58EncodedBytes, encoding: 'base58', offset: 8n } },
            ],
        })
        .send();

    const decoder = getRequestDecoder();
    return accounts
        .map(({ account, pubkey }) => {
            const [base64Data] = account.data;
            const r = decoder.decode(new Uint8Array(base64Encoder.encode(base64Data)));
            return {
                address: pubkey,
                amount: r.amount,
                createdAt: r.createdAt,
                operator: r.operator.__option === 'Some' ? r.operator.value : null,
                owner: r.owner,
                price: r.price,
                raw: r,
                state:
                    r.requestState === 0
                        ? ('pending' as const)
                        : r.requestState === 1
                          ? ('claimable' as const)
                          : ('canceled' as const),
                type: r.requestType === 0 ? ('deposit' as const) : ('redeem' as const),
            };
        })
        .sort((a, b) => Number(a.createdAt - b.createdAt));
}

export function useVault(shareMintAddress: string | null | undefined) {
    const rpc = useRpc();
    const [state, setState] = useState<VaultState | null>(null);
    const [requests, setRequests] = useState<VaultRequest[] | null>(null);
    const [error, setError] = useState<Error | null>(null);
    const [loading, setLoading] = useState(false);
    const [refreshKey, setRefreshKey] = useState(0);

    const refresh = useCallback(() => setRefreshKey(k => k + 1), []);

    useEffect(() => {
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
                const shareMint = shareMintAddress as Address;
                const pdas = await deriveVaultPdas(shareMint);
                const vaultAcc = await fetchEncodedAccount(rpc, pdas.vault);
                if (!vaultAcc.exists) throw new Error(`Vault not found for share mint ${shareMintAddress}`);
                const data = new Uint8Array(vaultAcc.data);
                const base = decodeVaultData(data);
                const extensions = parseExtensions(data);
                const [assetMint, shareMintInfo, reserveBalance, pendingBalance, vaultRequests] = await Promise.all([
                    fetchMintInfo(rpc, base.assetMint),
                    fetchMintInfo(rpc, shareMint),
                    fetchTokenAccountBalance(rpc, pdas.reserve),
                    fetchTokenAccountBalance(rpc, pdas.pendingVault),
                    fetchVaultRequests(rpc, pdas.vault),
                ]);
                if (cancelled) return;
                setState({
                    assetMint,
                    assetTokenProgram: assetMint.tokenProgram,
                    base,
                    extensions,
                    pdas,
                    pendingBalance,
                    rawData: data,
                    reserveBalance,
                    shareMint,
                    shareMintInfo,
                    shareTokenProgram: shareMintInfo.tokenProgram,
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
    }, [rpc, shareMintAddress, refreshKey]);

    return { error, loading, refresh, requests, state };
}

export function useTokenBalance(address: string | null | undefined) {
    const rpc = useRpc();
    const [balance, setBalance] = useState<bigint | null>(null);
    const [refreshKey, setRefreshKey] = useState(0);
    const refresh = useCallback(() => setRefreshKey(k => k + 1), []);

    useEffect(() => {
        if (!address) {
            setBalance(null);
            return;
        }
        let cancelled = false;
        void (async () => {
            try {
                const b = await fetchTokenAccountBalance(rpc, address as Address);
                if (!cancelled) setBalance(b);
            } catch {
                if (!cancelled) setBalance(null);
            }
        })();
        return () => {
            cancelled = true;
        };
    }, [address, rpc, refreshKey]);
    return { balance, refresh };
}
