'use client';

import * as React from 'react';
import { ExternalLink } from 'lucide-react';

import { explorerLink } from '@/lib/env';
import { shortAddress } from '@/lib/format';
import { cn } from '@/lib/cn';

import { CopyButton } from './copy-button';

export function AddressPill({
    value,
    kind = 'address',
    className,
    label,
    chars,
}: {
    value: string;
    kind?: 'address' | 'tx';
    className?: string;
    label?: string;
    chars?: number;
}) {
    if (!value) return null;
    return (
        <span
            className={cn(
                'inline-flex items-center gap-1 rounded-md border border-border bg-muted/40 px-2 py-1 text-xs font-mono',
                className,
            )}
        >
            {label && <span className="text-muted-foreground pr-1">{label}</span>}
            <span title={value}>{shortAddress(value, chars)}</span>
            <CopyButton value={value} />
            <a
                href={explorerLink(value, kind)}
                target="_blank"
                rel="noreferrer"
                className="text-muted-foreground transition hover:text-foreground"
                aria-label="Open in explorer"
            >
                <ExternalLink className="size-3.5" />
            </a>
        </span>
    );
}
