'use client';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
    EXTENSION_DESCRIPTIONS,
    EXTENSION_LABELS,
    ExtensionType,
    type ParsedExtension,
} from '@/lib/extensions';
import { formatTokenAmount } from '@/lib/format';

export function ExtensionList({
    extensions,
    assetDecimals,
    shareDecimals,
}: {
    extensions: ParsedExtension[];
    assetDecimals: number;
    shareDecimals: number;
}) {
    if (extensions.length === 0) {
        return (
            <Card className="bg-card/40">
                <CardHeader>
                    <CardTitle className="text-base">Extensions</CardTitle>
                    <CardDescription>No extensions are configured on this vault.</CardDescription>
                </CardHeader>
            </Card>
        );
    }

    return (
        <Card className="bg-card/40">
            <CardHeader>
                <CardTitle className="text-base">Extensions ({extensions.length})</CardTitle>
                <CardDescription>
                    Each extension is a TLV entry appended to the vault account. Toggleable behavior is wired to
                    core instructions.
                </CardDescription>
            </CardHeader>
            <CardContent>
                <ul className="space-y-3">
                    {extensions.map((ext) => (
                        <li key={ext.type} className="rounded-md border border-border/60 bg-background/40 p-3">
                            <div className="flex flex-wrap items-center justify-between gap-2">
                                <p className="text-sm font-medium">{EXTENSION_LABELS[ext.type]}</p>
                                <ExtensionState ext={ext} assetDecimals={assetDecimals} shareDecimals={shareDecimals} />
                            </div>
                            <p className="mt-1 text-xs text-muted-foreground">{EXTENSION_DESCRIPTIONS[ext.type]}</p>
                        </li>
                    ))}
                </ul>
            </CardContent>
        </Card>
    );
}

function ExtensionState({
    ext,
    assetDecimals,
    shareDecimals,
}: {
    ext: ParsedExtension;
    assetDecimals: number;
    shareDecimals: number;
}) {
    switch (ext.type) {
        case ExtensionType.DepositFee:
        case ExtensionType.WithdrawalFee:
            return (
                <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="secondary">{(ext.bps / 100).toFixed(2)}%</Badge>
                    <span className="text-xs text-muted-foreground">
                        last fee {formatTokenAmount(ext.lastFee, assetDecimals)}
                    </span>
                </div>
            );
        case ExtensionType.PausableSubscriptions:
        case ExtensionType.PausableRedemptions:
            return <Badge variant={ext.paused ? 'warning' : 'success'}>{ext.paused ? 'Paused' : 'Open'}</Badge>;
        case ExtensionType.SubscriptionQueue:
        case ExtensionType.RedemptionQueue:
            return (
                <span className="font-mono text-xs text-muted-foreground">
                    head={ext.head.toString()} tail={ext.tail.toString()}
                </span>
            );
        case ExtensionType.MinSubscription:
            return (
                <Badge variant="secondary">≥ {formatTokenAmount(ext.threshold, assetDecimals)}</Badge>
            );
        case ExtensionType.MinRedemption:
            return (
                <Badge variant="secondary">≥ {formatTokenAmount(ext.threshold, shareDecimals)}</Badge>
            );
    }
}
