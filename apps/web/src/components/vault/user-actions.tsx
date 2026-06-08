import * as React from 'react';
import { generateKeyPairSigner, type Address } from '@solana/kit';
import { ArrowDownToLine, ArrowUpFromLine, Hand, ShieldQuestion, X } from 'lucide-react';
import { toast } from 'sonner';

import { AddressPill } from '@/components/ui/address';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useWallet } from '@/contexts/WalletContext';
import { useSendTx } from '@/hooks/useSendTx';
import { ExtensionType } from '@/lib/extensions';
import { useTokenBalance, type VaultRequest, type VaultState } from '@/lib/hooks/use-vault';
import { formatTokenAmount, parseTokenAmount } from '@/lib/format';
import {
    buildCancelQueuedDepositIx,
    buildCancelQueuedRedemptionIx,
    buildCancelRequestIx,
    buildClaimIx,
    buildCreateDepositRequestIx,
    buildCreateRedeemRequestIx,
    buildSetOperatorIx,
} from '@/lib/program';
import { buildCreateAtaIdempotentIx, getAtaAddress } from '@/lib/token';

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
    const { account, createSigner } = useWallet();
    const { send, error: sendError } = useSendTx();
    const owner = account?.address as Address | undefined;

    const [depositAmount, setDepositAmount] = React.useState('');
    const [redeemAmount, setRedeemAmount] = React.useState('');
    const [userAssetAta, setUserAssetAta] = React.useState<string | null>(null);
    const [userShareAta, setUserShareAta] = React.useState<string | null>(null);

    React.useEffect(() => {
        if (!owner) {
            setUserAssetAta(null);
            setUserShareAta(null);
            return;
        }
        let cancelled = false;
        void (async () => {
            const [asset, share] = await Promise.all([
                getAtaAddress(vault.base.assetMint, owner, vault.assetTokenProgram),
                getAtaAddress(vault.shareMint, owner, vault.shareTokenProgram),
            ]);
            if (!cancelled) {
                setUserAssetAta(asset);
                setUserShareAta(share);
            }
        })();
        return () => {
            cancelled = true;
        };
    }, [owner, vault.base.assetMint, vault.shareMint, vault.assetTokenProgram, vault.shareTokenProgram]);

    const { balance: assetBalance, refresh: refreshAsset } = useTokenBalance(userAssetAta);
    const { balance: shareBalance, refresh: refreshShare } = useTokenBalance(userShareAta);

    const hasSubQueue = vault.extensions.some(e => e.type === ExtensionType.SubscriptionQueue);
    const hasRedeemQueue = vault.extensions.some(e => e.type === ExtensionType.RedemptionQueue);

    const myRequests = React.useMemo(
        () => requests.filter(r => owner && r.owner === owner && r.state !== 'canceled'),
        [requests, owner],
    );

    const refreshAll = () => {
        onRefresh();
        refreshAsset();
        refreshShare();
    };

    const handleDeposit = async () => {
        const signer = createSigner();
        if (!signer || !owner) {
            toast.error('Connect a wallet first');
            return;
        }
        try {
            const amount = parseTokenAmount(depositAmount || '0', vault.assetMint.decimals);
            if (amount === 0n) throw new Error('Enter an amount');
            const request = await generateKeyPairSigner();
            const { ata: userAta, instruction: ataIx } = await buildCreateAtaIdempotentIx({
                kind: vault.assetTokenProgram,
                mint: vault.base.assetMint,
                owner,
                payer: signer,
            });
            const depositIx = buildCreateDepositRequestIx({
                amount,
                assetMint: vault.base.assetMint,
                assetTokenProgram: vault.assetTokenProgram,
                operator: null,
                pendingVault: vault.pdas.pendingVault,
                request,
                shareMint: vault.shareMint,
                shareTokenProgram: vault.shareTokenProgram,
                user: signer,
                userTokenAccount: userAta,
                vault: vault.pdas.vault,
            });
            const sig = await send([ataIx, depositIx], { action: 'Deposit request' });
            if (sig) {
                toast.success('Deposit request submitted');
                setDepositAmount('');
                refreshAll();
            }
        } catch (err) {
            toast.error('Deposit failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleRedeem = async () => {
        const signer = createSigner();
        if (!signer || !owner) {
            toast.error('Connect a wallet first');
            return;
        }
        try {
            const amount = parseTokenAmount(redeemAmount || '0', vault.shareMintInfo.decimals);
            if (amount === 0n) throw new Error('Enter an amount');
            const request = await generateKeyPairSigner();
            const { ata: userShare, instruction: ataIx } = await buildCreateAtaIdempotentIx({
                kind: vault.shareTokenProgram,
                mint: vault.shareMint,
                owner,
                payer: signer,
            });
            const redeemIx = buildCreateRedeemRequestIx({
                amount,
                assetMint: vault.base.assetMint,
                assetTokenProgram: vault.assetTokenProgram,
                operator: null,
                pendingVault: vault.pdas.pendingVault,
                request,
                shareMint: vault.shareMint,
                shareTokenProgram: vault.shareTokenProgram,
                user: signer,
                userShareAccount: userShare,
                vault: vault.pdas.vault,
            });
            const sig = await send([ataIx, redeemIx], { action: 'Redeem request' });
            if (sig) {
                toast.success('Redeem request submitted');
                setRedeemAmount('');
                refreshAll();
            }
        } catch (err) {
            toast.error('Redeem failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleClaim = async (req: VaultRequest) => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const ataIxs = [];
            let userShare: Address | undefined;
            let userAsset: Address | undefined;
            if (req.type === 'deposit') {
                const created = await buildCreateAtaIdempotentIx({
                    kind: vault.shareTokenProgram,
                    mint: vault.shareMint,
                    owner: req.owner,
                    payer: signer,
                });
                userShare = created.ata;
                ataIxs.push(created.instruction);
            } else {
                const created = await buildCreateAtaIdempotentIx({
                    kind: vault.assetTokenProgram,
                    mint: vault.base.assetMint,
                    owner: req.owner,
                    payer: signer,
                });
                userAsset = created.ata;
                ataIxs.push(created.instruction);
            }
            const claimIx = buildClaimIx({
                assetMint: vault.base.assetMint,
                assetTokenProgram: vault.assetTokenProgram,
                owner: req.owner,
                pendingVault: req.type === 'redeem' ? vault.pdas.pendingVault : undefined,
                request: req.address,
                requestType: req.type,
                shareMint: vault.shareMint,
                shareTokenProgram: vault.shareTokenProgram,
                user: signer,
                userAssetAccount: userAsset,
                userShareAccount: userShare,
                vault: vault.pdas.vault,
            });
            const sig = await send([...ataIxs, claimIx], { action: 'Claim' });
            if (sig) {
                toast.success('Claimed');
                refreshAll();
            }
        } catch (err) {
            toast.error('Claim failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleCancel = async (req: VaultRequest) => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const useQueue = (req.type === 'deposit' && hasSubQueue) || (req.type === 'redeem' && hasRedeemQueue);
            let ix;
            if (useQueue && req.type === 'deposit') {
                const userTokenAccount = await getAtaAddress(vault.base.assetMint, req.owner, vault.assetTokenProgram);
                ix = buildCancelQueuedDepositIx({
                    assetMint: vault.base.assetMint,
                    assetPendingVault: vault.pdas.pendingVault,
                    assetTokenProgram: vault.assetTokenProgram,
                    request: req.address,
                    shareMint: vault.shareMint,
                    user: signer,
                    userTokenAccount,
                    vault: vault.pdas.vault,
                });
            } else if (useQueue && req.type === 'redeem') {
                const userShareAccount = await getAtaAddress(vault.shareMint, req.owner, vault.shareTokenProgram);
                ix = buildCancelQueuedRedemptionIx({
                    assetMint: vault.base.assetMint,
                    request: req.address,
                    shareMint: vault.shareMint,
                    shareTokenProgram: vault.shareTokenProgram,
                    user: signer,
                    userShareAccount,
                    vault: vault.pdas.vault,
                });
            } else {
                const userTokenAccount =
                    req.type === 'deposit'
                        ? await getAtaAddress(vault.base.assetMint, req.owner, vault.assetTokenProgram)
                        : undefined;
                const userShareAccount =
                    req.type === 'redeem'
                        ? await getAtaAddress(vault.shareMint, req.owner, vault.shareTokenProgram)
                        : undefined;
                ix = buildCancelRequestIx({
                    assetMint: vault.base.assetMint,
                    assetPendingVault: req.type === 'deposit' ? vault.pdas.pendingVault : undefined,
                    assetTokenProgram: vault.assetTokenProgram,
                    request: req.address,
                    requestType: req.type,
                    shareMint: vault.shareMint,
                    shareTokenProgram: vault.shareTokenProgram,
                    user: signer,
                    userShareAccount,
                    userTokenAccount,
                    vault: vault.pdas.vault,
                });
            }
            const sig = await send([ix], { action: 'Cancel request' });
            if (sig) {
                toast.success('Request canceled');
                refreshAll();
            }
        } catch (err) {
            toast.error('Cancel failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    const handleSetOperator = async (req: VaultRequest) => {
        const signer = createSigner();
        if (!signer) return;
        try {
            const operator = await generateKeyPairSigner();
            const ix = buildSetOperatorIx({ operator, request: req.address, user: signer });
            const sig = await send([ix], { action: 'Set operator' });
            if (sig) {
                toast.info('Ephemeral operator', { description: `Operator pubkey: ${operator.address}` });
                refreshAll();
            }
        } catch (err) {
            toast.error('Set operator failed', { description: err instanceof Error ? err.message : String(err) });
        }
    };

    return (
        <div className="space-y-6">
            {sendError ? (
                <Card className="border-destructive/40 bg-destructive/5">
                    <CardContent className="p-4 text-sm text-destructive">{sendError}</CardContent>
                </Card>
            ) : null}

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
                            variant="default"
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
                        addr={userAssetAta ?? undefined}
                    />
                    <Stat
                        label="Share wallet"
                        value={
                            shareBalance != null ? formatTokenAmount(shareBalance, vault.shareMintInfo.decimals) : '—'
                        }
                        addr={userShareAta ?? undefined}
                    />
                    <Stat label="Connected" value={owner ? owner.slice(0, 8) + '…' : '—'} />
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
