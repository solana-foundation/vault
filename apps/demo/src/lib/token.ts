import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    MINT_SIZE,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountIdempotentInstruction,
    createInitializeMint2Instruction,
    createMintToInstruction,
    getAccount,
    getAssociatedTokenAddressSync,
    getMint,
    getMinimumBalanceForRentExemptMint,
} from '@solana/spl-token';
import {
    Keypair,
    PublicKey,
    SystemProgram,
    type Connection,
    type TransactionInstruction,
} from '@solana/web3.js';

export type TokenProgramKind = 'spl' | 'token-2022';

export function tokenProgramId(kind: TokenProgramKind): PublicKey {
    return kind === 'token-2022' ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID;
}

export function detectTokenProgramKind(programId: PublicKey | string): TokenProgramKind {
    const id = typeof programId === 'string' ? programId : programId.toBase58();
    return id === TOKEN_2022_PROGRAM_ID.toBase58() ? 'token-2022' : 'spl';
}

export interface CreateMintResult {
    mint: Keypair;
    instructions: TransactionInstruction[];
    signers: Keypair[];
}

export async function buildCreateMintInstructions(args: {
    connection: Connection;
    payer: PublicKey;
    decimals: number;
    mintAuthority: PublicKey;
    freezeAuthority?: PublicKey | null;
    tokenProgram: TokenProgramKind;
}): Promise<CreateMintResult> {
    const mint = Keypair.generate();
    const lamports = await getMinimumBalanceForRentExemptMint(args.connection);
    const programId = tokenProgramId(args.tokenProgram);
    const ixs: TransactionInstruction[] = [
        SystemProgram.createAccount({
            fromPubkey: args.payer,
            newAccountPubkey: mint.publicKey,
            lamports,
            space: MINT_SIZE,
            programId,
        }),
        createInitializeMint2Instruction(
            mint.publicKey,
            args.decimals,
            args.mintAuthority,
            args.freezeAuthority ?? null,
            programId,
        ),
    ];
    return { mint, instructions: ixs, signers: [mint] };
}

export function getAtaAddress(mint: PublicKey, owner: PublicKey, kind: TokenProgramKind): PublicKey {
    return getAssociatedTokenAddressSync(mint, owner, true, tokenProgramId(kind), ASSOCIATED_TOKEN_PROGRAM_ID);
}

export function buildCreateAtaIxIfNeeded(args: {
    payer: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    kind: TokenProgramKind;
}): { ata: PublicKey; ix: TransactionInstruction } {
    const ata = getAtaAddress(args.mint, args.owner, args.kind);
    const ix = createAssociatedTokenAccountIdempotentInstruction(
        args.payer,
        ata,
        args.owner,
        args.mint,
        tokenProgramId(args.kind),
        ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    return { ata, ix };
}

export function buildMintToInstruction(args: {
    mint: PublicKey;
    destination: PublicKey;
    authority: PublicKey;
    amount: bigint;
    kind: TokenProgramKind;
}): TransactionInstruction {
    return createMintToInstruction(args.mint, args.destination, args.authority, args.amount, [], tokenProgramId(args.kind));
}

export interface MintInfo {
    address: PublicKey;
    decimals: number;
    supply: bigint;
    mintAuthority: PublicKey | null;
    tokenProgram: TokenProgramKind;
}

export async function fetchMint(connection: Connection, mint: PublicKey): Promise<MintInfo> {
    const acc = await connection.getAccountInfo(mint);
    if (!acc) throw new Error(`Mint ${mint.toBase58()} not found`);
    const kind = detectTokenProgramKind(acc.owner);
    const m = await getMint(connection, mint, undefined, tokenProgramId(kind));
    return {
        address: mint,
        decimals: m.decimals,
        supply: m.supply,
        mintAuthority: m.mintAuthority,
        tokenProgram: kind,
    };
}

export async function fetchTokenAccountBalance(
    connection: Connection,
    address: PublicKey,
): Promise<bigint | null> {
    try {
        const acc = await connection.getAccountInfo(address);
        if (!acc) return null;
        const kind = detectTokenProgramKind(acc.owner);
        const a = await getAccount(connection, address, undefined, tokenProgramId(kind));
        return a.amount;
    } catch {
        return null;
    }
}
