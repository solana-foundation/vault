import * as React from 'react';
import { Link, useNavigate } from 'react-router';
import { isAddress } from '@solana/kit';
import { ArrowRight, Trash2, Vault as VaultIcon } from 'lucide-react';

import { AddressPill } from '@/components/ui/address';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { getKnownVaults, removeKnownVault, type KnownVault } from '@/lib/vault-storage';

export function VaultsRoute() {
    const navigate = useNavigate();
    const [vaults, setVaults] = React.useState<KnownVault[]>([]);
    const [lookup, setLookup] = React.useState('');
    const [error, setError] = React.useState<string | null>(null);

    React.useEffect(() => {
        setVaults(getKnownVaults());
    }, []);

    const handleOpen = (e: React.FormEvent) => {
        e.preventDefault();
        setError(null);
        const value = lookup.trim();
        if (!isAddress(value)) {
            setError('That doesn’t look like a valid Solana address.');
            return;
        }
        void navigate(`/vault/${value}`);
    };

    return (
        <div className="space-y-8">
            <div className="flex items-end justify-between gap-4">
                <div>
                    <h1 className="text-3xl font-semibold">My vaults</h1>
                    <p className="mt-2 text-sm text-muted-foreground">
                        Vaults you&apos;ve created or opened on this device. Stored locally — clear browser data wipes
                        the list.
                    </p>
                </div>
                <Link to="/create">
                    <Button variant="default">New vault</Button>
                </Link>
            </div>

            <Card>
                <CardHeader>
                    <CardTitle>Open by share mint</CardTitle>
                    <CardDescription>
                        Already know the share mint address? Paste it here to jump straight in.
                    </CardDescription>
                </CardHeader>
                <CardContent>
                    <form onSubmit={handleOpen} className="flex flex-col gap-3 md:flex-row md:items-end">
                        <div className="flex-1">
                            <Label>Share mint address</Label>
                            <Input
                                className="mt-1.5"
                                value={lookup}
                                onChange={e => setLookup(e.target.value)}
                                placeholder="So1AnAa…"
                            />
                            {error ? <p className="mt-1 text-xs text-destructive">{error}</p> : null}
                        </div>
                        <Button type="submit" disabled={!lookup.trim()}>
                            Open <ArrowRight className="size-4" />
                        </Button>
                    </form>
                </CardContent>
            </Card>

            {vaults.length === 0 ? (
                <Card className="bg-card/40">
                    <CardContent className="flex flex-col items-center gap-3 p-12 text-center">
                        <VaultIcon className="size-8 text-muted-foreground" />
                        <p className="text-sm text-muted-foreground">No vaults yet.</p>
                        <Link to="/create">
                            <Button variant="outline">Create your first vault</Button>
                        </Link>
                    </CardContent>
                </Card>
            ) : (
                <div className="grid gap-3 md:grid-cols-2">
                    {vaults.map(v => (
                        <Card key={v.shareMint} className="bg-card/40">
                            <CardContent className="flex flex-col gap-3 p-5">
                                <div className="flex items-start justify-between gap-2">
                                    <div>
                                        <p className="font-medium">{v.label || 'Unnamed vault'}</p>
                                        <p className="text-xs text-muted-foreground">
                                            Created {new Date(v.createdAt).toLocaleString()}
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        className="text-muted-foreground transition hover:text-destructive"
                                        title="Forget this vault"
                                        onClick={() => {
                                            removeKnownVault(v.shareMint);
                                            setVaults(getKnownVaults());
                                        }}
                                    >
                                        <Trash2 className="size-4" />
                                    </button>
                                </div>
                                <div className="space-y-1.5">
                                    <AddressPill value={v.shareMint} label="share" />
                                    <AddressPill value={v.assetMint} label="asset" />
                                </div>
                                <Link to={`/vault/${v.shareMint}`} className="self-end">
                                    <Button size="sm">
                                        Open <ArrowRight className="size-4" />
                                    </Button>
                                </Link>
                            </CardContent>
                        </Card>
                    ))}
                </div>
            )}
        </div>
    );
}
