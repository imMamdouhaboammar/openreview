# OpenReview: Sandbox Execution Drivers

OpenReview executes arbitrary PR branch commands, formatters, test suites, and linter passes. Untrusted code submitted in external PRs must never run with unconstrained privileges. OpenReview supports two pluggable sandbox drivers: `docker` (OCI container) and `local` (isolated process).

---

## 1. Driver Comparison

| Feature | `docker` (Recommended for Multi-Tenant & Public Repos) | `local` (Recommended for Trusted CI & Local CLI) |
|:--------|:-------------------------------------------------------|:-------------------------------------------------|
| **Isolation Level** | High: Linux cgroups, namespaces, isolated network | Medium: Dedicated temp workspace directory |
| **Prerequisites** | Docker daemon or Podman running | Local host runtimes installed (Rust, Node, etc.) |
| **Resource Limits** | Hard memory (`2048MB`) & CPU (`2.0`) enforcement | Process nice levels, shell timeout command |
| **Startup Overhead** | ~1.5s container provision & attach | <10ms subprocess spawn |
| **Security Risk** | Minimal (contained inside ephemeral OCI container) | Potential host file access if malicious code escapes |

---

## 2. Docker Driver Configuration (`SANDBOX_DRIVER=docker`)

### How It Operates
1. OpenReview provisions an ephemeral Docker container using the configured image.
2. The PR repository is cloned into a temporary volume inside the container.
3. Network access is restricted or proxied.
4. Commands executed via the `bash` tool are executed via `docker exec`.
5. Upon session completion (or error), the container and temporary volume are immediately destroyed.

### Security Hardening Options
- `network_mode: "bridge"` (or `"none"` if offline tests only)
- `read_only_root_fs: false` (workspace volume writable, root system read-only)
- `security_opt: ["no-new-privileges:true"]`
- `pids_limit: 256` (prevents fork bombs)

---

## 3. Local Driver Configuration (`SANDBOX_DRIVER=local`)

### How It Operates
1. Creates an isolated temporary directory in `/tmp/openreview-<pr_id>-<uuid>/`.
2. Clones the PR branch using the temporary installation token.
3. Spawns commands via `std::process::Command` with working directory anchored to the temporary path.
4. Executes child processes under a non-root UID whenever available.
5. Recursively deletes the temporary directory in the teardown phase.

### When to Use Local Driver
- Dedicated ephemeral GitHub Actions runner VMs (where the VM itself is torn down after the job).
- Local developer workstation CLI testing (`openreview review --pr 12`).
- Air-gapped single-tenant development servers.
