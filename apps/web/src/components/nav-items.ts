import { Home, PlusCircle, Vault } from 'lucide-react';
import { type LucideIcon } from 'lucide-react';

export interface NavItem {
    icon: LucideIcon;
    label: string;
    path: string;
}

export const NAV_ITEMS: NavItem[] = [
    { icon: Home, label: 'Home', path: '/' },
    { icon: PlusCircle, label: 'Create vault', path: '/create' },
    { icon: Vault, label: 'My vaults', path: '/vaults' },
];
