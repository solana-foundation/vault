import {
    PublicKey,
    Transaction,
    TransactionMessage,
    VersionedTransaction,
    type Connection,
    type Signer,
    type TransactionInstruction,
} from '@solana/web3.js';
import type { WalletContextState } from '@solana/wallet-adapter-react';
import { toast } from 'sonner';

import { COMMITMENT } from './rpc';
import { explorerLink } from './env';

export interface SendIxsArgs {
    connection: Connection;
    wallet: WalletContextState;
    instructions: TransactionInstruction[];
    /** Extra non-wallet signers (e.g. ephemeral keypairs for newly created accounts). */
    signers?: Signer[];
    /** Skip confirmation toast — useful when the caller composes its own UX. */
    silent?: boolean;
    /** Toast message shown while the transaction is being submitted. */
    label?: string;
}

export async function sendIxs(args: SendIxsArgs): Promise<string> {
    const { connection, wallet, instructions, signers = [], silent, label } = args;
    if (!wallet.publicKey) throw new Error('Wallet not connected');
    if (!wallet.signTransaction) throw new Error('Wallet does not support signTransaction');

    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash(COMMITMENT);
    const messageV0 = new TransactionMessage({
        payerKey: wallet.publicKey,
        recentBlockhash: blockhash,
        instructions,
    }).compileToV0Message();
    const tx = new VersionedTransaction(messageV0);
    if (signers.length > 0) tx.sign(signers);

    const toastId = silent ? undefined : toast.loading(label ?? 'Sending transaction…');
    try {
        const signed = await wallet.signTransaction(tx);
        const sig = await connection.sendRawTransaction(signed.serialize(), {
            skipPreflight: false,
            preflightCommitment: COMMITMENT,
        });
        await connection.confirmTransaction(
            { signature: sig, blockhash, lastValidBlockHeight },
            COMMITMENT,
        );
        if (!silent) {
            toast.success('Transaction confirmed', {
                id: toastId,
                description: sig.slice(0, 24) + '…',
                action: {
                    label: 'Explorer',
                    onClick: () => window.open(explorerLink(sig, 'tx'), '_blank'),
                },
            });
        }
        return sig;
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        if (!silent) toast.error('Transaction failed', { id: toastId, description: message.slice(0, 240) });
        throw err;
    }
}

/**
 * Submit a legacy (non-versioned) transaction. Used when we need a partial signer flow that
 * works for older wallets — kept as a convenience but `sendIxs` is preferred.
 */
export async function sendLegacyIxs(args: SendIxsArgs): Promise<string> {
    const { connection, wallet, instructions, signers = [], silent, label } = args;
    if (!wallet.publicKey) throw new Error('Wallet not connected');
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash(COMMITMENT);
    const tx = new Transaction({ feePayer: wallet.publicKey, blockhash, lastValidBlockHeight });
    tx.add(...instructions);
    if (signers.length > 0) tx.partialSign(...signers);
    const toastId = silent ? undefined : toast.loading(label ?? 'Sending transaction…');
    try {
        const sig = await wallet.sendTransaction!(tx, connection, { skipPreflight: false });
        await connection.confirmTransaction({ signature: sig, blockhash, lastValidBlockHeight }, COMMITMENT);
        if (!silent) {
            toast.success('Transaction confirmed', {
                id: toastId,
                description: sig.slice(0, 24) + '…',
                action: { label: 'Explorer', onClick: () => window.open(explorerLink(sig, 'tx'), '_blank') },
            });
        }
        return sig;
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        if (!silent) toast.error('Transaction failed', { id: toastId, description: message.slice(0, 240) });
        throw err;
    }
}

export function ensureValidPubkey(s: string): PublicKey {
    return new PublicKey(s);
}
