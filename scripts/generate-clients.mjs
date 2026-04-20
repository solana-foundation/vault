import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-rust";
import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, "..");

// Load the Anchor IDL
const idlVaultPath = join(projectRoot, "target/idl/vault.json");
const vaultIdl = JSON.parse(readFileSync(idlVaultPath, "utf-8"));

const asyncIdlPath = join(projectRoot, "target/idl/async_vault.json");
const asyncIdl = JSON.parse(readFileSync(asyncIdlPath, "utf-8"));

const hookIdlPath = join(projectRoot, "target/idl/hook_program.json");
const hookIdl = JSON.parse(readFileSync(hookIdlPath, "utf-8"));

const dummyIdlPath = join(projectRoot, "target/idl/dummy_protocol.json");
const dummyIdl = JSON.parse(readFileSync(dummyIdlPath, "utf-8"));

// Create Codama tree from Anchor IDL
const vaultCodamaTree = createFromRoot(rootNodeFromAnchor(vaultIdl));
const asyncVaultCodamaTree = createFromRoot(rootNodeFromAnchor(asyncIdl));
const hookCodamaTree = createFromRoot(rootNodeFromAnchor(hookIdl));
const dummyCodamaTree = createFromRoot(rootNodeFromAnchor(dummyIdl));

// Generate Rust client
const vaultClientPath = join(projectRoot, "clients/rust/vault/src/generated");
const asyncVaultClientPath = join(projectRoot, "clients/rust/async_vault/src/generated");
const hookRustClientPath = join(projectRoot, "clients/rust/hook/src/generated");
const dummyRustClientPath = join(projectRoot, "clients/rust/dummy/src/generated");


vaultCodamaTree.accept(
  renderVisitor(vaultClientPath, {
    crateFolder: join(projectRoot, "clients/rust"),
    formatCode: true,
    toolchain: "+nightly",
  })
);

asyncVaultCodamaTree.accept(
  renderVisitor(asyncVaultClientPath, {
    crateFolder: join(projectRoot, "clients/rust"),
    formatCode: true,
    toolchain: "+nightly",
  })
);

hookCodamaTree.accept(
  renderVisitor(hookRustClientPath, {
    crateFolder: join(projectRoot, "clients/rust"),
    formatCode: true,
    toolchain: "+nightly",
  })
);

dummyCodamaTree.accept(
  renderVisitor(dummyRustClientPath, {
    crateFolder: join(projectRoot, "clients/rust"),
    formatCode: true,
    toolchain: "+nightly",
  })
);

console.log("Vault client generated successfully at:", vaultClientPath);
console.log("Async Vault client generated successfully at:", asyncVaultClientPath);
console.log("Hook Rust client generated successfully at:", hookRustClientPath);
console.log("Dummy Rust client generated successfully at:", dummyRustClientPath);
