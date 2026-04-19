#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

declare -rl PARAM1=${1-null}
# shellcheck disable=SC2155
declare -rl SCRIPT_NAME=$(basename "${0}")
# shellcheck disable=SC2155
declare -rl DIR_NAME=$(dirname "${0}")
# shellcheck disable=SC2155
declare -r CURDIR=$(pwd)
# shellcheck disable=SC2155
declare -r WHOAMI=$(whoami)

usage() {
  echo "${SCRIPT_NAME} usage:"
  echo ""
  echo "${SCRIPT_NAME} [-h|--help]"
  echo ""
  echo "-h:     Display this usage"
  echo "--help: Display this usage"
  echo ""
  echo "This script change group ownership and rights of the powercap"
  echo "special files. So you can run scaphandre as a regular user and"
  echo "access power data."
  exit 1
}

check_parameters() {
  if [[ ${PARAM1} == '-h' ]] || [[ ${PARAM1} == '--help' ]]; then
    usage
  fi
}

change_power_sf_group_owner_and_rights() {
  while IFS= read -r -d '' file; do
    sudo chown root:"${WHOAMI}" "${file}"
    sudo chmod g+r -R "${file}"
  done < <(find /sys/devices/virtual/powercap -name energy_uj -print0)
}

main() {
  cd "${DIR_NAME}"
  check_parameters
  change_power_sf_group_owner_and_rights
  cd "${CURDIR}"
}

main
