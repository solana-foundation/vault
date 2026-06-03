'use client';

import * as React from 'react';

import { cn } from '@/lib/cn';

interface TabsContextValue {
    value: string;
    setValue: (v: string) => void;
}
const TabsContext = React.createContext<TabsContextValue | null>(null);

export function Tabs({
    value,
    onValueChange,
    defaultValue,
    children,
    className,
}: {
    value?: string;
    onValueChange?: (v: string) => void;
    defaultValue?: string;
    children: React.ReactNode;
    className?: string;
}) {
    const [internal, setInternal] = React.useState(defaultValue ?? '');
    const current = value ?? internal;
    const setValue = (v: string) => {
        if (onValueChange) onValueChange(v);
        else setInternal(v);
    };
    return (
        <TabsContext.Provider value={{ value: current, setValue }}>
            <div className={className}>{children}</div>
        </TabsContext.Provider>
    );
}

export function TabsList({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            className={cn(
                'inline-flex h-10 items-center justify-center rounded-md bg-muted p-1 text-muted-foreground',
                className,
            )}
            {...props}
        />
    );
}

export function TabsTrigger({
    value,
    children,
    className,
}: {
    value: string;
    children: React.ReactNode;
    className?: string;
}) {
    const ctx = React.useContext(TabsContext)!;
    const active = ctx.value === value;
    return (
        <button
            type="button"
            onClick={() => ctx.setValue(value)}
            data-state={active ? 'active' : 'inactive'}
            className={cn(
                'inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50',
                active ? 'bg-background text-foreground shadow-sm' : '',
                className,
            )}
        >
            {children}
        </button>
    );
}

export function TabsContent({
    value,
    children,
    className,
}: {
    value: string;
    children: React.ReactNode;
    className?: string;
}) {
    const ctx = React.useContext(TabsContext)!;
    if (ctx.value !== value) return null;
    return (
        <div
            className={cn(
                'mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 animate-fade-in',
                className,
            )}
        >
            {children}
        </div>
    );
}
