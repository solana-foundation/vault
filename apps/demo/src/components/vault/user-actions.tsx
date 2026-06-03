'use client';

import * as React from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { Keypair, PublicKey } from '@solana/web3.js';
import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountIdempotentInstruction } from '@solana/spl-token';
import { ArrowDownToLine, ArrowUpFromLine, Hand, ShieldQuestion, X } from 'lucide-react';
import { toast } from 'sonner';

import { AddressPill } from '@/components/ui/address';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ExtensionType } from '@/lib/extensions';
import { formatTokenAmount, parseTokenAmount } from '@/lib/format';
import {
    buildCancelQueuedDepositIx,
    buildCancelQueuedRedemptionIx,
    buildCancelRequestIx,
    buildClaimIx,
    buildCreateDepositRequestIx,
    buildCreateRedeemRequestIx,
    buildSetOperatorIx,
    newRequestKeypair,
} from '@/lib/program';
import { sendIxs } from '@/lib/tx';
import { getAtaAddress, tokenProgramId } from '@/lib/token';
import { useTokenBalance, type VaultRequest, type VaultState } from '@/lib/hooks/use-vault';
import { RequestList } from './request-list';

export function UserActions({
    vault,
    requests,
    onRefresh,
}: {
    vault: VaultState;
    requests: VaultRequest[];
    onRefresh: () => void;
}) {
    const wallet = useWallet();
    const { connection } = useConnection();
    const owner = wallet.publicKey;

    const [depositAmount, setDepositAmount] = React.useState('');
    const [redeemAmount, setRedeemAmount] = React.useState('');

    const assetMintPk = React.useMemo(
        () => new PublicKey(vault.base.assetMint as unknown as string),
        [vault.base.assetMint],
    );

    const userAssetAta = owner ? getAtaAddress(assetMintPk, owner, vault.assetTokenProgram) : null;
    const userShareAta = owner ? getAtaAddress(vault.shareMint, owner, vault.shareTokenProgram) : null;

    const { balance: assetBalance, refresh: refreshAsset } = useTokenBalance(userAssetAta?.toBase58());
    const { balance: shareBalance, refresh: refreshShare } = useTokenBalance(userShareAta?.toBase58());

    const hasSubQueue = vault.extensions.some(e => e.type === ExtensionType.SubscriptionQueue);
    const hasRedeemQueue = vault.extensions.some(e => e.type === ExtensionType.RedemptionQueue);

    const myRequests = React.useMemo(
        () => requests.filter(r => owner && r.owner.toBase58() === owner.toBase58() && r.state !== 'canceled'),
        [requests, owner],
    );

    const refreshAll = () => {
        onRefresh();
        refreshAsset();
        refreshShare();
    };

    const handleDeposit = async () => {
        if (!owner) return;
        try {
            const amount = parseTokenAmount(depositAmount || '0', vault.assetMint.decimals);
            if (amount === 0n) throw new Error('Enter an amount');
            const requestKp = newRequestKeypair();
            const userAta = getAtaAddress(assetMintPk, owner, vault.assetTokenProgram);
            const ixs = [
                createAssociatedTokenAccountIdempotentInstruction(
                    owner,
                    userAta,
                    owner,
                    assetMintPk,
                    tokenProgramId(vault.assetTokenProgram),
                    ASSOCIATED_TOKEN_PROGRAM_ID,
                ),
                buildCreateDepositRequestIx({
                    user: owner,
                    vault: vault.pdas.vault,
                    request: requestKp.publicKey,
                    assetMint: assetMintPk,
                    shareMint: vault.shareMint,
                    pendingVault: vault.pdas.pendingVault,
                    userTokenAccount: userAta,
                    amount,
                    operator: null,
                    assetTokenProgram: vault.assetTokenProgram,
                    shareTokenProgram: vault.shareTokenProgram,
                }),
            ];
            await sendIxs({
                connection,
                wallet,
                instructions: ixs,
                signers: [requestKp],
                label: 'Submitting deposit request…',
            });
            setDepositAmount('');
            refreshAll();
        } catch (err) {
            toast.error('Deposit failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleRedeem = async () => {
        if (!owner) return;
        try {
            const amount = parseTokenAmount(redeemAmount || '0', vault.shareMintInfo.decimals);
            if (amount === 0n) throw new Error('Enter an amount');
            const requestKp = newRequestKeypair();
            const userShare = getAtaAddress(vault.shareMint, owner, vault.shareTokenProgram);
            const ixs = [
                createAssociatedTokenAccountIdempotentInstruction(
                    owner,
                    userShare,
                    owner,
                    vault.shareMint,
                    tokenProgramId(vault.shareTokenProgram),
                    ASSOCIATED_TOKEN_PROGRAM_ID,
                ),
                buildCreateRedeemRequestIx({
                    user: owner,
                    vault: vault.pdas.vault,
                    request: requestKp.publicKey,
                    assetMint: assetMintPk,
                    shareMint: vault.shareMint,
                    pendingVault: vault.pdas.pendingVault,
                    userShareAccount: userShare,
                    amount,
                    operator: null,
                    assetTokenProgram: vault.assetTokenProgram,
                    shareTokenProgram: vault.shareTokenProgram,
                }),
            ];
            await sendIxs({
                connection,
                wallet,
                instructions: ixs,
                signers: [requestKp],
                label: 'Submitting redeem request…',
            });
            setRedeemAmount('');
            refreshAll();
        } catch (err) {
            toast.error('Redeem failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleClaim = async (req: VaultRequest) => {
        if (!owner) return;
        try {
            const userShare = getAtaAddress(vault.shareMint, req.owner, vault.shareTokenProgram);
            const userAsset = getAtaAddress(assetMintPk, req.owner, vault.assetTokenProgram);
            const ataIxs = [];
            if (req.type === 'deposit') {
                ataIxs.push(
                    createAssociatedTokenAccountIdempotentInstruction(
                        owner,
                        userShare,
                        req.owner,
                        vault.shareMint,
                        tokenProgramId(vault.shareTokenProgram),
                        ASSOCIATED_TOKEN_PROGRAM_ID,
                    ),
                );
            } else {
                ataIxs.push(
                    createAssociatedTokenAccountIdempotentInstruction(
                        owner,
                        userAsset,
                        req.owner,
                        assetMintPk,
                        tokenProgramId(vault.assetTokenProgram),
                        ASSOCIATED_TOKEN_PROGRAM_ID,
                    ),
                );
            }
            const claimIx = buildClaimIx({
                user: owner,
                owner: req.owner,
                assetMint: assetMintPk,
                shareMint: vault.shareMint,
                vault: vault.pdas.vault,
                request: req.address,
                requestType: req.type,
                userShareAccount: req.type === 'deposit' ? userShare : undefined,
                userAssetAccount: req.type === 'redeem' ? userAsset : undefined,
                pendingVault: req.type === 'redeem' ? vault.pdas.pendingVault : undefined,
                assetTokenProgram: vault.assetTokenProgram,
                shareTokenProgram: vault.shareTokenProgram,
            });
            await sendIxs({
                connection,
                wallet,
                instructions: [...ataIxs, claimIx],
                label: 'Claiming…',
            });
            refreshAll();
        } catch (err) {
            toast.error('Claim failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleCancel = async (req: VaultRequest) => {
        if (!owner) return;
        try {
            const userAta = getAtaAddress(assetMintPk, req.owner, vault.assetTokenProgram);
            const userShare = getAtaAddress(vault.shareMint, req.owner, vault.shareTokenProgram);
            const useQueue = (req.type === 'deposit' && hasSubQueue) || (req.type === 'redeem' && hasRedeemQueue);
            const ix = useQueue
                ? req.type === 'deposit'
                    ? buildCancelQueuedDepositIx({
                          user: owner,
                          assetMint: assetMintPk,
                          shareMint: vault.shareMint,
                          vault: vault.pdas.vault,
                          request: req.address,
                          userTokenAccount: userAta,
                          assetPendingVault: vault.pdas.pendingVault,
                          assetTokenProgram: vault.assetTokenProgram,
                      })
                    : buildCancelQueuedRedemptionIx({
                          user: owner,
                          assetMint: assetMintPk,
                          shareMint: vault.shareMint,
                          vault: vault.pdas.vault,
                          request: req.address,
                          userShareAccount: userShare,
                          shareTokenProgram: vault.shareTokenProgram,
                      })
                : buildCancelRequestIx({
                      user: owner,
                      assetMint: assetMintPk,
                      shareMint: vault.shareMint,
                      vault: vault.pdas.vault,
                      request: req.address,
                      requestType: req.type,
                      userTokenAccount: req.type === 'deposit' ? userAta : undefined,
                      assetPendingVault: req.type === 'deposit' ? vault.pdas.pendingVault : undefined,
                      userShareAccount: req.type === 'redeem' ? userShare : undefined,
                      assetTokenProgram: vault.assetTokenProgram,
                      shareTokenProgram: vault.shareTokenProgram,
                  });
            await sendIxs({ connection, wallet, instructions: [ix], label: 'Canceling request…' });
            refreshAll();
        } catch (err) {
            toast.error('Cancel failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleSetOperator = async (req: VaultRequest) => {
        if (!owner) return;
        try {
            const operatorKp = Keypair.generate();
            const ix = buildSetOperatorIx({
                user: owner,
                operator: operatorKp.publicKey,
                request: req.address,
            });
            await sendIxs({
                connection,
                wallet,
                instructions: [ix],
                signers: [operatorKp],
                label: 'Setting operator…',
            });
            toast.info('Ephemeral operator', {
                description: `Operator pubkey: ${operatorKp.publicKey.toBase58()}`,
            });
            refreshAll();
        } catch (err) {
            toast.error('Set operator failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    return (
        <div className="space-y-6">
            <div className="grid gap-4 md:grid-cols-2">
                <Card>
                    <CardHeader>
                        <CardTitle className="flex items-center gap-2 text-base">
                            <ArrowDownToLine className="size-4 text-success" />
                            Deposit (subscribe)
                        </CardTitle>
                        <CardDescription>
                            Escrows assets into the pending vault and creates a request. Authority must approve before
                            you can claim shares.
                        </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-3">
                        <div>
                            <Label>Amount</Label>
                            <Input
                                inputMode="decimal"
                                className="mt-1.5"
                                value={depositAmount}
                                onChange={e => setDepositAmount(e.target.value)}
                                placeholder="100"
                            />
                            <p className="mt-1 text-xs text-muted-foreground">
                                Wallet balance:{' '}
                                {assetBalance != null ? formatTokenAmount(assetBalance, vault.assetMint.decimals) : '—'}{' '}
                                · {vault.assetMint.decimals} dec
                            </p>
                        </div>
                        <Button
                            onClick={handleDeposit}
                            disabled={!owner || !depositAmount}
                            className="w-full"
                            variant="gradient"
                        >
                            Submit deposit request
                        </Button>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="flex items-center gap-2 text-base">
                            <ArrowUpFromLine className="size-4 text-warning" />
                            Redeem
                        </CardTitle>
                        <CardDescription>
                            Burns shares immediately and creates a request. Authority approval converts the shares into
                            assets at NAV.
                        </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-3">
                        <div>
                            <Label>Shares to redeem</Label>
                            <Input
                                inputMode="decimal"
                                className="mt-1.5"
                                value={redeemAmount}
                                onChange={e => setRedeemAmount(e.target.value)}
                                placeholder="10"
                            />
                            <p className="mt-1 text-xs text-muted-foreground">
                                Share balance:{' '}
                                {shareBalance != null
                                    ? formatTokenAmount(shareBalance, vault.shareMintInfo.decimals)
                                    : '—'}
                            </p>
                        </div>
                        <Button onClick={handleRedeem} disabled={!owner || !redeemAmount} className="w-full">
                            Submit redeem request
                        </Button>
                    </CardContent>
                </Card>
            </div>

            <RequestList
                title={`Your active requests (${myRequests.length})`}
                description="Pending requests can be canceled. Claimable requests can be claimed."
                requests={myRequests}
                vault={vault}
                emptyLabel="You have no active requests."
                actions={req => (
                    <>
                        {req.state === 'claimable' ? (
                            <Button
                                size="sm"
                                onClick={() => handleClaim(req)}
                                className="bg-success text-success-foreground hover:bg-success/90"
                            >
                                <Hand className="size-3.5" /> Claim
                            </Button>
                        ) : (
                            <>
                                <Button size="sm" variant="outline" onClick={() => handleSetOperator(req)}>
                                    <ShieldQuestion className="size-3.5" /> Set operator
                                </Button>
                                <Button
                                    size="sm"
                                    variant="outline"
                                    onClick={() => handleCancel(req)}
                                    className="border-destructive/40 text-destructive hover:bg-destructive/10 hover:text-destructive"
                                >
                                    <X className="size-3.5" /> Cancel
                                </Button>
                            </>
                        )}
                    </>
                )}
            />

            <Card className="bg-card/40">
                <CardContent className="grid gap-3 p-4 md:grid-cols-3">
                    <Stat
                        label="Asset wallet"
                        value={assetBalance != null ? formatTokenAmount(assetBalance, vault.assetMint.decimals) : '—'}
                        addr={userAssetAta?.toBase58()}
                    />
                    <Stat
                        label="Share wallet"
                        value={
                            shareBalance != null ? formatTokenAmount(shareBalance, vault.shareMintInfo.decimals) : '—'
                        }
                        addr={userShareAta?.toBase58()}
                    />
                    <Stat label="Connected" value={owner?.toBase58().slice(0, 8) + '…' || '—'} />
                </CardContent>
            </Card>
        </div>
    );
}

function Stat({ label, value, addr }: { label: string; value: string; addr?: string }) {
    return (
        <div className="space-y-1">
            <p className="text-xs uppercase tracking-wide text-muted-foreground">{label}</p>
            <p className="font-mono text-sm font-medium tabular-nums">{value}</p>
            {addr ? <AddressPill value={addr} chars={4} /> : null}
        </div>
    );
}
