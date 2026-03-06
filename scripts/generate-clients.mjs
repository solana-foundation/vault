import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-rust";
import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, "..");

// Load the Anchor IDL
const idlPath = join(projectRoot, "target/idl/vault.json");
const idl = JSON.parse(readFileSync(idlPath, "utf-8"));

const hookidlPath = join(projectRoot, "target/idl/hook_program.json");
const hookIdl = JSON.parse(readFileSync(hookidlPath, "utf-8"));

// Create Codama tree from Anchor IDL
const codama = createFromRoot(rootNodeFromAnchor(idl));
const hookCodama = createFromRoot(rootNodeFromAnchor(hookIdl));

// Generate Rust client
const rustClientPath = join(projectRoot, "clients/rust/vault/src/generated");
const hookRustClientPath = join(projectRoot, "clients/rust/hook/src/generated");


codama.accept(
  renderVisitor(rustClientPath, {
    crateFolder: join(projectRoot, "clients/rust"),
    formatCode: true,
    toolchain: "+nightly",
  })
);

hookCodama.accept(
  renderVisitor(hookRustClientPath, {
    crateFolder: join(projectRoot, "clients/rust"),
    formatCode: true,
    toolchain: "+nightly",
  })
);

console.log("Rust client generated successfully at:", rustClientPath);
