import { Toaster } from './ui/sonner';
import { AppFooter } from './app-footer';
import { AppHeader } from './app-header';

export function AppLayout({ children }: { children: React.ReactNode }) {
    return (
        <div className="flex min-h-dvh flex-col">
            <AppHeader />
            <main className="mx-auto w-full max-w-7xl flex-1 px-6 pt-24 pb-12">{children}</main>
            <AppFooter />
            <Toaster />
        </div>
    );
}
