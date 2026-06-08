import { AccountRole, type Address, type Instruction, type TransactionSigner } from '@solana/kit';

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
    type RequestTypeArgs,
} from '@solana/vault';

import { PROGRAM_ADDRESS } from './config';
import { tokenProgramAddress, type TokenProgramKind } from './token';

const SYSTEM_PROGRAM = '11111111111111111111111111111111' as Address;
const PROGRAM_CONFIG = { programAddress: PROGRAM_ADDRESS } as const;

export interface VaultPdas {
    vault: Address;
    reserve: Address;
    pendingVault: Address;
}

export async function deriveVaultPdas(shareMint: Address): Promise<VaultPdas> {
    const seeds = { shareMint };
    const [v, r, p] = await Promise.all([
        findVaultPda(seeds, PROGRAM_CONFIG),
        findReservePda(seeds, PROGRAM_CONFIG),
        findPendingVaultPda(seeds, PROGRAM_CONFIG),
    ]);
    return { pendingVault: p[0], reserve: r[0], vault: v[0] };
}

export interface CreateVaultParams {
    payer: TransactionSigner;
    mintAuthority: TransactionSigner;
    assetMint: Address;
    shareMint: Address;
    vault: Address;
    reserve: Address;
    pendingVault: Address;
    authority: Address;
    feeRecipient: Address;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export function buildCreateVaultIx(p: CreateVaultParams): Promise<Instruction> {
    return getCreateVaultInstructionAsync(
        {
            assetMint: p.assetMint,
            assetTokenProgram: tokenProgramAddress(p.assetTokenProgram),
            authority: p.authority,
            feeRecipient: p.feeRecipient,
            mintAuthority: p.mintAuthority,
            payer: p.payer,
            pendingVault: p.pendingVault,
            reserve: p.reserve,
            shareMint: p.shareMint,
            shareTokenProgram: tokenProgramAddress(p.shareTokenProgram),
            systemProgram: SYSTEM_PROGRAM,
            vault: p.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildInitializeVaultIx(authority: TransactionSigner, shareMint: Address, vault: Address): Instruction {
    return getInitializeVaultInstruction({ authority, shareMint, vault }, PROGRAM_CONFIG);
}

/** Request fields the authority must assert when approving/rejecting (audit hardening). */
export interface RequestAssertion {
    owner: Address;
    requestType: RequestTypeArgs;
    amount: number | bigint;
    createdAt: number | bigint;
    navUpdateVersion: number | bigint;
}

export function buildUpdateVaultIxAsync(args: {
    authority: TransactionSigner;
    shareMint: Address;
    vault: Address;
    paused: boolean;
    feeRecipient: Address;
}): Promise<Instruction> {
    return getUpdateVaultInstructionAsync(
        {
            authority: args.authority,
            feeRecipient: args.feeRecipient,
            paused: args.paused,
            shareMint: args.shareMint,
            vault: args.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildUpdateNavIx(args: { authority: TransactionSigner; vault: Address; nav: bigint }): Instruction {
    return getUpdateVaultNavInstruction(
        { authority: args.authority, updatedNav: args.nav, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildInviteAuthorityIx(args: {
    authority: TransactionSigner;
    vault: Address;
    newAuthority: Address;
}): Instruction {
    return getInviteNewAuthorityInstruction(
        { authority: args.authority, newAuthority: args.newAuthority, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildAcceptAuthorityIx(args: { newAuthority: TransactionSigner; vault: Address }): Instruction {
    return getAcceptAuthorityInvitationInstruction(
        { newAuthority: args.newAuthority, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildWithdrawAssetsIx(args: {
    authority: TransactionSigner;
    assetMint: Address;
    vault: Address;
    vaultTokenAccount: Address;
    recipientTokenAccount: Address;
    amount: bigint;
    assetTokenProgram: TokenProgramKind;
}): Instruction {
    return getWithdrawAssetsInstruction(
        {
            amount: args.amount,
            assetMint: args.assetMint,
            assetTokenProgram: tokenProgramAddress(args.assetTokenProgram),
            authority: args.authority,
            recipientTokenAccount: args.recipientTokenAccount,
            vault: args.vault,
            vaultTokenAccount: args.vaultTokenAccount,
        },
        PROGRAM_CONFIG,
    );
}

export interface CreateRequestParams {
    user: TransactionSigner;
    vault: Address;
    request: TransactionSigner;
    assetMint: Address;
    shareMint: Address;
    pendingVault: Address;
    userTokenAccount?: Address;
    userShareAccount?: Address;
    amount: bigint;
    operator?: Address | null;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export function buildCreateDepositRequestIx(p: CreateRequestParams): Instruction {
    if (!p.userTokenAccount) throw new Error('userTokenAccount required for deposit');
    return getCreateDepositRequestInstruction(
        {
            args: { amount: p.amount, operator: p.operator ?? null },
            assetMint: p.assetMint,
            assetTokenProgram: tokenProgramAddress(p.assetTokenProgram),
            pendingVault: p.pendingVault,
            request: p.request,
            shareMint: p.shareMint,
            systemProgram: SYSTEM_PROGRAM,
            user: p.user,
            userTokenAccount: p.userTokenAccount,
            vault: p.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildCreateRedeemRequestIx(p: CreateRequestParams): Instruction {
    if (!p.userShareAccount) throw new Error('userShareAccount required for redeem');
    return getCreateRedeemRequestInstruction(
        {
            args: { amount: p.amount, operator: p.operator ?? null },
            assetMint: p.assetMint,
            request: p.request,
            shareMint: p.shareMint,
            shareTokenProgram: tokenProgramAddress(p.shareTokenProgram),
            systemProgram: SYSTEM_PROGRAM,
            user: p.user,
            userShareAccount: p.userShareAccount,
            vault: p.vault,
        },
        PROGRAM_CONFIG,
    );
}

export interface ApproveRequestParams {
    authority: TransactionSigner;
    assetMint: Address;
    shareMint: Address;
    vault: Address;
    request: Address;
    vaultTokenAccount: Address;
    pendingVault: Address;
    assetTokenProgram: TokenProgramKind;
    /** Fee recipient's asset token account — required as a remaining account when the vault charges a fee. */
    feeRecipientTokenAccount?: Address;
    assertion: RequestAssertion;
}

export function buildApproveRequestIx(p: ApproveRequestParams): Instruction {
    const ix = getApproveRequestInstruction(
        {
            amount: p.assertion.amount,
            assetMint: p.assetMint,
            assetTokenProgram: tokenProgramAddress(p.assetTokenProgram),
            authority: p.authority,
            createdAt: p.assertion.createdAt,
            navUpdateVersion: p.assertion.navUpdateVersion,
            owner: p.assertion.owner,
            pendingVault: p.pendingVault,
            request: p.request,
            requestType: p.assertion.requestType,
            shareMint: p.shareMint,
            vault: p.vault,
            vaultTokenAccount: p.vaultTokenAccount,
        },
        PROGRAM_CONFIG,
    );
    if (!p.feeRecipientTokenAccount) return ix;
    return {
        ...ix,
        accounts: [...ix.accounts, { address: p.feeRecipientTokenAccount, role: AccountRole.WRITABLE }],
    };
}

export interface RejectRequestParams {
    authority: TransactionSigner;
    assetMint: Address;
    shareMint: Address;
    vault: Address;
    request: Address;
    user: Address;
    requestType: 'deposit' | 'redeem';
    userTokenAccount?: Address;
    assetPendingVault?: Address;
    userShareAccount?: Address;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
    assertion: RequestAssertion;
}

export function buildRejectRequestIx(p: RejectRequestParams): Promise<Instruction> {
    return getRejectRequestInstructionAsync(
        {
            amount: p.assertion.amount,
            assetMint: p.assetMint,
            authority: p.authority,
            createdAt: p.assertion.createdAt,
            navUpdateVersion: p.assertion.navUpdateVersion,
            owner: p.assertion.owner,
            request: p.request,
            requestType: p.assertion.requestType,
            shareMint: p.shareMint,
            systemProgram: SYSTEM_PROGRAM,
            user: p.user,
            vault: p.vault,
            ...(p.requestType === 'deposit'
                ? {
                      assetPendingVault: p.assetPendingVault,
                      assetTokenProgram: tokenProgramAddress(p.assetTokenProgram),
                      userTokenAccount: p.userTokenAccount,
                  }
                : {
                      shareTokenProgram: tokenProgramAddress(p.shareTokenProgram),
                      userShareAccount: p.userShareAccount,
                  }),
        },
        PROGRAM_CONFIG,
    );
}

export interface CancelRequestParams {
    user: TransactionSigner;
    assetMint: Address;
    shareMint: Address;
    vault: Address;
    request: Address;
    requestType: 'deposit' | 'redeem';
    userTokenAccount?: Address;
    assetPendingVault?: Address;
    userShareAccount?: Address;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export function buildCancelRequestIx(p: CancelRequestParams): Instruction {
    return getCancelRequestInstruction(
        {
            assetMint: p.assetMint,
            assetPendingVault: p.assetPendingVault,
            assetTokenProgram: tokenProgramAddress(p.assetTokenProgram),
            request: p.request,
            shareMint: p.shareMint,
            shareTokenProgram: tokenProgramAddress(p.shareTokenProgram),
            systemProgram: SYSTEM_PROGRAM,
            user: p.user,
            userShareAccount: p.userShareAccount,
            userTokenAccount: p.userTokenAccount,
            vault: p.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildCancelQueuedDepositIx(args: {
    user: TransactionSigner;
    assetMint: Address;
    shareMint: Address;
    vault: Address;
    request: Address;
    userTokenAccount: Address;
    assetPendingVault: Address;
    assetTokenProgram: TokenProgramKind;
}): Instruction {
    return getCancelQueuedDepositRequestInstruction(
        {
            assetMint: args.assetMint,
            assetPendingVault: args.assetPendingVault,
            assetTokenProgram: tokenProgramAddress(args.assetTokenProgram),
            request: args.request,
            shareMint: args.shareMint,
            systemProgram: SYSTEM_PROGRAM,
            user: args.user,
            userTokenAccount: args.userTokenAccount,
            vault: args.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildCancelQueuedRedemptionIx(args: {
    user: TransactionSigner;
    assetMint: Address;
    shareMint: Address;
    vault: Address;
    request: Address;
    userShareAccount: Address;
    shareTokenProgram: TokenProgramKind;
}): Instruction {
    return getCancelQueuedRedemptionRequestInstruction(
        {
            assetMint: args.assetMint,
            request: args.request,
            shareMint: args.shareMint,
            shareTokenProgram: tokenProgramAddress(args.shareTokenProgram),
            systemProgram: SYSTEM_PROGRAM,
            user: args.user,
            userShareAccount: args.userShareAccount,
            vault: args.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildSkipCanceledIx(args: { vault: Address; request: Address; requestOwner: Address }): Instruction {
    return getSkipCanceledQueueRequestInstruction(
        { owner: args.requestOwner, request: args.request, systemProgram: SYSTEM_PROGRAM, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export interface ClaimParams {
    user: TransactionSigner;
    owner: Address;
    assetMint: Address;
    shareMint: Address;
    vault: Address;
    request: Address;
    requestType: 'deposit' | 'redeem';
    userShareAccount?: Address;
    userAssetAccount?: Address;
    pendingVault?: Address;
    assetTokenProgram: TokenProgramKind;
    shareTokenProgram: TokenProgramKind;
}

export function buildClaimIx(p: ClaimParams): Instruction {
    return getClaimInstruction(
        {
            assetMint: p.assetMint,
            assetTokenProgram: tokenProgramAddress(p.assetTokenProgram),
            owner: p.owner,
            pendingVault: p.pendingVault,
            request: p.request,
            shareMint: p.shareMint,
            shareTokenProgram: p.requestType === 'deposit' ? tokenProgramAddress(p.shareTokenProgram) : undefined,
            user: p.user,
            userAssetAccount: p.userAssetAccount,
            userShareAccount: p.userShareAccount,
            vault: p.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildSetOperatorIx(args: {
    user: TransactionSigner;
    operator: TransactionSigner;
    request: Address;
}): Instruction {
    return getSetOperatorInstruction(
        { operator: args.operator, request: args.request, user: args.user },
        PROGRAM_CONFIG,
    );
}

function feeArgs(bps: number): FeeTypeArgs {
    return { __kind: 'Percentage', bps };
}

export function buildInitDepositFeeIx(args: {
    payer: TransactionSigner;
    authority: TransactionSigner;
    vault: Address;
    bps: number;
}): Instruction {
    return getInitializeDepositFeeInstruction(
        {
            authority: args.authority,
            depositFee: feeArgs(args.bps),
            payer: args.payer,
            systemProgram: SYSTEM_PROGRAM,
            vault: args.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildInitWithdrawalFeeIx(args: {
    payer: TransactionSigner;
    authority: TransactionSigner;
    vault: Address;
    bps: number;
}): Instruction {
    return getInitializeWithdrawalFeeInstruction(
        {
            authority: args.authority,
            payer: args.payer,
            systemProgram: SYSTEM_PROGRAM,
            vault: args.vault,
            withdrawalFee: feeArgs(args.bps),
        },
        PROGRAM_CONFIG,
    );
}

export function buildUpdateDepositFeeIx(args: {
    authority: TransactionSigner;
    vault: Address;
    bps: number;
}): Instruction {
    return getUpdateDepositFeeInstruction(
        { authority: args.authority, newDepositFee: feeArgs(args.bps), vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildUpdateWithdrawalFeeIx(args: {
    authority: TransactionSigner;
    vault: Address;
    bps: number;
}): Instruction {
    return getUpdateWithdrawalFeeInstruction(
        { authority: args.authority, newWithdrawalFee: feeArgs(args.bps), vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildInitPausableSubscriptionsIx(args: {
    payer: TransactionSigner;
    authority: TransactionSigner;
    vault: Address;
    paused: boolean;
}): Instruction {
    return getInitializePausableSubscriptionsInstruction(
        {
            authority: args.authority,
            paused: args.paused,
            payer: args.payer,
            systemProgram: SYSTEM_PROGRAM,
            vault: args.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildInitPausableRedemptionsIx(args: {
    payer: TransactionSigner;
    authority: TransactionSigner;
    vault: Address;
    paused: boolean;
}): Instruction {
    return getInitializePausableRedemptionsInstruction(
        {
            authority: args.authority,
            paused: args.paused,
            payer: args.payer,
            systemProgram: SYSTEM_PROGRAM,
            vault: args.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildUpdatePausableSubscriptionsIx(args: {
    authority: TransactionSigner;
    vault: Address;
    paused: boolean;
}): Instruction {
    return getUpdatePausableSubscriptionsInstruction(
        { authority: args.authority, paused: args.paused, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildUpdatePausableRedemptionsIx(args: {
    authority: TransactionSigner;
    vault: Address;
    paused: boolean;
}): Instruction {
    return getUpdatePausableRedemptionsInstruction(
        { authority: args.authority, paused: args.paused, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildInitMinSubscriptionIx(args: {
    payer: TransactionSigner;
    authority: TransactionSigner;
    vault: Address;
    threshold: bigint;
}): Instruction {
    return getInitializeMinSubscriptionInstruction(
        {
            authority: args.authority,
            payer: args.payer,
            systemProgram: SYSTEM_PROGRAM,
            threshold: args.threshold,
            vault: args.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildInitMinRedemptionIx(args: {
    payer: TransactionSigner;
    authority: TransactionSigner;
    vault: Address;
    threshold: bigint;
}): Instruction {
    return getInitializeMinRedemptionInstruction(
        {
            authority: args.authority,
            payer: args.payer,
            systemProgram: SYSTEM_PROGRAM,
            threshold: args.threshold,
            vault: args.vault,
        },
        PROGRAM_CONFIG,
    );
}

export function buildUpdateMinSubscriptionIx(args: {
    authority: TransactionSigner;
    vault: Address;
    threshold: bigint;
}): Instruction {
    return getUpdateMinSubscriptionInstruction(
        { authority: args.authority, threshold: args.threshold, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildUpdateMinRedemptionIx(args: {
    authority: TransactionSigner;
    vault: Address;
    threshold: bigint;
}): Instruction {
    return getUpdateMinRedemptionInstruction(
        { authority: args.authority, threshold: args.threshold, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildInitSubscriptionQueueIx(args: {
    payer: TransactionSigner;
    authority: TransactionSigner;
    vault: Address;
}): Instruction {
    return getInitializeSubscriptionQueueInstruction(
        { authority: args.authority, payer: args.payer, systemProgram: SYSTEM_PROGRAM, vault: args.vault },
        PROGRAM_CONFIG,
    );
}

export function buildInitRedemptionQueueIx(args: {
    payer: TransactionSigner;
    authority: TransactionSigner;
    vault: Address;
}): Instruction {
    return getInitializeRedemptionQueueInstruction(
        { authority: args.authority, payer: args.payer, systemProgram: SYSTEM_PROGRAM, vault: args.vault },
        PROGRAM_CONFIG,
    );
}
