import { Toaster as Sonner } from 'sonner';
import type { ToasterProps } from 'sonner';

function Toaster({ ...props }: ToasterProps) {
    return (
        <Sonner
            theme="light"
            position="bottom-right"
            richColors
            closeButton
            className="toaster group"
            style={
                {
                    '--normal-bg': 'var(--popover)',
                    '--normal-border': 'var(--border)',
                    '--normal-text': 'var(--popover-foreground)',
                } as React.CSSProperties
            }
            {...props}
        />
    );
}

export { Toaster };
