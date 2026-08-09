import { cp, lstat, mkdtemp, readdir, realpath, rename, rm } from "node:fs/promises";
import { basename, dirname, join, relative, resolve, sep } from "node:path";

const destination = resolve("static/pkg");
const artifact = process.env.WASM_SMOKE_PACKAGE;
const requiredFiles = ["package.json", "tool_call_trace_web.js", "tool_call_trace_web_bg.wasm"];

async function metadata(path) {
  try {
    return await lstat(path);
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

async function validateTree(root, current = root) {
  for (const entry of await readdir(current)) {
    const path = join(current, entry);
    const entryMetadata = await lstat(path);
    if (entryMetadata.isSymbolicLink()) {
      throw new Error(`WASM package contains a symbolic link: ${relative(root, path)}`);
    }
    if (entryMetadata.isDirectory()) {
      await validateTree(root, path);
    } else if (!entryMetadata.isFile()) {
      throw new Error(`WASM package contains a special file: ${relative(root, path)}`);
    }
  }
}

async function validatePackage(root) {
  const rootMetadata = await metadata(root);
  if (!rootMetadata?.isDirectory() || rootMetadata.isSymbolicLink()) {
    throw new Error("WASM package must be a real directory");
  }
  await validateTree(root);
  for (const requiredFile of requiredFiles) {
    const requiredMetadata = await metadata(join(root, requiredFile));
    if (!requiredMetadata?.isFile() || requiredMetadata.isSymbolicLink()) {
      throw new Error(`WASM package is missing ${requiredFile}`);
    }
  }
}

function containsPath(parent, child) {
  const path = relative(parent, child);
  return path === "" || (path !== ".." && !path.startsWith(`..${sep}`));
}

async function installPackage(staging, target) {
  const previous = `${staging}-previous`;
  const targetMetadata = await metadata(target);
  if (targetMetadata && (!targetMetadata.isDirectory() || targetMetadata.isSymbolicLink())) {
    throw new Error("static/pkg must be a real directory");
  }

  if (!targetMetadata) {
    await rename(staging, target);
    return;
  }

  await rename(target, previous);
  try {
    await rename(staging, target);
  } catch (error) {
    await rename(previous, target);
    throw error;
  }

  try {
    await rm(previous, { recursive: true });
  } catch (error) {
    try {
      await rename(target, staging);
      await rename(previous, target);
    } catch (rollbackError) {
      throw new AggregateError([error, rollbackError], "WASM package rollback failed");
    }
    throw error;
  }
}

if (!artifact) {
  await validatePackage(destination);
} else {
  const artifactMetadata = await metadata(artifact);
  if (!artifactMetadata?.isDirectory() || artifactMetadata.isSymbolicLink()) {
    throw new Error("WASM_SMOKE_PACKAGE must point to a real directory");
  }
  const source = await realpath(artifact);
  const targetParent = await realpath(dirname(destination));
  const canonicalDestination = join(targetParent, basename(destination));
  if (containsPath(source, canonicalDestination) || containsPath(canonicalDestination, source)) {
    if (source !== canonicalDestination) {
      throw new Error("WASM source and destination must not overlap");
    }
    await validatePackage(source);
  } else {
    await validatePackage(source);
    const staging = await mkdtemp(join(targetParent, `.${basename(destination)}.staging-`));
    try {
      for (const entry of await readdir(source)) {
        await cp(join(source, entry), join(staging, entry), {
          recursive: true,
          force: false,
          errorOnExist: true,
        });
      }
      await validatePackage(staging);
      await installPackage(staging, canonicalDestination);
    } finally {
      await rm(staging, { recursive: true, force: true });
    }
  }
}
