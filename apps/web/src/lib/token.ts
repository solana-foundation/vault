import { getCreateAccountInstruction } from '@solana-program/system';
import {
    fetchMint as fetchSplMint,
    findAssociatedTokenPda as findSplAta,
    getCreateAssociatedTokenIdempotentInstruction as getCreateSplAtaInstruction,
    getInitializeMint2Instruction as getInitializeSplMint2Instruction,
    getMintSize,
    getMintToInstruction as getSplMintToInstruction,
    TOKEN_PROGRAM_ADDRESS,
} from '@solana-program/token';
import {
    fetchMint as fetchToken2022Mint,
    findAssociatedTokenPda as findToken2022Ata,
    getCreateAssociatedTokenIdempotentInstruction as getCreateToken2022AtaInstruction,
    getInitializeMint2Instruction as getInitializeToken2022Mint2Instruction,
    getMintToInstruction as getToken2022MintToInstruction,
    TOKEN_2022_PROGRAM_ADDRESS,
} from '@solana-program/token-2022';
import {
    fetchEncodedAccount,
    generateKeyPairSigner,
    type Address,
    type Instruction,
    type Rpc,
    type SolanaRpcApi,
    type TransactionSigner,
} from '@solana/kit';

export type TokenProgramKind = 'spl' | 'token-2022';
export type VaultRpc = Rpc<SolanaRpcApi>;

export function tokenProgramAddress(kind: TokenProgramKind): Address {
    return kind === 'token-2022' ? TOKEN_2022_PROGRAM_ADDRESS : TOKEN_PROGRAM_ADDRESS;
}

export function detectTokenProgramKind(programAddress: Address | string): TokenProgramKind {
    return programAddress === TOKEN_2022_PROGRAM_ADDRESS ? 'token-2022' : 'spl';
}

export async function getAtaAddress(mint: Address, owner: Address, kind: TokenProgramKind): Promise<Address> {
    const seeds = { mint, owner, tokenProgram: tokenProgramAddress(kind) };
    const [ata] = kind === 'token-2022' ? await findToken2022Ata(seeds) : await findSplAta(seeds);
    return ata;
}

export interface CreateAtaResult {
    ata: Address;
    instruction: Instruction;
}

export async function buildCreateAtaIdempotentIx(args: {
    payer: TransactionSigner;
    owner: Address;
    mint: Address;
    kind: TokenProgramKind;
}): Promise<CreateAtaResult> {
    const ata = await getAtaAddress(args.mint, args.owner, args.kind);
    const input = {
        ata,
        mint: args.mint,
        owner: args.owner,
        payer: args.payer,
        tokenProgram: tokenProgramAddress(args.kind),
    };
    const instruction =
        args.kind === 'token-2022' ? getCreateToken2022AtaInstruction(input) : getCreateSplAtaInstruction(input);
    return { ata, instruction };
}

export function buildMintToIx(args: {
    mint: Address;
    destination: Address;
    authority: TransactionSigner;
    amount: bigint;
    kind: TokenProgramKind;
}): Instruction {
    const input = {
        amount: args.amount,
        mint: args.mint,
        mintAuthority: args.authority,
        token: args.destination,
    };
    return args.kind === 'token-2022' ? getToken2022MintToInstruction(input) : getSplMintToInstruction(input);
}

export interface CreateMintResult {
    mint: TransactionSigner;
    instructions: Instruction[];
}

export async function buildCreateMintInstructions(args: {
    rpc: VaultRpc;
    payer: TransactionSigner;
    decimals: number;
    mintAuthority: Address;
    kind: TokenProgramKind;
}): Promise<CreateMintResult> {
    const mint = await generateKeyPairSigner();
    const space = BigInt(getMintSize());
    const lamports = await args.rpc.getMinimumBalanceForRentExemption(space).send();
    const programAddress = tokenProgramAddress(args.kind);

    const createAccountIx = getCreateAccountInstruction({
        lamports,
        newAccount: mint,
        payer: args.payer,
        programAddress,
        space,
    });

    const initMintInput = {
        decimals: args.decimals,
        freezeAuthority: null,
        mint: mint.address,
        mintAuthority: args.mintAuthority,
    };
    const initializeMintIx =
        args.kind === 'token-2022'
            ? getInitializeToken2022Mint2Instruction(initMintInput)
            : getInitializeSplMint2Instruction(initMintInput);

    return { mint, instructions: [createAccountIx, initializeMintIx] };
}

export interface MintInfo {
    address: Address;
    decimals: number;
    supply: bigint;
    mintAuthority: Address | null;
    tokenProgram: TokenProgramKind;
}

export async function fetchMintInfo(rpc: VaultRpc, mint: Address): Promise<MintInfo> {
    const account = await fetchEncodedAccount(rpc, mint);
    if (!account.exists) throw new Error(`Mint ${mint} not found`);
    const kind = detectTokenProgramKind(account.programAddress);
    const decoded = kind === 'token-2022' ? await fetchToken2022Mint(rpc, mint) : await fetchSplMint(rpc, mint);
    const mintAuthority = decoded.data.mintAuthority;
    return {
        address: mint,
        decimals: decoded.data.decimals,
        mintAuthority: mintAuthority.__option === 'Some' ? mintAuthority.value : null,
        supply: decoded.data.supply,
        tokenProgram: kind,
    };
}

export async function fetchTokenAccountBalance(rpc: VaultRpc, address: Address): Promise<bigint | null> {
    const account = await fetchEncodedAccount(rpc, address);
    if (!account.exists) return null;
    const data = account.data;
    if (data.length < 72) return null;
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    return view.getBigUint64(64, true);
}
