'use client';

import * as React from 'react';
import { Check, Copy } from 'lucide-react';

import { cn } from '@/lib/cn';

export function CopyButton({ value, className }: { value: string; className?: string }) {
    const [copied, setCopied] = React.useState(false);
    const handle = () => {
        if (typeof navigator === 'undefined') return;
        void navigator.clipboard.writeText(value);
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
    };
    return (
        <button
            type="button"
            onClick={handle}
            className={cn(
                'inline-flex items-center justify-center rounded-md p-1 text-muted-foreground transition hover:bg-accent hover:text-accent-foreground',
                className,
            )}
            aria-label="Copy to clipboard"
        >
            {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
        </button>
    );
}
