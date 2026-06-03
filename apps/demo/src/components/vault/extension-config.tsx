'use client';

import * as React from 'react';

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent } from '@/components/ui/card';

import { ExtensionType, EXTENSION_DESCRIPTIONS, EXTENSION_LABELS, type ExtensionTypeValue } from '@/lib/extensions';

export interface ExtensionsConfig {
    depositFee: { enabled: boolean; bps: number };
    withdrawalFee: { enabled: boolean; bps: number };
    pausableSubscriptions: { enabled: boolean; paused: boolean };
    pausableRedemptions: { enabled: boolean; paused: boolean };
    minSubscription: { enabled: boolean; threshold: string };
    minRedemption: { enabled: boolean; threshold: string };
    subscriptionQueue: { enabled: boolean };
    redemptionQueue: { enabled: boolean };
}

export const DEFAULT_EXT_CONFIG: ExtensionsConfig = {
    depositFee: { enabled: false, bps: 50 },
    withdrawalFee: { enabled: false, bps: 50 },
    pausableSubscriptions: { enabled: false, paused: false },
    pausableRedemptions: { enabled: false, paused: false },
    minSubscription: { enabled: false, threshold: '1' },
    minRedemption: { enabled: false, threshold: '1' },
    subscriptionQueue: { enabled: false },
    redemptionQueue: { enabled: false },
};

interface RowProps {
    type: ExtensionTypeValue;
    enabled: boolean;
    onToggle: (v: boolean) => void;
    children?: React.ReactNode;
}

function Row({ type, enabled, onToggle, children }: RowProps) {
    return (
        <Card className="bg-card/50">
            <CardContent className="flex flex-col gap-3 p-4">
                <div className="flex items-start justify-between gap-3">
                    <div className="space-y-1">
                        <p className="text-sm font-medium">{EXTENSION_LABELS[type]}</p>
                        <p className="text-xs text-muted-foreground">{EXTENSION_DESCRIPTIONS[type]}</p>
                    </div>
                    <Switch checked={enabled} onCheckedChange={onToggle} />
                </div>
                {enabled && children ? <div className="border-t border-border/60 pt-3">{children}</div> : null}
            </CardContent>
        </Card>
    );
}

export function ExtensionConfigForm({
    config,
    onChange,
}: {
    config: ExtensionsConfig;
    onChange: (next: ExtensionsConfig) => void;
}) {
    const set = <K extends keyof ExtensionsConfig>(key: K, value: ExtensionsConfig[K]) =>
        onChange({ ...config, [key]: value });

    return (
        <div className="grid gap-3 md:grid-cols-2">
            <Row
                type={ExtensionType.DepositFee}
                enabled={config.depositFee.enabled}
                onToggle={enabled => set('depositFee', { ...config.depositFee, enabled })}
            >
                <Label className="mb-1 block text-xs text-muted-foreground">Fee (basis points · 100 = 1%)</Label>
                <Input
                    type="number"
                    min={0}
                    max={10000}
                    value={config.depositFee.bps}
                    onChange={e => set('depositFee', { ...config.depositFee, bps: Number(e.target.value) })}
                />
            </Row>
            <Row
                type={ExtensionType.WithdrawalFee}
                enabled={config.withdrawalFee.enabled}
                onToggle={enabled => set('withdrawalFee', { ...config.withdrawalFee, enabled })}
            >
                <Label className="mb-1 block text-xs text-muted-foreground">Fee (basis points)</Label>
                <Input
                    type="number"
                    min={0}
                    max={10000}
                    value={config.withdrawalFee.bps}
                    onChange={e => set('withdrawalFee', { ...config.withdrawalFee, bps: Number(e.target.value) })}
                />
            </Row>
            <Row
                type={ExtensionType.PausableSubscriptions}
                enabled={config.pausableSubscriptions.enabled}
                onToggle={enabled => set('pausableSubscriptions', { ...config.pausableSubscriptions, enabled })}
            >
                <div className="flex items-center justify-between">
                    <Label className="text-xs text-muted-foreground">Start paused</Label>
                    <Switch
                        checked={config.pausableSubscriptions.paused}
                        onCheckedChange={paused =>
                            set('pausableSubscriptions', { ...config.pausableSubscriptions, paused })
                        }
                    />
                </div>
            </Row>
            <Row
                type={ExtensionType.PausableRedemptions}
                enabled={config.pausableRedemptions.enabled}
                onToggle={enabled => set('pausableRedemptions', { ...config.pausableRedemptions, enabled })}
            >
                <div className="flex items-center justify-between">
                    <Label className="text-xs text-muted-foreground">Start paused</Label>
                    <Switch
                        checked={config.pausableRedemptions.paused}
                        onCheckedChange={paused =>
                            set('pausableRedemptions', { ...config.pausableRedemptions, paused })
                        }
                    />
                </div>
            </Row>
            <Row
                type={ExtensionType.MinSubscription}
                enabled={config.minSubscription.enabled}
                onToggle={enabled => set('minSubscription', { ...config.minSubscription, enabled })}
            >
                <Label className="mb-1 block text-xs text-muted-foreground">Minimum deposit (asset units)</Label>
                <Input
                    inputMode="decimal"
                    value={config.minSubscription.threshold}
                    onChange={e => set('minSubscription', { ...config.minSubscription, threshold: e.target.value })}
                />
            </Row>
            <Row
                type={ExtensionType.MinRedemption}
                enabled={config.minRedemption.enabled}
                onToggle={enabled => set('minRedemption', { ...config.minRedemption, enabled })}
            >
                <Label className="mb-1 block text-xs text-muted-foreground">Minimum redeem (share units)</Label>
                <Input
                    inputMode="decimal"
                    value={config.minRedemption.threshold}
                    onChange={e => set('minRedemption', { ...config.minRedemption, threshold: e.target.value })}
                />
            </Row>
            <Row
                type={ExtensionType.SubscriptionQueue}
                enabled={config.subscriptionQueue.enabled}
                onToggle={enabled => set('subscriptionQueue', { enabled })}
            />
            <Row
                type={ExtensionType.RedemptionQueue}
                enabled={config.redemptionQueue.enabled}
                onToggle={enabled => set('redemptionQueue', { enabled })}
            />
        </div>
    );
}
