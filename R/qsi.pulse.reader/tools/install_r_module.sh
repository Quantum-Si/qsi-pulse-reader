#!/usr/bin/env bash
set -euo pipefail

# Installs the R module from a full repository checkout.
# Steps:
#  1. Copy the canonical rust-core crate (without target/) into the R src tree.
#  2. Run remotes::install_local (fallback to R CMD INSTALL if remotes missing).
#
# Usage (from repo root or any directory):
#   bash R/qsi.pulse.reader/tools/install_r_module.sh
#
# Optional env vars:
#   R_CMD (default: R)
#   FORCE (default: true)  -> passed to remotes::install_local(force=...)
#   KEEP_RUST_CORE (default: false) -> if 'true', do not remove staged copy after install
#   VERBOSE (default: false) -> if 'true', echo R install output (no -q)
#
# Exit codes:
#  0 success
#  >0 failure

R_CMD=${R_CMD:-R}
FORCE=${FORCE:-true}
KEEP_RUST_CORE=${KEEP_RUST_CORE:-false}
VERBOSE=${VERBOSE:-false}

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT_DIR=$(cd "${SCRIPT_DIR}/../../.." && pwd)
PKG_DIR="${ROOT_DIR}/R/qsi.pulse.reader"
RUST_SRC="${ROOT_DIR}/rust-core"
RUST_DST="${PKG_DIR}/src/rust/rust-core"

log(){ echo "[qsi-pulse-reader][install] $*" >&2; }

log "Root: ${ROOT_DIR}"

if [[ ! -d "${RUST_SRC}/src" ]]; then
  log "ERROR: Missing ${RUST_SRC}/src (ensure you cloned the full repo)."
  exit 2
fi

log "Staging rust-core -> ${RUST_DST}"
rm -rf "${RUST_DST}"
mkdir -p "${RUST_DST}"
cp -r "${RUST_SRC}/src" "${RUST_DST}/"
cp "${RUST_SRC}/Cargo.toml" "${RUST_DST}/"
[[ -f "${RUST_SRC}/Cargo.lock" ]] && cp "${RUST_SRC}/Cargo.lock" "${RUST_DST}/"
[[ -f "${RUST_SRC}/LICENSE.rst" ]] && cp "${RUST_SRC}/LICENSE.rst" "${RUST_DST}/"

R_FLAGS="-q"
if [[ ${VERBOSE} == "true" ]]; then
  R_FLAGS=""
fi

log "Installing R package (FORCE=${FORCE}, VERBOSE=${VERBOSE})"
INSTALL_EXPR="if(!requireNamespace('remotes', quietly=TRUE)) install.packages('remotes'); remotes::install_local('${PKG_DIR}', force=${FORCE})"

set +e
"${R_CMD}" ${R_FLAGS} -e "${INSTALL_EXPR}"
RC=$?
set -e

if [[ ${RC} -ne 0 ]]; then
  log "remotes::install_local failed (code ${RC}); falling back to R CMD INSTALL"
  (cd "${PKG_DIR}" && "${R_CMD}" CMD INSTALL .) || { log "Fallback install failed"; exit 3; }
fi

if [[ ${KEEP_RUST_CORE} != "true" ]]; then
  log "Cleaning staged rust-core (KEEP_RUST_CORE=true to retain)"
  rm -rf "${RUST_DST}"
else
  log "Leaving staged rust-core in place (KEEP_RUST_CORE=true)."
fi

log "Done."
