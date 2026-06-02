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

const idl = JSON.parse(readFileSync(join(projectRoot, 'target/idl/async_vault.json'), 'utf-8')) as AnchorIdl;
const codama = createFromRoot(rootNodeFromAnchor(idl));
codama.update(deduplicateIdenticalDefinedTypesVisitor());

const rustClientPath = join(projectRoot, 'clients/rust/async_vault/src/generated');
codama.accept(
    renderRustVisitor(rustClientPath, {
        crateFolder: join(projectRoot, 'clients/rust'),
        formatCode: false,
    }),
);
execFileSync('cargo', ['+nightly', 'fmt', '-p', 'async-vault-client'], { cwd: projectRoot, stdio: 'inherit' });
console.log('Rust client generated at:', rustClientPath);

codama.update(updateDefinedTypesVisitor({ RequestArgs: { name: 'CreateRequestArgs' } }));

const jsClientPath = join(projectRoot, 'clients/typescript/src/generated');
void codama.accept(renderJavaScriptVisitor(jsClientPath));
console.log('TypeScript client generated at:', jsClientPath);
