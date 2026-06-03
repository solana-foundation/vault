'use client';

import * as React from 'react';
import Link from 'next/link';
import { useParams } from 'next/navigation';
import { useWallet } from '@solana/wallet-adapter-react';
import { Loader2, RefreshCcw } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { AuthorityActions } from '@/components/vault/authority-actions';
import { ExtensionList } from '@/components/vault/extension-list';
import { RequestList } from '@/components/vault/request-list';
import { UserActions } from '@/components/vault/user-actions';
import { VaultSummary } from '@/components/vault/vault-summary';
import { useVault } from '@/lib/hooks/use-vault';
import { getKnownVaults } from '@/lib/vault-storage';

export default function VaultDetailPage() {
    const params = useParams<{ shareMint: string }>();
    const wallet = useWallet();
    const { state, requests, loading, error, refresh } = useVault(params.shareMint);
    const [tab, setTab] = React.useState<'overview' | 'authority' | 'user'>('overview');

    const known = React.useMemo(
        () => getKnownVaults().find((v) => v.shareMint === params.shareMint),
        [params.shareMint],
    );

    if (error) {
        return (
            <Card className="mx-auto max-w-xl border-destructive/40 bg-destructive/10">
                <CardContent className="space-y-3 p-6">
                    <h2 className="text-lg font-semibold">Couldn&apos;t load vault</h2>
                    <p className="text-sm text-muted-foreground">{error.message}</p>
                    <p className="text-xs text-muted-foreground">
                        Make sure the program is deployed on the configured cluster and the share mint address is
                        correct.
                    </p>
                    <Link href="/vaults" className="text-sm underline-offset-4 hover:underline">
                        ← Back to vaults
                    </Link>
                </CardContent>
            </Card>
        );
    }

    if (!state || !requests) {
        return (
            <div className="flex items-center justify-center gap-2 py-24 text-muted-foreground">
                <Loader2 className="size-5 animate-spin" />
                <span>Fetching vault…</span>
            </div>
        );
    }

    const isAuthority =
        wallet.publicKey?.toBase58() === (state.base.authority as unknown as string);

    return (
        <div className="space-y-6">
            <div className="flex items-center justify-between">
                <div>
                    <Link
                        href="/vaults"
                        className="text-xs uppercase tracking-wide text-muted-foreground hover:text-foreground"
                    >
                        ← All vaults
                    </Link>
                </div>
                <Button variant="ghost" size="sm" onClick={refresh}>
                    <RefreshCcw className={`size-4 ${loading ? 'animate-spin' : ''}`} /> Refresh
                </Button>
            </div>

            <VaultSummary vault={state} label={known?.label} />

            <Tabs value={tab} onValueChange={(v) => setTab(v as typeof tab)}>
                <TabsList>
                    <TabsTrigger value="overview">Overview</TabsTrigger>
                    <TabsTrigger value="user">User portal</TabsTrigger>
                    <TabsTrigger value="authority">
                        Authority {isAuthority ? <span className="ml-1 text-success">●</span> : null}
                    </TabsTrigger>
                </TabsList>

                <TabsContent value="overview" className="space-y-6">
                    <ExtensionList
                        extensions={state.extensions}
                        assetDecimals={state.assetMint.decimals}
                        shareDecimals={state.shareMintInfo.decimals}
                    />
                    <RequestList
                        title={`Open requests (${requests.filter((r) => r.state !== 'canceled').length})`}
                        description="A live view of every async request currently held by this vault."
                        requests={requests.filter((r) => r.state !== 'canceled')}
                        vault={state}
                        emptyLabel="No open requests."
                    />
                </TabsContent>

                <TabsContent value="user" className="space-y-6">
                    <UserActions vault={state} requests={requests} onRefresh={refresh} />
                </TabsContent>

                <TabsContent value="authority" className="space-y-6">
                    <AuthorityActions
                        vault={state}
                        requests={requests}
                        onRefresh={refresh}
                        demoAssetMintAuthority={known?.demoAssetMintAuthority}
                    />
                </TabsContent>
            </Tabs>
        </div>
    );
}
