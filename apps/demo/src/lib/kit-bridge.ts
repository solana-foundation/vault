import { AccountRole, type AccountMeta, type Instruction } from '@solana/kit';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';

/**
 * Convert an @solana/kit Instruction (produced by the codama-generated client) into
 * a web3.js v1 TransactionInstruction so we can submit it via wallet-adapter.
 */
export function kitIxToWeb3(ix: Instruction): TransactionInstruction {
    const accounts = (ix.accounts ?? []) as readonly AccountMeta[];
    return new TransactionInstruction({
        programId: new PublicKey(ix.programAddress),
        keys: accounts.map((a) => ({
            pubkey: new PublicKey(a.address),
            isSigner: a.role === AccountRole.READONLY_SIGNER || a.role === AccountRole.WRITABLE_SIGNER,
            isWritable: a.role === AccountRole.WRITABLE || a.role === AccountRole.WRITABLE_SIGNER,
        })),
        data: Buffer.from(ix.data ?? new Uint8Array()),
    });
}
