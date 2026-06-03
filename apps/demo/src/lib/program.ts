import { createNoopSigner, type Address } from '@solana/kit';
import { Keypair, PublicKey, type TransactionInstruction } from '@solana/web3.js';

import {
    findPendingVaultPda,
    findReservePda,
    findVaultPda,
    getAcceptAuthorityInvitationInstruction,
    getApproveRequestInstruction,
    getCancelQueuedDepositRequestInstruction,
    getCancelQueuedRedemptionRequestInstruction,
    getCancelRequestInstruction,
    getClaimInstruction,
    getCreateDepositRequestInstruction,
    getCreateRedeemRequestInstruction,
    getCreateVaultInstructionAsync,
    getInitializeDepositFeeInstruction,
    getInitializeMinRedemptionInstruction,
    getInitializeMinSubscriptionInstruction,
    getInitializePausableRedemptionsInstruction,
    getInitializePausableSubscriptionsInstruction,
    getInitializeRedemptionQueueInstruction,
    getInitializeSubscriptionQueueInstruction,
    getInitializeVaultInstruction,
    getInitializeWithdrawalFeeInstruction,
    getInviteNewAuthorityInstruction,
    getRejectRequestInstructionAsync,
    getSetOperatorInstruction,
    getSkipCanceledQueueRequestInstruction,
    getUpdateDepositFeeInstruction,
    getUpdateMinRedemptionInstruction,
    getUpdateMinSubscriptionInstruction,
    getUpdatePausableRedemptionsInstruction,
    getUpdatePausableSubscriptionsInstruction,
    getUpdateVaultInstructionAsync,
    getUpdateVaultNavInstruction,
    getUpdateWithdrawalFeeInstruction,
    getWithdrawAssetsInstruction,
    type FeeTypeArgs,
} from '@solana/vault';

import { PROGRAM_ID_STRING } from './env';
import { kitIxToWeb3 } from './kit-bridge';
import { tokenProgramId, type TokenProgramKind } from './token';

const PROGRAM_ADDRESS = PROGRAM_ID_STRING as Address;
const SYSTEM_PROGRAM = '11111111111111111111111111111111' as Address;

function addr(p: PublicKey | string): Address {
    return (typeof p === 'string' ? p : p.toBase58()) as Address;
}

function noop(p: PublicKey | string) {
    return createNoopSigner(addr(p));
}

export interface VaultPdas {
    vault: PublicKey;
    reserve: PublicKey;
    pendingVault: PublicKey;
}

export async function deriveVaultPdas(shareMint: PublicKey | string): Promise<VaultPdas> {
    const seeds = { shareMint: addr(shareMint) };
    const [v, r, p] = await Promise.all([
        findVaultPda(seeds, { programAddress: PROGRAM_ADDRESS }),
        findReservePda(seeds, { programAddress: PROGRAM_ADDRESS }),
        findPendingVaultPda(seeds, { programAddress: PROGRAM_ADDRESS }),
    ]);
    return {
        vault: new PublicKey(v[0]),
        reserve: new PublicKey(r[0]),
        pendingVault: new PublicKey(p[0]),
    };
}

export interface CreateVaultParams {
    payer: PublicKey;
    mintAuthority: PublicKey;
    assetMint: PublicKey;
    shareMint: PublicKey;
    authority: PublicKey;
    feeRecipient: PublicKey;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export async function buildCreateVaultIx(p: CreateVaultParams): Promise<TransactionInstruction> {
    const ix = await getCreateVaultInstructionAsync(
        {
            payer: noop(p.payer),
            mintAuthority: noop(p.mintAuthority),
            assetMint: addr(p.assetMint),
            shareMint: addr(p.shareMint),
            assetTokenProgram: addr(tokenProgramId(p.assetTokenProgram)),
            shareTokenProgram: addr(tokenProgramId(p.shareTokenProgram)),
            authority: addr(p.authority),
            feeRecipient: addr(p.feeRecipient),
            systemProgram: SYSTEM_PROGRAM,
        },
        { programAddress: PROGRAM_ADDRESS },
    );
    return kitIxToWeb3(ix);
}

export function buildInitializeVaultIx(authority: PublicKey, vault: PublicKey): TransactionInstruction {
    return kitIxToWeb3(
        getInitializeVaultInstruction(
            { authority: noop(authority), vault: addr(vault) },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export async function buildUpdateVaultIxAsync(args: {
    authority: PublicKey;
    shareMint: PublicKey;
    paused: boolean;
    feeRecipient: PublicKey;
}): Promise<TransactionInstruction> {
    const ix = await getUpdateVaultInstructionAsync(
        {
            authority: noop(args.authority),
            shareMint: addr(args.shareMint),
            paused: args.paused,
            feeRecipient: addr(args.feeRecipient),
        },
        { programAddress: PROGRAM_ADDRESS },
    );
    return kitIxToWeb3(ix);
}

export function buildUpdateNavIx(args: {
    authority: PublicKey;
    vault: PublicKey;
    nav: bigint;
}): TransactionInstruction {
    return kitIxToWeb3(
        getUpdateVaultNavInstruction(
            { authority: noop(args.authority), vault: addr(args.vault), updatedNav: args.nav },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildInviteAuthorityIx(args: {
    authority: PublicKey;
    vault: PublicKey;
    newAuthority: PublicKey;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInviteNewAuthorityInstruction(
            {
                authority: noop(args.authority),
                vault: addr(args.vault),
                newAuthority: addr(args.newAuthority),
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildAcceptAuthorityIx(args: { newAuthority: PublicKey; vault: PublicKey }): TransactionInstruction {
    return kitIxToWeb3(
        getAcceptAuthorityInvitationInstruction(
            { newAuthority: noop(args.newAuthority), vault: addr(args.vault) },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildWithdrawAssetsIx(args: {
    authority: PublicKey;
    assetMint: PublicKey;
    vault: PublicKey;
    vaultTokenAccount: PublicKey;
    recipientTokenAccount: PublicKey;
    amount: bigint;
    assetTokenProgram: TokenProgramKind;
}): TransactionInstruction {
    return kitIxToWeb3(
        getWithdrawAssetsInstruction(
            {
                authority: noop(args.authority),
                assetMint: addr(args.assetMint),
                vault: addr(args.vault),
                vaultTokenAccount: addr(args.vaultTokenAccount),
                recipientTokenAccount: addr(args.recipientTokenAccount),
                assetTokenProgram: addr(tokenProgramId(args.assetTokenProgram)),
                amount: args.amount,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export interface CreateRequestParams {
    user: PublicKey;
    vault: PublicKey;
    request: PublicKey;
    assetMint: PublicKey;
    shareMint: PublicKey;
    pendingVault: PublicKey;
    userTokenAccount?: PublicKey;
    userShareAccount?: PublicKey;
    amount: bigint;
    operator?: PublicKey | null;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export function buildCreateDepositRequestIx(p: CreateRequestParams): TransactionInstruction {
    if (!p.userTokenAccount) throw new Error('userTokenAccount required for deposit');
    return kitIxToWeb3(
        getCreateDepositRequestInstruction(
            {
                user: noop(p.user),
                assetMint: addr(p.assetMint),
                shareMint: addr(p.shareMint),
                vault: addr(p.vault),
                request: noop(p.request),
                userTokenAccount: addr(p.userTokenAccount),
                pendingVault: addr(p.pendingVault),
                assetTokenProgram: addr(tokenProgramId(p.assetTokenProgram)),
                systemProgram: SYSTEM_PROGRAM,
                args: {
                    amount: p.amount,
                    operator: p.operator ? addr(p.operator) : null,
                },
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildCreateRedeemRequestIx(p: CreateRequestParams): TransactionInstruction {
    if (!p.userShareAccount) throw new Error('userShareAccount required for redeem');
    return kitIxToWeb3(
        getCreateRedeemRequestInstruction(
            {
                user: noop(p.user),
                assetMint: addr(p.assetMint),
                shareMint: addr(p.shareMint),
                vault: addr(p.vault),
                request: noop(p.request),
                userShareAccount: addr(p.userShareAccount),
                shareTokenProgram: addr(tokenProgramId(p.shareTokenProgram)),
                systemProgram: SYSTEM_PROGRAM,
                args: {
                    amount: p.amount,
                    operator: p.operator ? addr(p.operator) : null,
                },
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export interface ApproveRequestParams {
    authority: PublicKey;
    assetMint: PublicKey;
    shareMint: PublicKey;
    vault: PublicKey;
    request: PublicKey;
    vaultTokenAccount: PublicKey;
    pendingVault: PublicKey;
    assetTokenProgram: TokenProgramKind;
}

export function buildApproveRequestIx(p: ApproveRequestParams): TransactionInstruction {
    return kitIxToWeb3(
        getApproveRequestInstruction(
            {
                authority: noop(p.authority),
                assetMint: addr(p.assetMint),
                shareMint: addr(p.shareMint),
                vault: addr(p.vault),
                request: addr(p.request),
                vaultTokenAccount: addr(p.vaultTokenAccount),
                pendingVault: addr(p.pendingVault),
                assetTokenProgram: addr(tokenProgramId(p.assetTokenProgram)),
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export interface RejectRequestParams {
    authority: PublicKey;
    assetMint: PublicKey;
    shareMint: PublicKey;
    request: PublicKey;
    user: PublicKey;
    requestType: 'deposit' | 'redeem';
    userTokenAccount?: PublicKey;
    assetPendingVault?: PublicKey;
    userShareAccount?: PublicKey;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export async function buildRejectRequestIx(p: RejectRequestParams): Promise<TransactionInstruction> {
    const ix = await getRejectRequestInstructionAsync(
        {
            authority: noop(p.authority),
            assetMint: addr(p.assetMint),
            shareMint: addr(p.shareMint),
            request: addr(p.request),
            user: addr(p.user),
            ...(p.requestType === 'deposit'
                ? {
                      userTokenAccount: p.userTokenAccount ? addr(p.userTokenAccount) : undefined,
                      assetPendingVault: p.assetPendingVault ? addr(p.assetPendingVault) : undefined,
                      assetTokenProgram: addr(tokenProgramId(p.assetTokenProgram)),
                  }
                : {
                      userShareAccount: p.userShareAccount ? addr(p.userShareAccount) : undefined,
                      shareTokenProgram: addr(tokenProgramId(p.shareTokenProgram)),
                  }),
            systemProgram: SYSTEM_PROGRAM,
        },
        { programAddress: PROGRAM_ADDRESS },
    );
    return kitIxToWeb3(ix);
}

export interface CancelRequestParams {
    user: PublicKey;
    assetMint: PublicKey;
    shareMint: PublicKey;
    vault: PublicKey;
    request: PublicKey;
    requestType: 'deposit' | 'redeem';
    userTokenAccount?: PublicKey;
    assetPendingVault?: PublicKey;
    userShareAccount?: PublicKey;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export function buildCancelRequestIx(p: CancelRequestParams): TransactionInstruction {
    return kitIxToWeb3(
        getCancelRequestInstruction(
            {
                user: noop(p.user),
                assetMint: addr(p.assetMint),
                shareMint: addr(p.shareMint),
                vault: addr(p.vault),
                request: addr(p.request),
                userTokenAccount: p.userTokenAccount ? addr(p.userTokenAccount) : undefined,
                assetPendingVault: p.assetPendingVault ? addr(p.assetPendingVault) : undefined,
                userShareAccount: p.userShareAccount ? addr(p.userShareAccount) : undefined,
                assetTokenProgram: addr(tokenProgramId(p.assetTokenProgram)),
                shareTokenProgram: addr(tokenProgramId(p.shareTokenProgram)),
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildCancelQueuedDepositIx(args: {
    user: PublicKey;
    assetMint: PublicKey;
    shareMint: PublicKey;
    vault: PublicKey;
    request: PublicKey;
    userTokenAccount: PublicKey;
    assetPendingVault: PublicKey;
    assetTokenProgram: TokenProgramKind;
}): TransactionInstruction {
    return kitIxToWeb3(
        getCancelQueuedDepositRequestInstruction(
            {
                user: noop(args.user),
                assetMint: addr(args.assetMint),
                shareMint: addr(args.shareMint),
                vault: addr(args.vault),
                request: addr(args.request),
                userTokenAccount: addr(args.userTokenAccount),
                assetPendingVault: addr(args.assetPendingVault),
                assetTokenProgram: addr(tokenProgramId(args.assetTokenProgram)),
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildCancelQueuedRedemptionIx(args: {
    user: PublicKey;
    assetMint: PublicKey;
    shareMint: PublicKey;
    vault: PublicKey;
    request: PublicKey;
    userShareAccount: PublicKey;
    shareTokenProgram: TokenProgramKind;
}): TransactionInstruction {
    return kitIxToWeb3(
        getCancelQueuedRedemptionRequestInstruction(
            {
                user: noop(args.user),
                assetMint: addr(args.assetMint),
                shareMint: addr(args.shareMint),
                vault: addr(args.vault),
                request: addr(args.request),
                userShareAccount: addr(args.userShareAccount),
                shareTokenProgram: addr(tokenProgramId(args.shareTokenProgram)),
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildSkipCanceledIx(args: {
    vault: PublicKey;
    request: PublicKey;
    requestOwner: PublicKey;
}): TransactionInstruction {
    return kitIxToWeb3(
        getSkipCanceledQueueRequestInstruction(
            {
                vault: addr(args.vault),
                request: addr(args.request),
                owner: addr(args.requestOwner),
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export interface ClaimParams {
    user: PublicKey;
    owner: PublicKey;
    assetMint: PublicKey;
    shareMint: PublicKey;
    vault: PublicKey;
    request: PublicKey;
    requestType: 'deposit' | 'redeem';
    userShareAccount?: PublicKey;
    userAssetAccount?: PublicKey;
    pendingVault?: PublicKey;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export function buildClaimIx(p: ClaimParams): TransactionInstruction {
    return kitIxToWeb3(
        getClaimInstruction(
            {
                user: noop(p.user),
                owner: addr(p.owner),
                assetMint: addr(p.assetMint),
                shareMint: addr(p.shareMint),
                vault: addr(p.vault),
                request: addr(p.request),
                pendingVault: p.pendingVault ? addr(p.pendingVault) : undefined,
                userShareAccount: p.userShareAccount ? addr(p.userShareAccount) : undefined,
                userAssetAccount: p.userAssetAccount ? addr(p.userAssetAccount) : undefined,
                assetTokenProgram: addr(tokenProgramId(p.assetTokenProgram)),
                shareTokenProgram: p.requestType === 'deposit' ? addr(tokenProgramId(p.shareTokenProgram)) : undefined,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildSetOperatorIx(args: {
    user: PublicKey;
    operator: PublicKey;
    request: PublicKey;
}): TransactionInstruction {
    return kitIxToWeb3(
        getSetOperatorInstruction(
            {
                user: noop(args.user),
                operator: noop(args.operator),
                request: addr(args.request),
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

// ── Extension initialize / update ───────────────────────────────────────────

function feeArgs(bps: number): FeeTypeArgs {
    return { __kind: 'Percentage', bps };
}

export function buildInitDepositFeeIx(args: {
    payer: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
    bps: number;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInitializeDepositFeeInstruction(
            {
                payer: noop(args.payer),
                authority: noop(args.authority),
                vault: addr(args.vault),
                depositFee: feeArgs(args.bps),
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildInitWithdrawalFeeIx(args: {
    payer: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
    bps: number;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInitializeWithdrawalFeeInstruction(
            {
                payer: noop(args.payer),
                authority: noop(args.authority),
                vault: addr(args.vault),
                withdrawalFee: feeArgs(args.bps),
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildUpdateDepositFeeIx(args: {
    authority: PublicKey;
    vault: PublicKey;
    bps: number;
}): TransactionInstruction {
    return kitIxToWeb3(
        getUpdateDepositFeeInstruction(
            {
                authority: noop(args.authority),
                vault: addr(args.vault),
                newDepositFee: feeArgs(args.bps),
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildUpdateWithdrawalFeeIx(args: {
    authority: PublicKey;
    vault: PublicKey;
    bps: number;
}): TransactionInstruction {
    return kitIxToWeb3(
        getUpdateWithdrawalFeeInstruction(
            {
                authority: noop(args.authority),
                vault: addr(args.vault),
                newWithdrawalFee: feeArgs(args.bps),
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildInitPausableSubscriptionsIx(args: {
    payer: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
    paused: boolean;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInitializePausableSubscriptionsInstruction(
            {
                payer: noop(args.payer),
                authority: noop(args.authority),
                vault: addr(args.vault),
                paused: args.paused,
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildInitPausableRedemptionsIx(args: {
    payer: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
    paused: boolean;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInitializePausableRedemptionsInstruction(
            {
                payer: noop(args.payer),
                authority: noop(args.authority),
                vault: addr(args.vault),
                paused: args.paused,
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildUpdatePausableSubscriptionsIx(args: {
    authority: PublicKey;
    vault: PublicKey;
    paused: boolean;
}): TransactionInstruction {
    return kitIxToWeb3(
        getUpdatePausableSubscriptionsInstruction(
            {
                authority: noop(args.authority),
                vault: addr(args.vault),
                paused: args.paused,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildUpdatePausableRedemptionsIx(args: {
    authority: PublicKey;
    vault: PublicKey;
    paused: boolean;
}): TransactionInstruction {
    return kitIxToWeb3(
        getUpdatePausableRedemptionsInstruction(
            {
                authority: noop(args.authority),
                vault: addr(args.vault),
                paused: args.paused,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildInitMinSubscriptionIx(args: {
    payer: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
    threshold: bigint;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInitializeMinSubscriptionInstruction(
            {
                payer: noop(args.payer),
                authority: noop(args.authority),
                vault: addr(args.vault),
                threshold: args.threshold,
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildInitMinRedemptionIx(args: {
    payer: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
    threshold: bigint;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInitializeMinRedemptionInstruction(
            {
                payer: noop(args.payer),
                authority: noop(args.authority),
                vault: addr(args.vault),
                threshold: args.threshold,
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildUpdateMinSubscriptionIx(args: {
    authority: PublicKey;
    vault: PublicKey;
    threshold: bigint;
}): TransactionInstruction {
    return kitIxToWeb3(
        getUpdateMinSubscriptionInstruction(
            {
                authority: noop(args.authority),
                vault: addr(args.vault),
                threshold: args.threshold,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildUpdateMinRedemptionIx(args: {
    authority: PublicKey;
    vault: PublicKey;
    threshold: bigint;
}): TransactionInstruction {
    return kitIxToWeb3(
        getUpdateMinRedemptionInstruction(
            {
                authority: noop(args.authority),
                vault: addr(args.vault),
                threshold: args.threshold,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildInitSubscriptionQueueIx(args: {
    payer: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInitializeSubscriptionQueueInstruction(
            {
                payer: noop(args.payer),
                authority: noop(args.authority),
                vault: addr(args.vault),
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

export function buildInitRedemptionQueueIx(args: {
    payer: PublicKey;
    authority: PublicKey;
    vault: PublicKey;
}): TransactionInstruction {
    return kitIxToWeb3(
        getInitializeRedemptionQueueInstruction(
            {
                payer: noop(args.payer),
                authority: noop(args.authority),
                vault: addr(args.vault),
                systemProgram: SYSTEM_PROGRAM,
            },
            { programAddress: PROGRAM_ADDRESS },
        ),
    );
}

/** Generate a fresh keypair for a Request account (signer at create time). */
export function newRequestKeypair(): Keypair {
    return Keypair.generate();
}
