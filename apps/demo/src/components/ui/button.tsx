import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/cn';

const buttonVariants = cva(
    'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 [&_svg]:size-4 [&_svg]:shrink-0',
    {
        variants: {
            variant: {
                default: 'bg-primary text-primary-foreground hover:bg-primary/90',
                destructive: 'bg-destructive text-destructive-foreground hover:bg-destructive/90',
                outline:
                    'border border-border bg-background hover:bg-accent hover:text-accent-foreground',
                secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
                ghost: 'hover:bg-accent hover:text-accent-foreground',
                link: 'text-primary underline-offset-4 hover:underline',
                gradient:
                    'bg-gradient-to-r from-solana-purple to-solana-green text-white hover:opacity-90',
            },
            size: {
                default: 'h-10 px-4 py-2',
                sm: 'h-9 rounded-md px-3 text-xs',
                lg: 'h-11 rounded-md px-8',
                icon: 'h-10 w-10',
            },
        },
        defaultVariants: {
            variant: 'default',
            size: 'default',
        },
    },
);

export interface ButtonProps
    extends React.ButtonHTMLAttributes<HTMLButtonElement>,
        VariantProps<typeof buttonVariants> {
    loading?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
    ({ className, variant, size, loading, disabled, children, ...props }, ref) => {
        return (
            <button
                className={cn(buttonVariants({ variant, size, className }))}
                ref={ref}
                disabled={disabled || loading}
                {...props}
            >
                {loading ? (
                    <svg
                        className="size-4 animate-spin"
                        viewBox="0 0 24 24"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <circle
                            cx="12"
                            cy="12"
                            r="10"
                            stroke="currentColor"
                            strokeWidth="3"
                            opacity="0.25"
                        />
                        <path
                            fill="currentColor"
                            d="M12 2a10 10 0 0 1 10 10h-3a7 7 0 0 0-7-7V2Z"
                        />
                    </svg>
                ) : null}
                {children}
            </button>
        );
    },
);
Button.displayName = 'Button';
