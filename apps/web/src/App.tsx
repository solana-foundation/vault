import { Analytics } from '@vercel/analytics/react';
import { Navigate, Route, Routes } from 'react-router';

import { AppLayout } from '@/components/app-layout';
import { AppProviders } from '@/components/app-providers';
import { CreateVaultRoute } from '@/routes/create-vault';
import { Home } from '@/routes/home';
import { VaultDetailRoute } from '@/routes/vault-detail';
import { VaultsRoute } from '@/routes/vaults';

export function App() {
    return (
        <AppProviders>
            <Routes>
                <Route
                    path="/"
                    element={
                        <AppLayout>
                            <Home />
                        </AppLayout>
                    }
                />
                <Route
                    path="/create"
                    element={
                        <AppLayout>
                            <CreateVaultRoute />
                        </AppLayout>
                    }
                />
                <Route
                    path="/vaults"
                    element={
                        <AppLayout>
                            <VaultsRoute />
                        </AppLayout>
                    }
                />
                <Route
                    path="/vault/:shareMint"
                    element={
                        <AppLayout>
                            <VaultDetailRoute />
                        </AppLayout>
                    }
                />
                <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
            <Analytics />
        </AppProviders>
    );
}
