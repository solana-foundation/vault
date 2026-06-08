/**
 * Generates Rust and TypeScript clients from the Anchor IDL.
 */

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { AnchorIdl } from '@codama/nodes-from-anchor';
import { rootNodeFromAnchor } from '@codama/nodes-from-anchor';
import { renderVisitor as renderJavaScriptVisitor } from '@codama/renderers-js';
import { renderVisitor as renderRustVisitor } from '@codama/renderers-rust';
import { createFromRoot, deduplicateIdenticalDefinedTypesVisitor, updateDefinedTypesVisitor } from 'codama';

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

const idl = JSON.parse(readFileSync(join(projectRoot, 'idl/async_vault.json'), 'utf-8')) as AnchorIdl;
const codama = createFromRoot(rootNodeFromAnchor(idl));
codama.update(deduplicateIdenticalDefinedTypesVisitor());

const rustCrateFolder = join(projectRoot, 'clients/rust/async_vault');
codama.accept(
    renderRustVisitor(rustCrateFolder, {
        formatCode: false,
        syncCargoToml: false,
    }),
);
execFileSync('cargo', ['+nightly', 'fmt', '-p', 'async-vault-client'], { cwd: projectRoot, stdio: 'inherit' });
console.log('Rust client generated at:', join(rustCrateFolder, 'src/generated'));

codama.update(updateDefinedTypesVisitor({ RequestArgs: { name: 'CreateRequestArgs' } }));

const tsPackageFolder = join(projectRoot, 'clients/typescript');
void codama.accept(
    renderJavaScriptVisitor(tsPackageFolder, {
        kitImportStrategy: 'rootOnly',
        syncPackageJson: false,
    }),
);
console.log('TypeScript client generated at:', join(tsPackageFolder, 'src/generated'));
