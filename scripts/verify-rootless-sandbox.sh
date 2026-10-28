#!/usr/bin/env bash
# Operator-run security smoke test for the actual rootless Docker execution host.
# Not a substitute for a complete live GitHub App integration test.
set -euo pipefail

: "${SANDBOX_WORKSPACE_DIR:?Set the absolute shared workspace directory used by OpenReview}"
[[ "${SANDBOX_WORKSPACE_DIR}" == /* ]] || { echo 'Workspace must be absolute' >&2; exit 2; }
[[ "${SANDBOX_WORKSPACE_DIR}" != '/' ]] || { echo 'Workspace must not be filesystem root' >&2; exit 2; }
[[ -d "${SANDBOX_WORKSPACE_DIR}" ]] || { echo 'Workspace does not exist' >&2; exit 2; }
command -v docker >/dev/null 2>&1 || { echo 'Docker CLI is unavailable' >&2; exit 2; }

docker info --format '{{json .SecurityOptions}}' | grep -q 'rootless' || {
  echo 'Refusing to test against a rootful Docker daemon' >&2
  exit 2
}
[[ "$(docker info --format '{{.CgroupDriver}}/{{.CgroupVersion}}')" == 'systemd/2' ]] || {
  echo 'Rootless Docker has no enforceable cgroup v2/systemd resource policy' >&2
  exit 2
}
docker image inspect openreview-sandbox:latest >/dev/null

scratch_dir="$(mktemp -d "${SANDBOX_WORKSPACE_DIR%/}/openreview-smoke-XXXXXXXX")"
review_container="openreview-review-smoke-$$"
fix_container="openreview-fix-smoke-$$"
cleanup() {
  docker rm -f "${review_container}" "${fix_container}" >/dev/null 2>&1 || true
  if [[ -d "${scratch_dir}" && "${scratch_dir}" == "${SANDBOX_WORKSPACE_DIR%/}"/openreview-smoke-* ]]; then
    rm -r -- "${scratch_dir}"
  fi
}
trap cleanup EXIT INT TERM

printf 'original' > "${scratch_dir}/source.txt"
run_container() {
  local name="$1" mount_mode="$2"
  local mount="type=bind,source=${scratch_dir},target=/workspace"
  [[ "${mount_mode}" == 'review' ]] && mount+=',readonly'
  docker run -d --rm --name "${name}" \
    --memory=2g --cpus=2 --pids-limit=100 \
    --cap-drop=ALL --security-opt=no-new-privileges:true \
    --network=none --read-only --tmpfs=/tmp:rw,nosuid,size=1g \
    --user=0:0 --mount "${mount}" -w /workspace \
    openreview-sandbox:latest sleep 120 >/dev/null
}
assert_cgroups() {
  local name="$1" memory pids cpu quota period
  memory="$(docker exec "${name}" cat /sys/fs/cgroup/memory.max)"
  pids="$(docker exec "${name}" cat /sys/fs/cgroup/pids.max)"
  cpu="$(docker exec "${name}" cat /sys/fs/cgroup/cpu.max)"
  [[ "${memory}" =~ ^[0-9]+$ ]] && (( memory > 0 && memory <= 2147483648 )) || return 1
  [[ "${pids}" =~ ^[0-9]+$ ]] && (( pids > 0 && pids <= 100 )) || return 1
  read -r quota period <<< "${cpu}"
  [[ "${quota}" =~ ^[0-9]+$ && "${period}" =~ ^[0-9]+$ ]] || return 1
  (( quota > 0 && period > 0 && quota <= 2 * period ))
}

run_container "${review_container}" review
[[ "$(docker exec "${review_container}" cat /workspace/source.txt)" == 'original' ]]
if docker exec "${review_container}" sh -c 'echo changed > /workspace/source.txt' >/dev/null 2>&1; then
  echo 'FAIL: Review workspace was writable' >&2
  exit 1
fi
[[ "$(<"${scratch_dir}/source.txt")" == 'original' ]]
assert_cgroups "${review_container}" || { echo 'Review cgroup limits ineffective' >&2; exit 1; }

run_container "${fix_container}" fix
docker exec "${fix_container}" sh -c 'printf verified > /workspace/fix.txt'
[[ "$(<"${scratch_dir}/fix.txt")" == 'verified' ]]
assert_cgroups "${fix_container}" || { echo 'Fix cgroup limits ineffective' >&2; exit 1; }

# No host app secret is ever mounted or handed to the sandbox's environment.
if docker exec "${fix_container}" printenv | grep -Eq '^(GITHUB_APP_PRIVATE_KEY|GITHUB_TOKEN|OPENREVIEW_GIT_TOKEN|JOB_API_TOKEN)='; then
  echo 'FAIL: App credential environment leaked to sandbox' >&2
  exit 1
fi
if docker exec "${fix_container}" curl --max-time 3 -fsS https://github.com >/dev/null 2>&1; then
  echo 'FAIL: Sandbox unexpectedly has external network access' >&2
  exit 1
fi

docker rm -f "${review_container}" "${fix_container}" >/dev/null
[[ -z "$(docker ps -a -q --filter "name=^/${review_container}$")" ]]
[[ -z "$(docker ps -a -q --filter "name=^/${fix_container}$")" ]]
echo 'PASS: rootless daemon, enforced cgroups, Review read-only, Fix write, no injected tokens/network, container cleanup'
