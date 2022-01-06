#! /usr/env bash

set -o errtrace
set -o pipefail
set -o nounset
set -o errexit

PACKAGE="diff-against-bitte-commit"

function show_help () {
  echo "$PACKAGE - what happens if this project would use bitte commit XYZ"
  echo " "
  echo "$PACKAGE -c BITTE_COMMIT [-n NODE [-n NODE ] ... ] [-t TFTARGET [-t TFTARGET ] ... ]"
  echo ' '
  echo 'options:'
  echo '-h, --help                show help'
  echo '-c, --commit BITTE_COMMIT diffs against BITTE_COMMIT'
  echo '-n, --node NODE           add nodes to diff (default: core-1)'
  echo '-t, --terraform TFTARGET  add terraform targets to diff (default: hydrate-cluster)'
  echo ' '
  echo 'BITTE_COMMIT a commit rev from the bitte repository and the future'
  echo '             right-hand side of the comparision'
  echo ' '
  echo "nix-diff binary path: $(which nix-diff)"
  echo "nix version: $(nix --version)"
}

test $# -eq 0 && (show_help && exit 1)

BITTE_REPO="input-output-hk/bitte"
NODES=(core-1)
TFTARGET=(hydrate-cluster)

while test $# -gt 0; do
  case "$1" in
    -h|--help)
      show_help
      exit 0
      ;;
    -c|--commit)
      shift
      if test $# -gt 0; then
        export BITTE_COMMIT=$1
      else
        echo "no BITTE_COMMIT specified"
        exit 1
      fi
      shift
      ;;
    -n|--node)
      shift
      if test $# -gt 0; then
        NODES+=("$1")
      else
        echo "no NODE specified"
        exit 1
      fi
      shift
      ;;
    -t|--terraform)
      shift
      if test $# -gt 0; then
        TFTARGET+=("$1")
      else
        echo "no TFTARGET specified"
        exit 1
      fi
      shift
      ;;
    *)
      show_help
      exit 1
      ;;
  esac
done

swap_bitte=(--override-input bitte "github:$BITTE_REPO/$BITTE_COMMIT" --update-input bitte)

function cmd {
  local cmd
  cmd="$*"
  >&2 echo ">>> $cmd"

  if 2>/dev/null $cmd; then
    >&2 echo "$(tput setaf 2; tput bold)OK$(tput sgr 0)"
  else
    >&2 echo "$(tput setaf 1; tput bold)COMMAND FAILED$(tput sgr 0)" && exit 1
  fi
}

function diff {
  local attrpath="$1"
  local left
  local right

  echo
  echo "$(tput bold)------------ DIFFING ${attrpath} ...$(tput sgr 0)"

  left=$(cmd nix eval --raw "$PRJ_ROOT#$attrpath.drvPath")
  right=$(cmd nix eval "${swap_bitte[@]}" --raw "$PRJ_ROOT#$attrpath.drvPath")
  echo
  echo "$(tput setaf 1; tput bold)- left=$left$(tput sgr 0)"
  echo "$(tput setaf 2; tput bold)+ right=$right$(tput sgr 0)"
  echo

  echo "$(tput bold)------------ NIX-DIFF ------------$(tput sgr 0)"
  cmd nix-diff --character-oriented "$left" "$right"
  echo "$(tput bold)------------ DIFF-CLOSURES ------------$(tput sgr 0)"
  cmd nix store diff-closures  "$left" "$right"
  echo
  echo
}

for i in "${NODES[@]}"
do
  attrpath="nixosConfigurations.$BITTE_CLUSTER-$i.config.system.build.toplevel"
  diff "$attrpath"
done

for i in "${TFTARGET[@]}"
do
  attrpath="clusters.$BITTE_CLUSTER.tf.$i.config"
  diff "$attrpath"
done

