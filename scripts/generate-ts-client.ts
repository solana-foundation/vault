/**
 * Generates only the TypeScript client from the Anchor IDL.
 * Used by the web app build pipeline (no Rust toolchain required).
 */

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { AnchorIdl } from '@codama/nodes-from-anchor';
import { rootNodeFromAnchor } from '@codama/nodes-from-anchor';
import { renderVisitor as renderJavaScriptVisitor } from '@codama/renderers-js';
import { createFromRoot, deduplicateIdenticalDefinedTypesVisitor, updateDefinedTypesVisitor } from 'codama';

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

const idl = JSON.parse(readFileSync(join(projectRoot, 'idl/async_vault.json'), 'utf-8')) as AnchorIdl;
const codama = createFromRoot(rootNodeFromAnchor(idl));
codama.update(deduplicateIdenticalDefinedTypesVisitor());
codama.update(updateDefinedTypesVisitor({ RequestArgs: { name: 'CreateRequestArgs' } }));

const tsPackageFolder = join(projectRoot, 'clients/typescript');
void codama.accept(
    renderJavaScriptVisitor(tsPackageFolder, {
        kitImportStrategy: 'rootOnly',
        syncPackageJson: false,
    }),
);
console.log('TypeScript client generated at:', join(tsPackageFolder, 'src/generated'));
