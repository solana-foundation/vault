'use client';

import * as React from 'react';
import { ArrowDownToLine, ArrowUpFromLine, ChevronRight } from 'lucide-react';

import { AddressPill } from '@/components/ui/address';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { formatTokenAmount } from '@/lib/format';
import type { VaultRequest, VaultState } from '@/lib/hooks/use-vault';

export function RequestList({
    title,
    description,
    requests,
    vault,
    actions,
    emptyLabel,
}: {
    title: string;
    description?: string;
    requests: VaultRequest[];
    vault: VaultState;
    actions?: (req: VaultRequest) => React.ReactNode;
    emptyLabel?: string;
}) {
    return (
        <Card>
            <CardHeader>
                <CardTitle className="text-base">{title}</CardTitle>
                {description ? <CardDescription>{description}</CardDescription> : null}
            </CardHeader>
            <CardContent>
                {requests.length === 0 ? (
                    <p className="py-4 text-center text-sm text-muted-foreground">
                        {emptyLabel ?? 'No requests yet.'}
                    </p>
                ) : (
                    <ul className="space-y-2">
                        {requests.map((r) => (
                            <li key={r.address.toBase58()}>
                                <RequestRow req={r} vault={vault} actions={actions?.(r)} />
                            </li>
                        ))}
                    </ul>
                )}
            </CardContent>
        </Card>
    );
}

function stateBadge(state: VaultRequest['state']) {
    if (state === 'pending') return <Badge variant="warning">Pending</Badge>;
    if (state === 'claimable') return <Badge variant="success">Claimable</Badge>;
    return <Badge variant="muted">Canceled</Badge>;
}

function RequestRow({
    req,
    vault,
    actions,
}: {
    req: VaultRequest;
    vault: VaultState;
    actions?: React.ReactNode;
}) {
    const decimals = req.type === 'deposit' ? vault.assetMint.decimals : vault.shareMintInfo.decimals;
    const symbol = req.type === 'deposit' ? 'asset' : 'shares';
    const amount = formatTokenAmount(req.amount, decimals);

    return (
        <div className="rounded-md border border-border/60 bg-background/40 p-3">
            <div className="flex flex-wrap items-center gap-3">
                <span className="inline-flex items-center gap-1.5 text-sm font-medium">
                    {req.type === 'deposit' ? (
                        <ArrowDownToLine className="size-4 text-success" />
                    ) : (
                        <ArrowUpFromLine className="size-4 text-warning" />
                    )}
                    <span className="capitalize">{req.type}</span>
                </span>
                {stateBadge(req.state)}
                <span className="font-mono text-sm tabular-nums">
                    {amount} {symbol}
                </span>
                <AddressPill value={req.address.toBase58()} chars={4} />
                <ChevronRight className="size-3 text-muted-foreground" />
                <AddressPill label="owner" value={req.owner.toBase58()} chars={4} />
                {req.operator ? (
                    <AddressPill label="op" value={req.operator.toBase58()} chars={4} />
                ) : null}
                {req.state === 'claimable' && req.price > 0n ? (
                    <span className="text-xs text-muted-foreground">
                        @ NAV {formatTokenAmount(req.price, vault.assetMint.decimals)}
                    </span>
                ) : null}
                {actions ? <span className="ml-auto flex items-center gap-2">{actions}</span> : null}
            </div>
        </div>
    );
}
