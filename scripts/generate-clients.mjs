import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-rust";
import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, "..");

// Load the Anchor IDL
const asyncIdlPath = join(projectRoot, "target/idl/async_vault.json");
const asyncIdl = JSON.parse(readFileSync(asyncIdlPath, "utf-8"));

// Create Codama tree from Anchor IDL
const asyncVaultCodamaTree = createFromRoot(rootNodeFromAnchor(asyncIdl));

// Generate Rust client
const asyncVaultClientPath = join(projectRoot, "clients/rust/async_vault/src/generated");

asyncVaultCodamaTree.accept(
  renderVisitor(asyncVaultClientPath, {
    crateFolder: join(projectRoot, "clients/rust"),
    formatCode: true,
    toolchain: "+nightly",
  })
);

console.log("Async Vault client generated successfully at:", asyncVaultClientPath);
