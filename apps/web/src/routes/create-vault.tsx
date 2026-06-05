import * as React from 'react';
import { Link, useNavigate } from 'react-router';
import { isAddress, type Address, type Instruction } from '@solana/kit';
import { ArrowRight, CheckCircle2, Sparkles } from 'lucide-react';
import { toast } from 'sonner';

import { AddressPill } from '@/components/ui/address';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { DEFAULT_EXT_CONFIG, ExtensionConfigForm, type ExtensionsConfig } from '@/components/vault/extension-config';
import { useWallet } from '@/contexts/WalletContext';
import { useSendTx } from '@/hooks/useSendTx';
import { useRpc } from '@/hooks/useRpc';
import { parseTokenAmount } from '@/lib/format';
import {
    buildCreateVaultIx,
    buildInitDepositFeeIx,
    buildInitMinRedemptionIx,
    buildInitMinSubscriptionIx,
    buildInitPausableRedemptionsIx,
    buildInitPausableSubscriptionsIx,
    buildInitRedemptionQueueIx,
    buildInitSubscriptionQueueIx,
    buildInitWithdrawalFeeIx,
    buildInitializeVaultIx,
    deriveVaultPdas,
} from '@/lib/program';
import {
    buildCreateAtaIdempotentIx,
    buildCreateMintInstructions,
    buildMintToIx,
    fetchMintInfo,
    type TokenProgramKind,
} from '@/lib/token';
import { saveKnownVault } from '@/lib/vault-storage';

type AssetMode = 'demo' | 'existing';

function requireAddress(value: string, field: string): Address {
    const trimmed = value.trim();
    if (!isAddress(trimmed)) throw new Error(`${field} is not a valid public key`);
    return trimmed;
}

export function CreateVaultRoute() {
    const navigate = useNavigate();
    const { account, createSigner } = useWallet();
    const { send } = useSendTx();
    const rpc = useRpc();
    const connected = Boolean(account);

    const [step, setStep] = React.useState(1);
    const [submitting, setSubmitting] = React.useState(false);

    const [label, setLabel] = React.useState('Demo RWA Vault');
    const [assetMode, setAssetMode] = React.useState<AssetMode>('demo');
    const [assetMintInput, setAssetMintInput] = React.useState('');
    const [assetDecimals, setAssetDecimals] = React.useState(6);
    const [shareDecimals, setShareDecimals] = React.useState(6);
    const [demoMintAmount, setDemoMintAmount] = React.useState('100000');
    const [tokenProgram, setTokenProgram] = React.useState<TokenProgramKind>('spl');
    const [feeRecipientInput, setFeeRecipientInput] = React.useState('');

    const [extensions, setExtensions] = React.useState<ExtensionsConfig>(DEFAULT_EXT_CONFIG);

    const handleSubmit = async () => {
        const signer = createSigner();
        const owner = account?.address as Address | undefined;
        if (!signer || !owner) {
            toast.error('Connect a wallet first');
            return;
        }
        const feeRecipient: Address = feeRecipientInput.trim()
            ? requireAddress(feeRecipientInput, 'Fee recipient')
            : owner;

        try {
            setSubmitting(true);

            let assetMint: Address;
            let assetDecimalsResolved = assetDecimals;
            let assetTokenProgram: TokenProgramKind = tokenProgram;

            if (assetMode === 'demo') {
                const demoMint = await buildCreateMintInstructions({
                    decimals: assetDecimals,
                    kind: tokenProgram,
                    mintAuthority: owner,
                    payer: signer,
                    rpc,
                });
                assetMint = demoMint.mint.address;
                const { ata, instruction: ataIx } = await buildCreateAtaIdempotentIx({
                    kind: tokenProgram,
                    mint: assetMint,
                    owner,
                    payer: signer,
                });
                const mintToIx = buildMintToIx({
                    amount: parseTokenAmount(demoMintAmount || '0', assetDecimals),
                    authority: signer,
                    destination: ata,
                    kind: tokenProgram,
                    mint: assetMint,
                });
                const sig = await send([...demoMint.instructions, ataIx, mintToIx], {
                    action: 'Create demo asset mint',
                });
                if (!sig) throw new Error('Demo asset mint transaction failed');
            } else {
                assetMint = requireAddress(assetMintInput, 'Asset mint');
                const info = await fetchMintInfo(rpc, assetMint);
                assetDecimalsResolved = info.decimals;
                assetTokenProgram = info.tokenProgram;
            }

            const shareMintCreate = await buildCreateMintInstructions({
                decimals: shareDecimals,
                kind: tokenProgram,
                mintAuthority: owner,
                payer: signer,
                rpc,
            });
            const shareMint = shareMintCreate.mint.address;

            const pdas = await deriveVaultPdas(shareMint);
            const createVaultIx = await buildCreateVaultIx({
                assetMint,
                assetTokenProgram,
                authority: owner,
                feeRecipient,
                mintAuthority: signer,
                payer: signer,
                pendingVault: pdas.pendingVault,
                reserve: pdas.reserve,
                shareMint,
                shareTokenProgram: tokenProgram,
                vault: pdas.vault,
            });

            const extensionIxs: Instruction[] = [];
            if (extensions.depositFee.enabled) {
                extensionIxs.push(
                    buildInitDepositFeeIx({
                        authority: signer,
                        bps: extensions.depositFee.bps,
                        payer: signer,
                        vault: pdas.vault,
                    }),
                );
            }
            if (extensions.withdrawalFee.enabled) {
                extensionIxs.push(
                    buildInitWithdrawalFeeIx({
                        authority: signer,
                        bps: extensions.withdrawalFee.bps,
                        payer: signer,
                        vault: pdas.vault,
                    }),
                );
            }
            if (extensions.pausableSubscriptions.enabled) {
                extensionIxs.push(
                    buildInitPausableSubscriptionsIx({
                        authority: signer,
                        paused: extensions.pausableSubscriptions.paused,
                        payer: signer,
                        vault: pdas.vault,
                    }),
                );
            }
            if (extensions.pausableRedemptions.enabled) {
                extensionIxs.push(
                    buildInitPausableRedemptionsIx({
                        authority: signer,
                        paused: extensions.pausableRedemptions.paused,
                        payer: signer,
                        vault: pdas.vault,
                    }),
                );
            }
            if (extensions.minSubscription.enabled) {
                extensionIxs.push(
                    buildInitMinSubscriptionIx({
                        authority: signer,
                        payer: signer,
                        threshold: parseTokenAmount(extensions.minSubscription.threshold || '0', assetDecimalsResolved),
                        vault: pdas.vault,
                    }),
                );
            }
            if (extensions.minRedemption.enabled) {
                extensionIxs.push(
                    buildInitMinRedemptionIx({
                        authority: signer,
                        payer: signer,
                        threshold: parseTokenAmount(extensions.minRedemption.threshold || '0', shareDecimals),
                        vault: pdas.vault,
                    }),
                );
            }
            if (extensions.subscriptionQueue.enabled) {
                extensionIxs.push(
                    buildInitSubscriptionQueueIx({ authority: signer, payer: signer, vault: pdas.vault }),
                );
            }
            if (extensions.redemptionQueue.enabled) {
                extensionIxs.push(buildInitRedemptionQueueIx({ authority: signer, payer: signer, vault: pdas.vault }));
            }
            const initVaultIx = buildInitializeVaultIx(signer, shareMint, pdas.vault);

            const sig = await send([...shareMintCreate.instructions, createVaultIx, ...extensionIxs, initVaultIx], {
                action: 'Deploy vault',
            });
            if (!sig) throw new Error('Vault deployment transaction failed');

            saveKnownVault({
                assetMint,
                assetTokenProgram,
                createdAt: Date.now(),
                isAuthority: true,
                label,
                shareMint,
                shareTokenProgram: tokenProgram,
                ...(assetMode === 'demo' ? { demoAssetMintAuthority: owner } : {}),
            });

            toast.success('Vault deployed', { description: 'Redirecting…' });
            void navigate(`/vault/${shareMint}`);
        } catch (err) {
            toast.error('Vault creation failed', {
                description: err instanceof Error ? err.message : String(err),
            });
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <div className="mx-auto max-w-4xl space-y-8">
            <div>
                <p className="text-xs uppercase tracking-wide text-muted-foreground">Step {step} of 3</p>
                <h1 className="mt-1 text-3xl font-semibold">Create a demo vault</h1>
                <p className="mt-2 text-sm text-muted-foreground">
                    The wizard will mint a synthetic asset, deploy an{' '}
                    <code className="font-mono text-foreground">async_vault</code> with the extensions you toggle, and
                    drop you into the authority console.
                </p>
            </div>

            {!connected ? (
                <Card>
                    <CardContent className="p-6 text-sm text-muted-foreground">
                        Connect a wallet (top right) to begin. You&apos;ll need a tiny bit of devnet SOL — request an
                        airdrop with{' '}
                        <code className="font-mono text-foreground">
                            solana airdrop 2 &lt;your-pubkey&gt; --url devnet
                        </code>
                        .
                    </CardContent>
                </Card>
            ) : null}

            <ol className="grid grid-cols-3 gap-2 text-xs">
                {['Asset', 'Extensions', 'Review'].map((s, i) => (
                    <li
                        key={s}
                        className={`flex items-center gap-2 rounded-md border px-3 py-2 ${
                            step === i + 1 ? 'border-primary text-foreground' : 'border-border text-muted-foreground'
                        }`}
                    >
                        <span
                            className={`inline-flex size-5 items-center justify-center rounded-full text-[10px] ${
                                step > i + 1
                                    ? 'bg-success text-success-foreground'
                                    : step === i + 1
                                      ? 'bg-primary text-primary-foreground'
                                      : 'bg-muted'
                            }`}
                        >
                            {step > i + 1 ? <CheckCircle2 className="size-3" /> : i + 1}
                        </span>
                        {s}
                    </li>
                ))}
            </ol>

            {step === 1 ? (
                <Card>
                    <CardHeader>
                        <CardTitle>Pick an asset</CardTitle>
                        <CardDescription>
                            The vault accepts deposits in this token. For a self-contained demo, mint a synthetic asset
                            you control.
                        </CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-6">
                        <div>
                            <Label>Vault label (saved locally)</Label>
                            <Input
                                value={label}
                                onChange={e => setLabel(e.target.value)}
                                className="mt-1.5"
                                placeholder="My demo vault"
                            />
                        </div>

                        <div className="grid gap-3 md:grid-cols-2">
                            <button
                                type="button"
                                onClick={() => setAssetMode('demo')}
                                className={`rounded-lg border p-4 text-left transition ${
                                    assetMode === 'demo'
                                        ? 'border-primary bg-primary/10'
                                        : 'border-border bg-card/40 hover:border-border'
                                }`}
                            >
                                <div className="flex items-center gap-2">
                                    <Sparkles className="size-4 text-primary" />
                                    <p className="font-medium">Synthetic demo asset</p>
                                </div>
                                <p className="mt-2 text-xs text-muted-foreground">
                                    Mints a fake stablecoin you control + airdrops some to your wallet so you can
                                    deposit immediately.
                                </p>
                            </button>
                            <button
                                type="button"
                                onClick={() => setAssetMode('existing')}
                                className={`rounded-lg border p-4 text-left transition ${
                                    assetMode === 'existing'
                                        ? 'border-primary bg-primary/10'
                                        : 'border-border bg-card/40 hover:border-border'
                                }`}
                            >
                                <p className="font-medium">Use existing mint</p>
                                <p className="mt-2 text-xs text-muted-foreground">
                                    Paste a devnet mint address (e.g. devnet USDC). The decimals + token program are
                                    auto-detected.
                                </p>
                            </button>
                        </div>

                        {assetMode === 'demo' ? (
                            <div className="grid gap-4 md:grid-cols-3">
                                <div>
                                    <Label>Token program</Label>
                                    <select
                                        className="mt-1.5 flex h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
                                        value={tokenProgram}
                                        onChange={e => setTokenProgram(e.target.value as TokenProgramKind)}
                                    >
                                        <option value="spl">SPL Token</option>
                                        <option value="token-2022">Token-2022</option>
                                    </select>
                                </div>
                                <div>
                                    <Label>Asset decimals</Label>
                                    <Input
                                        type="number"
                                        min={0}
                                        max={9}
                                        value={assetDecimals}
                                        onChange={e => setAssetDecimals(Number(e.target.value))}
                                        className="mt-1.5"
                                    />
                                </div>
                                <div>
                                    <Label>Mint to wallet</Label>
                                    <Input
                                        inputMode="decimal"
                                        value={demoMintAmount}
                                        onChange={e => setDemoMintAmount(e.target.value)}
                                        className="mt-1.5"
                                    />
                                </div>
                            </div>
                        ) : (
                            <div>
                                <Label>Asset mint address</Label>
                                <Input
                                    value={assetMintInput}
                                    onChange={e => setAssetMintInput(e.target.value)}
                                    className="mt-1.5"
                                    placeholder="So1AnAa..."
                                />
                            </div>
                        )}

                        <Separator />

                        <div className="grid gap-4 md:grid-cols-2">
                            <div>
                                <Label>Share decimals</Label>
                                <Input
                                    type="number"
                                    min={0}
                                    max={9}
                                    value={shareDecimals}
                                    onChange={e => setShareDecimals(Number(e.target.value))}
                                    className="mt-1.5"
                                />
                                <p className="mt-1 text-xs text-muted-foreground">
                                    The wizard creates a fresh share mint owned by you, then transfers authority to the
                                    vault PDA during <code className="font-mono">create_vault</code>.
                                </p>
                            </div>
                            <div>
                                <Label>Fee recipient (optional)</Label>
                                <Input
                                    value={feeRecipientInput}
                                    onChange={e => setFeeRecipientInput(e.target.value)}
                                    className="mt-1.5"
                                    placeholder="Defaults to your wallet"
                                />
                            </div>
                        </div>

                        <div className="flex justify-end">
                            <Button onClick={() => setStep(2)}>
                                Next: extensions <ArrowRight className="size-4" />
                            </Button>
                        </div>
                    </CardContent>
                </Card>
            ) : null}

            {step === 2 ? (
                <div className="space-y-4">
                    <Card>
                        <CardHeader>
                            <CardTitle>Choose extensions</CardTitle>
                            <CardDescription>
                                Each extension adds bytes to the vault account and conditional logic to core
                                instructions. Toggle as many as you like — they&apos;ll all be initialized in the same
                                transaction.
                            </CardDescription>
                        </CardHeader>
                        <CardContent>
                            <ExtensionConfigForm config={extensions} onChange={setExtensions} />
                        </CardContent>
                    </Card>
                    <div className="flex items-center justify-between">
                        <Button variant="ghost" onClick={() => setStep(1)}>
                            Back
                        </Button>
                        <Button onClick={() => setStep(3)}>
                            Review <ArrowRight className="size-4" />
                        </Button>
                    </div>
                </div>
            ) : null}

            {step === 3 ? (
                <div className="space-y-4">
                    <Card>
                        <CardHeader>
                            <CardTitle>Review &amp; deploy</CardTitle>
                            <CardDescription>
                                The wizard sends two transactions: (1) create the demo asset mint and airdrop tokens to
                                you, then (2) create the share mint, the vault, every extension, and finalize via{' '}
                                <code className="font-mono text-foreground">initialize_vault</code>.
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <ReviewRow label="Vault label" value={label} />
                            <ReviewRow
                                label="Asset"
                                value={
                                    assetMode === 'demo'
                                        ? `Synthetic ${tokenProgram === 'token-2022' ? 'Token-2022' : 'SPL'} (decimals=${assetDecimals}, airdrop=${demoMintAmount})`
                                        : assetMintInput || '—'
                                }
                            />
                            <ReviewRow label="Share mint" value={`Fresh keypair · decimals=${shareDecimals}`} />
                            <ReviewRow
                                label="Authority / payer"
                                value={account ? <AddressPill value={account.address} /> : '—'}
                            />
                            <ReviewRow
                                label="Fee recipient"
                                value={
                                    feeRecipientInput.trim() ? (
                                        <AddressPill value={feeRecipientInput.trim()} />
                                    ) : (
                                        'Wallet'
                                    )
                                }
                            />
                            <Separator />
                            <div>
                                <p className="mb-2 text-sm font-medium">Extensions</p>
                                <div className="flex flex-wrap gap-2">
                                    {Object.entries(extensions)
                                        .filter(([, v]) => (v as { enabled: boolean }).enabled)
                                        .map(([k]) => (
                                            <Badge key={k} variant="secondary">
                                                {k}
                                            </Badge>
                                        ))}
                                    {Object.values(extensions).every(v => !(v as { enabled: boolean }).enabled) ? (
                                        <span className="text-xs text-muted-foreground">No extensions selected</span>
                                    ) : null}
                                </div>
                            </div>
                        </CardContent>
                    </Card>
                    <div className="flex items-center justify-between">
                        <Button variant="ghost" onClick={() => setStep(2)} disabled={submitting}>
                            Back
                        </Button>
                        <Button
                            variant="default"
                            onClick={handleSubmit}
                            disabled={!connected || submitting}
                            loading={submitting}
                        >
                            {submitting ? 'Deploying…' : 'Deploy vault'}
                        </Button>
                    </div>
                </div>
            ) : null}

            <div className="text-center text-xs text-muted-foreground">
                Already have a vault?{' '}
                <Link to="/vaults" className="underline-offset-4 hover:underline">
                    Open it from your dashboard
                </Link>
                .
            </div>
        </div>
    );
}

function ReviewRow({ label, value }: { label: string; value: React.ReactNode }) {
    return (
        <div className="flex items-start justify-between gap-4">
            <span className="text-sm text-muted-foreground">{label}</span>
            <span className="text-right text-sm font-medium">{value}</span>
        </div>
    );
}
