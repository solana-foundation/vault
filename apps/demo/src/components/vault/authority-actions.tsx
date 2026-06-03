'use client';

import * as React from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { PublicKey } from '@solana/web3.js';
import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountIdempotentInstruction } from '@solana/spl-token';
import { Coins, Pause, Play, Send, ShieldCheck, ShieldOff, Sparkles } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { ExtensionType } from '@/lib/extensions';
import { formatTokenAmount, parseTokenAmount } from '@/lib/format';
import {
    buildAcceptAuthorityIx,
    buildApproveRequestIx,
    buildInviteAuthorityIx,
    buildRejectRequestIx,
    buildUpdateNavIx,
    buildUpdatePausableRedemptionsIx,
    buildUpdatePausableSubscriptionsIx,
    buildUpdateVaultIxAsync,
    buildWithdrawAssetsIx,
} from '@/lib/program';
import { sendIxs } from '@/lib/tx';
import { buildMintToInstruction, getAtaAddress, tokenProgramId, type TokenProgramKind } from '@/lib/token';
import type { VaultRequest, VaultState } from '@/lib/hooks/use-vault';
import { RequestList } from './request-list';
import { updateKnownVault } from '@/lib/vault-storage';

export function AuthorityActions({
    vault,
    requests,
    onRefresh,
    demoAssetMintAuthority,
}: {
    vault: VaultState;
    requests: VaultRequest[];
    onRefresh: () => void;
    demoAssetMintAuthority?: string | null;
}) {
    const wallet = useWallet();
    const { connection } = useConnection();

    const [navInput, setNavInput] = React.useState('1');
    const [withdrawAmount, setWithdrawAmount] = React.useState('');
    const [withdrawTo, setWithdrawTo] = React.useState('');
    const [newAuthority, setNewAuthority] = React.useState('');
    const [feeRecipient, setFeeRecipient] = React.useState(vault.base.feeRecipient as unknown as string);
    const [demoMintAmount, setDemoMintAmount] = React.useState('1000');

    const hasPausableSubs = vault.extensions.some(e => e.type === ExtensionType.PausableSubscriptions);
    const hasPausableRedeems = vault.extensions.some(e => e.type === ExtensionType.PausableRedemptions);

    const isAuthority = wallet.publicKey?.toBase58() === (vault.base.authority as unknown as string);
    const isPendingAuthority =
        vault.base.pendingAuthority.__option === 'Some' &&
        wallet.publicKey?.toBase58() === (vault.base.pendingAuthority.value as unknown as string);

    const subsExt = vault.extensions.find(e => e.type === ExtensionType.PausableSubscriptions) as
        | { type: typeof ExtensionType.PausableSubscriptions; paused: boolean }
        | undefined;
    const redeemsExt = vault.extensions.find(e => e.type === ExtensionType.PausableRedemptions) as
        | { type: typeof ExtensionType.PausableRedemptions; paused: boolean }
        | undefined;

    const sendTx = async (
        label: string,
        ixs: Awaited<ReturnType<typeof buildUpdateNavIx>>[] | Parameters<typeof sendIxs>[0]['instructions'],
    ) => {
        if (!wallet.publicKey) return;
        await sendIxs({
            connection,
            wallet,
            instructions: ixs as Parameters<typeof sendIxs>[0]['instructions'],
            label,
        });
        onRefresh();
    };

    const handleUpdateNav = async () => {
        if (!wallet.publicKey || !isAuthority) return;
        try {
            const nav = parseTokenAmount(navInput || '0', vault.assetMint.decimals);
            const ix = buildUpdateNavIx({ authority: wallet.publicKey, vault: vault.pdas.vault, nav });
            await sendTx('Updating NAV…', [ix]);
        } catch (err) {
            toast.error('NAV update failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleApprove = async (req: VaultRequest) => {
        if (!wallet.publicKey) return;
        try {
            const ix = buildApproveRequestIx({
                authority: wallet.publicKey,
                assetMint: new PublicKey(vault.base.assetMint as unknown as string),
                shareMint: vault.shareMint,
                vault: vault.pdas.vault,
                request: req.address,
                vaultTokenAccount: vault.pdas.reserve,
                pendingVault: vault.pdas.pendingVault,
                assetTokenProgram: vault.assetTokenProgram,
            });
            await sendTx('Approving request…', [ix]);
        } catch (err) {
            toast.error('Approve failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleReject = async (req: VaultRequest) => {
        if (!wallet.publicKey) return;
        try {
            const userAta = getAtaAddress(
                req.type === 'deposit' ? new PublicKey(vault.base.assetMint as unknown as string) : vault.shareMint,
                req.owner,
                req.type === 'deposit' ? vault.assetTokenProgram : vault.shareTokenProgram,
            );
            const ix = await buildRejectRequestIx({
                authority: wallet.publicKey,
                assetMint: new PublicKey(vault.base.assetMint as unknown as string),
                shareMint: vault.shareMint,
                request: req.address,
                user: req.owner,
                requestType: req.type,
                ...(req.type === 'deposit'
                    ? { userTokenAccount: userAta, assetPendingVault: vault.pdas.pendingVault }
                    : { userShareAccount: userAta }),
                assetTokenProgram: vault.assetTokenProgram,
                shareTokenProgram: vault.shareTokenProgram,
            });
            await sendTx('Rejecting request…', [ix]);
        } catch (err) {
            toast.error('Reject failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleWithdraw = async () => {
        if (!wallet.publicKey) return;
        try {
            const recipient = withdrawTo.trim() ? new PublicKey(withdrawTo.trim()) : wallet.publicKey;
            const recipientAta = getAtaAddress(
                new PublicKey(vault.base.assetMint as unknown as string),
                recipient,
                vault.assetTokenProgram,
            );
            const ix1 = createAssociatedTokenAccountIdempotentInstruction(
                wallet.publicKey,
                recipientAta,
                recipient,
                new PublicKey(vault.base.assetMint as unknown as string),
                tokenProgramId(vault.assetTokenProgram),
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );
            const ix2 = buildWithdrawAssetsIx({
                authority: wallet.publicKey,
                assetMint: new PublicKey(vault.base.assetMint as unknown as string),
                vault: vault.pdas.vault,
                vaultTokenAccount: vault.pdas.reserve,
                recipientTokenAccount: recipientAta,
                amount: parseTokenAmount(withdrawAmount || '0', vault.assetMint.decimals),
                assetTokenProgram: vault.assetTokenProgram,
            });
            await sendTx('Withdrawing assets…', [ix1, ix2]);
            setWithdrawAmount('');
        } catch (err) {
            toast.error('Withdraw failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleInvite = async () => {
        if (!wallet.publicKey) return;
        try {
            const next = new PublicKey(newAuthority.trim());
            const ix = buildInviteAuthorityIx({
                authority: wallet.publicKey,
                vault: vault.pdas.vault,
                newAuthority: next,
            });
            await sendTx('Inviting new authority…', [ix]);
        } catch (err) {
            toast.error('Invite failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleAccept = async () => {
        if (!wallet.publicKey) return;
        try {
            const ix = buildAcceptAuthorityIx({
                newAuthority: wallet.publicKey,
                vault: vault.pdas.vault,
            });
            await sendTx('Accepting authority…', [ix]);
            updateKnownVault(vault.shareMint.toBase58(), { isAuthority: true });
        } catch (err) {
            toast.error('Accept failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleTogglePauseAll = async () => {
        if (!wallet.publicKey) return;
        try {
            const ix = await buildUpdateVaultIxAsync({
                authority: wallet.publicKey,
                shareMint: vault.shareMint,
                paused: !vault.base.paused,
                feeRecipient: feeRecipient.trim()
                    ? new PublicKey(feeRecipient.trim())
                    : new PublicKey(vault.base.feeRecipient as unknown as string),
            });
            await sendTx(vault.base.paused ? 'Unpausing vault…' : 'Pausing vault…', [ix]);
        } catch (err) {
            toast.error('Update failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleTogglePauseSubs = async (paused: boolean) => {
        if (!wallet.publicKey) return;
        try {
            const ix = buildUpdatePausableSubscriptionsIx({
                authority: wallet.publicKey,
                vault: vault.pdas.vault,
                paused,
            });
            await sendTx('Updating pausable subscriptions…', [ix]);
        } catch (err) {
            toast.error('Update failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleTogglePauseRedeems = async (paused: boolean) => {
        if (!wallet.publicKey) return;
        try {
            const ix = buildUpdatePausableRedemptionsIx({
                authority: wallet.publicKey,
                vault: vault.pdas.vault,
                paused,
            });
            await sendTx('Updating pausable redemptions…', [ix]);
        } catch (err) {
            toast.error('Update failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const handleAirdropDemo = async () => {
        if (!wallet.publicKey) return;
        try {
            const assetMint = new PublicKey(vault.base.assetMint as unknown as string);
            const ata = getAtaAddress(assetMint, wallet.publicKey, vault.assetTokenProgram);
            const ix1 = createAssociatedTokenAccountIdempotentInstruction(
                wallet.publicKey,
                ata,
                wallet.publicKey,
                assetMint,
                tokenProgramId(vault.assetTokenProgram),
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );
            const ix2 = buildMintToInstruction({
                mint: assetMint,
                destination: ata,
                authority: wallet.publicKey,
                amount: parseTokenAmount(demoMintAmount || '0', vault.assetMint.decimals),
                kind: vault.assetTokenProgram,
            });
            await sendTx('Airdropping demo asset…', [ix1, ix2]);
        } catch (err) {
            toast.error('Mint failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        }
    };

    const pendingRequests = requests.filter(r => r.state === 'pending');

    return (
        <div className="space-y-6">
            {!isAuthority ? (
                <Card className="border-warning/40 bg-warning/5">
                    <CardContent className="p-4 text-sm">
                        Connected wallet is{' '}
                        <span className="font-mono">{wallet.publicKey?.toBase58().slice(0, 8)}…</span> — not the current
                        authority. Authority-only actions will be disabled.
                        {isPendingAuthority ? (
                            <Button onClick={handleAccept} variant="gradient" size="sm" className="ml-3">
                                Accept authority invitation
                            </Button>
                        ) : null}
                    </CardContent>
                </Card>
            ) : null}

            <RequestList
                title={`Pending requests (${pendingRequests.length})`}
                description="Approve to settle at current NAV, or reject to refund the user."
                requests={pendingRequests}
                vault={vault}
                emptyLabel="No pending requests."
                actions={req => (
                    <>
                        <Button
                            size="sm"
                            disabled={!isAuthority}
                            onClick={() => handleApprove(req)}
                            className="bg-success text-success-foreground hover:bg-success/90"
                        >
                            Approve
                        </Button>
                        <Button size="sm" variant="outline" disabled={!isAuthority} onClick={() => handleReject(req)}>
                            Reject
                        </Button>
                    </>
                )}
            />

            <div className="grid gap-4 md:grid-cols-2">
                <Card>
                    <CardHeader>
                        <CardTitle className="text-base">Update NAV</CardTitle>
                        <CardDescription>
                            Sets the per-share net asset value used to settle approved requests. Increments{' '}
                            <code className="font-mono">nav_version</code>.
                        </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-3">
                        <div className="flex items-end gap-2">
                            <div className="flex-1">
                                <Label>NAV (assets per share)</Label>
                                <Input
                                    inputMode="decimal"
                                    className="mt-1.5"
                                    value={navInput}
                                    onChange={e => setNavInput(e.target.value)}
                                />
                            </div>
                            <Button onClick={handleUpdateNav} disabled={!isAuthority}>
                                Update
                            </Button>
                        </div>
                        <p className="text-xs text-muted-foreground">
                            Current: {formatTokenAmount(vault.base.nav, vault.assetMint.decimals)}
                        </p>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-base">Pause controls</CardTitle>
                        <CardDescription>Halt new flows without changing balances.</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-2">
                        <div className="flex items-center justify-between rounded-md border border-border/60 p-3">
                            <div>
                                <p className="text-sm font-medium">Vault paused</p>
                                <p className="text-xs text-muted-foreground">
                                    Locks every entrypoint that depends on{' '}
                                    <code className="font-mono">!vault.paused</code>.
                                </p>
                            </div>
                            <Button
                                size="sm"
                                variant={vault.base.paused ? 'gradient' : 'outline'}
                                onClick={handleTogglePauseAll}
                                disabled={!isAuthority}
                            >
                                {vault.base.paused ? <Play className="size-4" /> : <Pause className="size-4" />}
                                {vault.base.paused ? 'Resume' : 'Pause'}
                            </Button>
                        </div>
                        {hasPausableSubs ? (
                            <div className="flex items-center justify-between rounded-md border border-border/60 p-3">
                                <div>
                                    <p className="text-sm font-medium">Subscriptions</p>
                                    <p className="text-xs text-muted-foreground">
                                        Block new <code className="font-mono">create_deposit_request</code>.
                                    </p>
                                </div>
                                <Button
                                    size="sm"
                                    variant={subsExt?.paused ? 'gradient' : 'outline'}
                                    onClick={() => handleTogglePauseSubs(!subsExt?.paused)}
                                    disabled={!isAuthority}
                                >
                                    {subsExt?.paused ? 'Resume' : 'Pause'}
                                </Button>
                            </div>
                        ) : null}
                        {hasPausableRedeems ? (
                            <div className="flex items-center justify-between rounded-md border border-border/60 p-3">
                                <div>
                                    <p className="text-sm font-medium">Redemptions</p>
                                    <p className="text-xs text-muted-foreground">
                                        Block new <code className="font-mono">create_redeem_request</code>.
                                    </p>
                                </div>
                                <Button
                                    size="sm"
                                    variant={redeemsExt?.paused ? 'gradient' : 'outline'}
                                    onClick={() => handleTogglePauseRedeems(!redeemsExt?.paused)}
                                    disabled={!isAuthority}
                                >
                                    {redeemsExt?.paused ? 'Resume' : 'Pause'}
                                </Button>
                            </div>
                        ) : null}
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-base">Withdraw vault assets</CardTitle>
                        <CardDescription>
                            Move assets out of the vault reserve — typically to deploy them off-chain.{' '}
                            <code className="font-mono">total_asset_balance</code> still tracks the virtual balance for
                            NAV math.
                        </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-3">
                        <div>
                            <Label>Amount</Label>
                            <Input
                                inputMode="decimal"
                                className="mt-1.5"
                                value={withdrawAmount}
                                onChange={e => setWithdrawAmount(e.target.value)}
                            />
                        </div>
                        <div>
                            <Label>Recipient (optional)</Label>
                            <Input
                                className="mt-1.5"
                                value={withdrawTo}
                                onChange={e => setWithdrawTo(e.target.value)}
                                placeholder="Defaults to your wallet"
                            />
                        </div>
                        <Button onClick={handleWithdraw} disabled={!isAuthority || !withdrawAmount} className="w-full">
                            <Send className="size-4" /> Withdraw
                        </Button>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle className="text-base">Authority transfer</CardTitle>
                        <CardDescription>
                            Two-step: invite a new authority, then they call{' '}
                            <code className="font-mono">accept_authority_invitation</code> to take over.
                        </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-3">
                        <Label>New authority pubkey</Label>
                        <Input
                            className="mt-1.5"
                            value={newAuthority}
                            onChange={e => setNewAuthority(e.target.value)}
                            placeholder="So1AnAa…"
                        />
                        <div className="flex flex-wrap gap-2">
                            <Button onClick={handleInvite} disabled={!isAuthority || !newAuthority}>
                                <ShieldCheck className="size-4" /> Invite
                            </Button>
                            {vault.base.pendingAuthority.__option === 'Some' ? (
                                <Button variant="outline" onClick={handleAccept} disabled={!isPendingAuthority}>
                                    <ShieldOff className="size-4" /> Accept invitation
                                </Button>
                            ) : null}
                        </div>
                        {vault.base.pendingAuthority.__option === 'Some' ? (
                            <p className="text-xs text-muted-foreground">
                                Pending: {(vault.base.pendingAuthority.value as unknown as string).slice(0, 8)}…
                            </p>
                        ) : null}
                    </CardContent>
                </Card>

                {demoAssetMintAuthority && wallet.publicKey?.toBase58() === demoAssetMintAuthority ? (
                    <Card>
                        <CardHeader>
                            <CardTitle className="text-base">Mint demo asset</CardTitle>
                            <CardDescription>
                                Convenience action — you&apos;re the mint authority for this synthetic asset.
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-3">
                            <div>
                                <Label>Amount to your wallet</Label>
                                <Input
                                    inputMode="decimal"
                                    className="mt-1.5"
                                    value={demoMintAmount}
                                    onChange={e => setDemoMintAmount(e.target.value)}
                                />
                            </div>
                            <Button onClick={handleAirdropDemo} className="w-full" variant="outline">
                                <Sparkles className="size-4" /> Mint
                            </Button>
                        </CardContent>
                    </Card>
                ) : null}
            </div>

            <Separator />
            <RequestList
                title={`All requests (${requests.length})`}
                description="Historical & claimable requests for this vault."
                requests={requests}
                vault={vault}
                emptyLabel="No requests yet."
            />
        </div>
    );
}
