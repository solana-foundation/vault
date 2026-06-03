import Link from 'next/link';
import { ArrowRight, GitBranch, Layers, ShieldCheck, Workflow, Zap } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';

const FEATURES: { icon: React.ComponentType<{ className?: string }>; title: string; body: string }[] = [
    {
        icon: Workflow,
        title: 'Async deposits & redemptions',
        body: 'Requests are queued, NAV is updated by the authority, then users claim — modeled on ERC-7540.',
    },
    {
        icon: Layers,
        title: 'Composable TLV extensions',
        body: 'Fees, pausable flows, min subscription/redemption thresholds, FIFO queues — opt in per vault.',
    },
    {
        icon: ShieldCheck,
        title: 'Bring-your-own share mint',
        body: 'The vault accepts a pre-configured mint (Token / Token-2022) instead of creating one.',
    },
    {
        icon: GitBranch,
        title: 'Two-step authority transfer',
        body: 'Invite + accept — the new authority must sign too. No accidental hand-offs.',
    },
    {
        icon: Zap,
        title: 'Operator delegation',
        body: 'Owners can delegate claim authority to an operator without transferring ownership.',
    },
];

const SCRIPT = [
    'Create a synthetic asset mint and share mint',
    'Deploy the vault with deposit fee + pausable subscriptions',
    'Mint demo asset to your wallet, request a deposit',
    'Switch to authority view, update NAV and approve',
    'Claim the freshly minted shares — done.',
];

export default function HomePage() {
    return (
        <div className="space-y-20">
            <section className="grid gap-10 py-10 md:grid-cols-[3fr_2fr] md:py-20">
                <div className="space-y-6">
                    <div className="inline-flex items-center gap-2 rounded-full border border-border bg-card/60 px-3 py-1 text-xs">
                        <span className="size-1.5 rounded-full bg-solana-green" />
                        <span className="text-muted-foreground">live demo · async_vault on devnet</span>
                    </div>
                    <h1 className="text-balance text-4xl font-semibold leading-tight md:text-6xl">
                        A standard, async{' '}
                        <span className="bg-gradient-to-r from-solana-purple via-fuchsia-400 to-solana-green bg-clip-text text-transparent">
                            tokenized vault
                        </span>{' '}
                        primitive for Solana.
                    </h1>
                    <p className="max-w-2xl text-pretty text-base text-muted-foreground md:text-lg">
                        Walk through the entire deposit → approve → claim lifecycle of the Solana Foundation{' '}
                        <code className="font-mono text-foreground">async_vault</code> program — including every
                        extension — without writing a single line of code.
                    </p>
                    <div className="flex flex-wrap items-center gap-3">
                        <Link href="/create">
                            <Button size="lg" variant="gradient">
                                Create a demo vault <ArrowRight className="size-4" />
                            </Button>
                        </Link>
                        <Link href="/vaults">
                            <Button size="lg" variant="outline">
                                Open existing vault
                            </Button>
                        </Link>
                    </div>
                </div>

                <Card className="self-center bg-card/60 backdrop-blur">
                    <CardContent className="space-y-4 p-6">
                        <p className="text-xs uppercase tracking-wide text-muted-foreground">
                            Suggested 90-second demo
                        </p>
                        <ol className="space-y-2.5 text-sm">
                            {SCRIPT.map((step, i) => (
                                <li key={step} className="flex items-start gap-3">
                                    <span className="mt-0.5 inline-flex size-6 shrink-0 items-center justify-center rounded-full bg-primary/15 text-xs font-semibold text-primary">
                                        {i + 1}
                                    </span>
                                    <span className="text-foreground/90">{step}</span>
                                </li>
                            ))}
                        </ol>
                    </CardContent>
                </Card>
            </section>

            <section className="space-y-6">
                <div className="flex items-end justify-between">
                    <div>
                        <p className="text-xs uppercase tracking-wide text-muted-foreground">What ships</p>
                        <h2 className="mt-1 text-3xl font-semibold">Every program feature, click-able.</h2>
                    </div>
                </div>
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                    {FEATURES.map(f => (
                        <Card key={f.title} className="bg-card/40">
                            <CardContent className="p-6">
                                <f.icon className="size-6 text-primary" />
                                <h3 className="mt-4 font-semibold">{f.title}</h3>
                                <p className="mt-2 text-sm text-muted-foreground">{f.body}</p>
                            </CardContent>
                        </Card>
                    ))}
                </div>
            </section>

            <section className="rounded-2xl border border-border bg-card/40 p-8 md:p-12">
                <div className="grid gap-8 md:grid-cols-2 md:gap-12">
                    <div>
                        <h3 className="text-2xl font-semibold">Reuse a shared, audited primitive.</h3>
                        <p className="mt-3 text-sm text-muted-foreground">
                            Real-world asset issuers and institutions repeatedly build the same vault primitives.
                            <code className="mx-1 font-mono text-foreground">async_vault</code> standardizes
                            subscription / redemption flows so teams can innovate on top instead of forking yet another
                            implementation.
                        </p>
                    </div>
                    <ul className="space-y-3 text-sm">
                        <FeatureRow label="Composable with sRFC-37" value="Token Access Control List for KYC'd RWAs" />
                        <FeatureRow label="No forced ATAs" value="Caller pre-initializes any token accounts" />
                        <FeatureRow label="Token-2022 ready" value="Asset & share mints can use either token program" />
                        <FeatureRow
                            label="LiteSVM tested"
                            value="Rust-only integration suite covers each instruction"
                        />
                    </ul>
                </div>
            </section>
        </div>
    );
}

function FeatureRow({ label, value }: { label: string; value: string }) {
    return (
        <li className="flex items-start justify-between gap-4 border-b border-border/60 pb-3 last:border-0 last:pb-0">
            <span className="text-foreground">{label}</span>
            <span className="text-right text-muted-foreground">{value}</span>
        </li>
    );
}
