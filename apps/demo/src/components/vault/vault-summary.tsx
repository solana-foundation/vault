'use client';

import * as React from 'react';
import { Pause, Play } from 'lucide-react';

import { AddressPill } from '@/components/ui/address';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { formatTokenAmount } from '@/lib/format';
import type { VaultState } from '@/lib/hooks/use-vault';

export function VaultSummary({ vault, label }: { vault: VaultState; label?: string }) {
    const { base, assetMint, shareMintInfo, reserveBalance, pendingBalance } = vault;

    const navDisplay = formatTokenAmount(base.nav, assetMint.decimals);
    const totalAssets = formatTokenAmount(base.totalAssetBalance, assetMint.decimals);
    const reserve = formatTokenAmount(reserveBalance ?? 0n, assetMint.decimals);
    const pending = formatTokenAmount(pendingBalance ?? 0n, assetMint.decimals);
    const supply = formatTokenAmount(shareMintInfo.supply, shareMintInfo.decimals);

    return (
        <Card className="overflow-hidden">
            <div className="relative bg-gradient-to-br from-primary/15 via-transparent to-solana-green/10 px-6 py-5">
                <div className="flex flex-wrap items-center gap-3">
                    {label ? <h2 className="text-xl font-semibold">{label}</h2> : null}
                    <Badge variant={base.paused ? 'warning' : 'success'} className="gap-1">
                        {base.paused ? <Pause className="size-3" /> : <Play className="size-3" />}
                        {base.paused ? 'Paused' : 'Active'}
                    </Badge>
                    <Badge variant={base.initialized ? 'secondary' : 'outline'}>
                        {base.initialized ? 'Initialized' : 'Pending init'}
                    </Badge>
                    <Badge variant="outline">{`NAV v${base.navVersion}`}</Badge>
                    <Badge variant="outline">{`${base.pendingAsyncRequests} pending`}</Badge>
                </div>
                <div className="mt-2 flex flex-wrap items-center gap-2">
                    <AddressPill value={vault.shareMint.toBase58()} label="share" chars={6} />
                    <AddressPill value={base.assetMint as unknown as string} label="asset" chars={6} />
                    <AddressPill value={base.authority as unknown as string} label="authority" chars={6} />
                    {base.pendingAuthority.__option === 'Some' ? (
                        <AddressPill
                            value={base.pendingAuthority.value as unknown as string}
                            label="pending"
                            chars={6}
                        />
                    ) : null}
                </div>
            </div>
            <CardContent className="grid gap-4 border-t border-border/60 p-6 md:grid-cols-5">
                <Stat label="NAV" value={navDisplay} sub={`per share · v${base.navVersion}`} />
                <Stat label="Total assets" value={totalAssets} sub="virtual balance" />
                <Stat label="Reserve" value={reserve} sub="vault token account" />
                <Stat label="Pending escrow" value={pending} sub="pending vault" />
                <Stat label="Share supply" value={supply} sub={`${shareMintInfo.decimals} dec`} />
            </CardContent>
        </Card>
    );
}

function Stat({ label, value, sub }: { label: string; value: string; sub?: string }) {
    return (
        <div className="space-y-0.5">
            <p className="text-xs uppercase tracking-wide text-muted-foreground">{label}</p>
            <p className="font-mono text-xl font-semibold tabular-nums">{value}</p>
            {sub ? <p className="text-[11px] text-muted-foreground">{sub}</p> : null}
        </div>
    );
}
