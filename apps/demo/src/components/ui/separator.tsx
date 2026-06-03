import * as React from 'react';
import { cn } from '@/lib/cn';

export function Separator({
    orientation = 'horizontal',
    className,
    ...props
}: React.HTMLAttributes<HTMLDivElement> & { orientation?: 'horizontal' | 'vertical' }) {
    return (
        <div
            role="separator"
            className={cn(
                'shrink-0 bg-border',
                orientation === 'horizontal' ? 'h-px w-full' : 'h-full w-px',
                className,
            )}
            {...props}
        />
    );
}
