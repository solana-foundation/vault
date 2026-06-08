import * as React from 'react';
import { isAddress, type Address } from '@solana/kit';
import { Pause, Play, Send, ShieldCheck, ShieldOff, Sparkles } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { useWallet } from '@/contexts/WalletContext';
import { useSendTx } from '@/hooks/useSendTx';
import { ExtensionType } from '@/lib/extensions';
import { type VaultRequest, type VaultState } from '@/lib/hooks/use-vault';
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
import { buildCreateAtaIdempotentIx, buildMintToIx, getAtaAddress } from '@/lib/token';
import { updateKnownVault } from '@/lib/vault-storage';

import { RequestList } from './request-list';

function requireAddress(value: string, field: string): Address {
    const trimmed = value.trim();
    if (!isAddress(trimmed)) throw new Error(`${field} is not a valid public key`);
    return trimmed;
}

function requestAssertion(req: VaultRequest) {
    return {
        amount: req.raw.amount,
        createdAt: req.raw.createdAt,
        navUpdateVersion: req.raw.navUpdateVersion,
        owner: req.raw.owner,
        requestType: req.raw.requestType,
    };
}

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
    const { account, createSigner } = useWallet();
    const { send, error: sendError } = useSendTx();
    const owner = account?.address as Address | undefined;

    const [navInput, setNavInput] = React.useState('1');
    const [withdrawAmount, setWithdrawAmount] = React.useState('');
    const [withdrawTo, setWithdrawTo] = React.useState('');
    const [newAuthority, setNewAuthority] = React.useState('');
    const feeRecipient = vault.base.feeRecipient as string;
    const [demoMintAmount, setDemoMintAmount] = React.useState('1000');

    const hasPausableSubs = vault.extensions.some(e => e.type === ExtensionType.PausableSubscriptions);
    const hasPausableRedeems = vault.extensions.some(e => e.type === ExtensionType.PausableRedemptions);

    const isAuthority = owner === (vault.base.authority as string);
    const isPendingAuthority =
        vault.base.pendingAuthority.__option === 'Some' && owner === (vault.base.pendingAuthority.value as string);

    const subsExt = vault.extensions.find(e => e.type === ExtensionType.PausableSubscriptions) as
        | { type: typeof ExtensionType.PausableSubscriptions; paused: boolean }
        | undefined;
    const redeemsExt = vault.extensions.find(e => e.type === ExtensionType.PausableRedemptions) as
        | { type: typeof ExtensionType.PausableRedemptions; paused: boolean }
        | undefined;

    const handleUpdateNav = async () => {
        const signer = createSigner();
        if (!signer || !isAuthority) return;
        try {
            const nav = parseTokenAmount(navInput || '0', vault.assetMint.decimals);
            const ix = buildUpdateNavIx({ authority: signer, nav, vault: vault.pdas.vault });
            if (await send([ix], { action: 'Update NAV' })) {
                toast.success('NAV updated');
                onRefresh();
            }
        } catch (err) {
            toast.error('NAV update failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleApprove = async (req: VaultRequest) => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const feeExtType = req.type === 'deposit' ? ExtensionType.DepositFee : ExtensionType.WithdrawalFee;
            const feeExt = vault.extensions.find(e => e.type === feeExtType);
            const hasFee =
                !!feeExt &&
                'feeKind' in feeExt &&
                (feeExt.feeKind === 'percentage' ? feeExt.bps > 0 : feeExt.amount > 0n);

            const ixs = [];
            let feeRecipientTokenAccount: Address | undefined;
            if (hasFee) {
                // Fee is transferred to the fee recipient's asset token account, which must exist
                // and be supplied to the program as a remaining account.
                const created = await buildCreateAtaIdempotentIx({
                    kind: vault.assetTokenProgram,
                    mint: vault.base.assetMint,
                    owner: vault.base.feeRecipient,
                    payer: signer,
                });
                feeRecipientTokenAccount = created.ata;
                ixs.push(created.instruction);
            }

            ixs.push(
                buildApproveRequestIx({
                    assertion: requestAssertion(req),
                    assetMint: vault.base.assetMint,
                    assetTokenProgram: vault.assetTokenProgram,
                    authority: signer,
                    feeRecipientTokenAccount,
                    pendingVault: vault.pdas.pendingVault,
                    request: req.address,
                    shareMint: vault.shareMint,
                    vault: vault.pdas.vault,
                    vaultTokenAccount: vault.pdas.reserve,
                }),
            );

            if (await send(ixs, { action: 'Approve request' })) {
                toast.success('Request approved');
                onRefresh();
            }
        } catch (err) {
            toast.error('Approve failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleReject = async (req: VaultRequest) => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const userAta = await getAtaAddress(
                req.type === 'deposit' ? vault.base.assetMint : vault.shareMint,
                req.owner,
                req.type === 'deposit' ? vault.assetTokenProgram : vault.shareTokenProgram,
            );
            const ix = await buildRejectRequestIx({
                assertion: requestAssertion(req),
                assetMint: vault.base.assetMint,
                assetTokenProgram: vault.assetTokenProgram,
                authority: signer,
                request: req.address,
                requestType: req.type,
                shareMint: vault.shareMint,
                shareTokenProgram: vault.shareTokenProgram,
                user: req.owner,
                vault: vault.pdas.vault,
                ...(req.type === 'deposit'
                    ? { assetPendingVault: vault.pdas.pendingVault, userTokenAccount: userAta }
                    : { userShareAccount: userAta }),
            });
            if (await send([ix], { action: 'Reject request' })) {
                toast.success('Request rejected');
                onRefresh();
            }
        } catch (err) {
            toast.error('Reject failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleWithdraw = async () => {
        const signer = createSigner();
        if (!signer || !owner) return;
        try {
            const recipient = withdrawTo.trim() ? requireAddress(withdrawTo, 'Recipient') : owner;
            const { ata: recipientAta, instruction: ataIx } = await buildCreateAtaIdempotentIx({
                kind: vault.assetTokenProgram,
                mint: vault.base.assetMint,
                owner: recipient,
                payer: signer,
            });
            const withdrawIx = buildWithdrawAssetsIx({
                amount: parseTokenAmount(withdrawAmount || '0', vault.assetMint.decimals),
                assetMint: vault.base.assetMint,
                assetTokenProgram: vault.assetTokenProgram,
                authority: signer,
                recipientTokenAccount: recipientAta,
                vault: vault.pdas.vault,
                vaultTokenAccount: vault.pdas.reserve,
            });
            if (await send([ataIx, withdrawIx], { action: 'Withdraw assets' })) {
                toast.success('Assets withdrawn');
                setWithdrawAmount('');
                onRefresh();
            }
        } catch (err) {
            toast.error('Withdraw failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleInvite = async () => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const next = requireAddress(newAuthority, 'New authority');
            const ix = buildInviteAuthorityIx({ authority: signer, newAuthority: next, vault: vault.pdas.vault });
            if (await send([ix], { action: 'Invite authority' })) {
                toast.success('Authority invited');
                onRefresh();
            }
        } catch (err) {
            toast.error('Invite failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleAccept = async () => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const ix = buildAcceptAuthorityIx({ newAuthority: signer, vault: vault.pdas.vault });
            if (await send([ix], { action: 'Accept authority' })) {
                updateKnownVault(vault.shareMint, { isAuthority: true });
                toast.success('Authority accepted');
                onRefresh();
            }
        } catch (err) {
            toast.error('Accept failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleTogglePauseAll = async () => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const recipient = feeRecipient.trim()
                ? requireAddress(feeRecipient, 'Fee recipient')
                : (vault.base.feeRecipient as Address);
            const ix = await buildUpdateVaultIxAsync({
                authority: signer,
                feeRecipient: recipient,
                paused: !vault.base.paused,
                shareMint: vault.shareMint,
                vault: vault.pdas.vault,
            });
            if (await send([ix], { action: vault.base.paused ? 'Unpause vault' : 'Pause vault' })) {
                toast.success(vault.base.paused ? 'Vault resumed' : 'Vault paused');
                onRefresh();
            }
        } catch (err) {
            toast.error('Update failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleTogglePauseSubs = async (paused: boolean) => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const ix = buildUpdatePausableSubscriptionsIx({ authority: signer, paused, vault: vault.pdas.vault });
            if (await send([ix], { action: 'Update pausable subscriptions' })) {
                toast.success('Updated');
                onRefresh();
            }
        } catch (err) {
            toast.error('Update failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleTogglePauseRedeems = async (paused: boolean) => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const ix = buildUpdatePausableRedemptionsIx({ authority: signer, paused, vault: vault.pdas.vault });
            if (await send([ix], { action: 'Update pausable redemptions' })) {
                toast.success('Updated');
                onRefresh();
            }
        } catch (err) {
            toast.error('Update failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleAirdropDemo = async () => {
        const signer = createSigner();
        if (!signer || !owner) return;
        try {
            const { ata, instruction: ataIx } = await buildCreateAtaIdempotentIx({
                kind: vault.assetTokenProgram,
                mint: vault.base.assetMint,
                owner,
                payer: signer,
            });
            const mintIx = buildMintToIx({
                amount: parseTokenAmount(demoMintAmount || '0', vault.assetMint.decimals),
                authority: signer,
                destination: ata,
                kind: vault.assetTokenProgram,
                mint: vault.base.assetMint,
            });
            if (await send([ataIx, mintIx], { action: 'Airdrop demo asset' })) {
                toast.success('Demo asset minted');
                onRefresh();
            }
        } catch (err) {
            toast.error('Mint failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const pendingRequests = requests.filter(r => r.state === 'pending');

    return (
        <div className="space-y-6">
            {sendError ? (
                <Card className="border-destructive/40 bg-destructive/5">
                    <CardContent className="p-4 text-sm text-destructive">{sendError}</CardContent>
                </Card>
            ) : null}

            {!isAuthority ? (
                <Card className="border-warning/40 bg-warning/5">
                    <CardContent className="p-4 text-sm">
                        Connected wallet is <span className="font-mono">{owner ? owner.slice(0, 8) : '—'}…</span> — not
                        the current authority. Authority-only actions will be disabled.
                        {isPendingAuthority ? (
                            <Button onClick={handleAccept} variant="default" size="sm" className="ml-3">
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
                                variant={vault.base.paused ? 'default' : 'outline'}
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
                                    variant={subsExt?.paused ? 'default' : 'outline'}
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
                                    variant={redeemsExt?.paused ? 'default' : 'outline'}
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
                                Pending: {(vault.base.pendingAuthority.value as string).slice(0, 8)}…
                            </p>
                        ) : null}
                    </CardContent>
                </Card>

                {demoAssetMintAuthority && owner === demoAssetMintAuthority ? (
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
