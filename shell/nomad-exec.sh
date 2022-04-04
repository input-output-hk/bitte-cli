#!/run/current-system/sw/bin/bash
set -euo pipefail

PACKAGE="nomad-exec"

while getopts 'c:u:n:j:t:a:e:psvh' c; do
  case "$c" in
  c) CONSUL_HTTP_ADDR="$OPTARG" ;;
  u) NOMAD_ADDR="$OPTARG" ;;
  n)
    NOMAD_NAMESPACE="$OPTARG"
    NOMAD_NAMESPACE_DECL="true"
    ;;
  j) JOB="$OPTARG" ;;
  t) TASK="$OPTARG" ;;
  a) ALLOC="$OPTARG" ;;
  e) CMD="$OPTARG" ;;
  p) PROMPT="true" ;;
  s) SHOW_TAGS="true" ;;
  v) VERBOSE="true" ;;
  *)
    echo "This command makes exec into a nomad allocation easier."
    echo "usage: $PACKAGE [-c CONSUL_HTTP_ADDR] [-u NOMAD_ADDR] [-n NOMAD_NAMESPACE] [-j JOB] [-t TASK] [-a ALLOC] [-e CMD] [-p] [-s] [-v] [-h]"
    echo
    echo "  -c  CONSUL_HTTP_ADDR for consul upstream server.  Default: env CONSUL_HTTP_ADDR (${CONSUL_HTTP_ADDR:-"UNSET"})"
    echo "  -u  NOMAD_ADDR for nomad upstream server.  Default: env NOMAD_ADDR (${NOMAD_ADDR:-"UNSET"})"
    echo "  -n  NOMAD_NAMESPACE to select the allocation from.  Default: env NOMAD_NAMESPACE (${NOMAD_NAMESPACE:-"UNSET"})"
    echo "  -j  JOB to select the allocation from"
    echo "  -t  TASK to select the allocation from"
    echo "  -a  ALLOCATION for exec"
    echo '  -e  CMD for exec. Default: "/bin/bash"'
    echo "  -p  prompt for some defaults if not declared as options (NOMAD_NAMESPACE, CMD)"
    echo "  -s  show consul service allocation associated addresses and tags (requires services to be tagged with the NOMAD_ALLOC_ID)"
    echo "  -v  verbose"
    echo "  -h  help"
    echo
    echo "$PACKAGE expects NOMAD_TOKEN and CONSUL_HTTP_TOKEN to be available in the environment for issuing cli and api calls."
    exit 0
    ;;
  esac
done

function choose {
  CHOICE=""
  OBJECT="$1"
  STRIP="$2"
  STRINGCMD="$3"

  echo "Choose a $OBJECT:"
  if [ "$STRIP" == "strip" ]; then
    mapfile -t ARRAY < <(eval "$STRINGCMD")
  else
    mapfile ARRAY < <(eval "$STRINGCMD")
  fi
  for i in "${!ARRAY[@]}"; do
    printf "%s\t%s" "$((i + 1))" "${ARRAY[$i]}"
  done
  while true; do
    read -r -p 'Selection (q to quit): ' REPLY
    if [[ $REPLY =~ ^[0-9]+$ ]] && [ "$REPLY" -ge 1 ] && [ "$REPLY" -le ${#ARRAY[@]} ]; then
      CHOICE="$(echo "${ARRAY[$((REPLY - 1))]}" | tr -d '\n')"
      echo
      break
    elif [ "$REPLY" == "q" ]; then
      exit 0
    else
      echo "Selection must be between 1 and ${#ARRAY[@]} or \"q\" to quit.  Please try again."
    fi
  done
}

if [ -z "${CONSUL_HTTP_TOKEN:-}" ]; then
  echo "CONSUL_HTTP_TOKEN is unset.  Please set the token and try again."
  exit 1
fi
[ -z "${VERBOSE:-}" ] || echo "CONSUL_HTTP_TOKEN is set"

if [ -z "${CONSUL_HTTP_ADDR:-}" ]; then
  echo "CONSUL_HTTP_ADDR is unset."
  echo 'Please set either CONSUL_HTTP_ADDR in the environment or specify the "-c" option and try again.'
  exit 1
fi
[ -z "${VERBOSE:-}" ] || echo "CONSUL_HTTP_ADDR is: $CONSUL_HTTP_ADDR"

if [ -z "${NOMAD_TOKEN:-}" ]; then
  echo "NOMAD_TOKEN is unset.  Please set the token and try again."
  exit 1
fi
[ -z "${VERBOSE:-}" ] || echo "NOMAD_TOKEN is set"

if [ -z "${NOMAD_ADDR:-}" ]; then
  echo "NOMAD_ADDR is unset."
  echo 'Please set either NOMAD_ADDR in the environment or specify the "-u" option and try again.'
  exit 1
fi
[ -z "${VERBOSE:-}" ] || echo "NOMAD_ADDR is: $NOMAD_ADDR"

if [ "${PROMPT:-}" == "true" ] && [ -z "${NOMAD_NAMESPACE_DECL:-}" ] || [ -z "${NOMAD_NAMESPACE:-}" ] && [ -z "${NOMAD_NAMESPACE_DECL:-}" ]; then
  choose \
    "namespace" \
    "nostrip" \
    "nomad namespace list --json | jq -r 'map(.Name) | unique | sort[]'"
  NOMAD_NAMESPACE="$CHOICE"
fi
[ -z "${VERBOSE:-}" ] || echo "NOMAD_NAMESPACE is: $NOMAD_NAMESPACE"

if [ -z "${JOB:-}" ]; then
  choose \
    "job" \
    "nostrip" \
    "curl -s -H \"X-Nomad-Token: $NOMAD_TOKEN\" \"$NOMAD_ADDR/v1/jobs?namespace=$NOMAD_NAMESPACE\" | jq -r 'map(.ID) | unique | sort[]'"
  JOB="$CHOICE"
fi
[ -z "${VERBOSE:-}" ] || echo "JOB is: $JOB"

if [ -z "${TASK:-}" ]; then
  choose \
    "running task" \
    "nostrip" \
    "curl -s -H \"X-Nomad-Token: $NOMAD_TOKEN\" \"$NOMAD_ADDR/v1/job/$JOB/allocations?namespace=$NOMAD_NAMESPACE\" | jq -r 'map(.TaskStates | select(any(.State == \"running\")) | keys[]) | unique | sort[]'"
  TASK="$CHOICE"
fi
[ -z "${VERBOSE:-}" ] || echo "TASK is: $TASK"

if [ -z "${ALLOC:-}" ]; then
  mapfile -t SHOW_ALLOCS < <(curl -s -H "X-Nomad-Token: $NOMAD_TOKEN" "$NOMAD_ADDR/v1/job/$JOB/allocations?namespace=$NOMAD_NAMESPACE" | jq -r "map(select(.TaskStates | keys | select(index(\"$TASK\")))) | map(select(.TaskStates.vector.State == \"running\")) | map(.ID) | unique | sort[]")
  if [ "${SHOW_TAGS:-}" == "true" ]; then
    SERVICE_CATALOG="$(curl -s -H "X-Consul-Token: $CONSUL_HTTP_TOKEN" "$CONSUL_HTTP_ADDR/v1/catalog/services")"
    COUNT="1"
    CACHE_SERVICE=()
    CACHE_SERVICE_JSON=()
    for SHOW_ALLOC in "${SHOW_ALLOCS[@]}"; do
      echo "Tags for alloc $COUNT: $SHOW_ALLOC"
      COUNT="$((COUNT + 1))"
      SERVICES="$(jq -r "[[map(.) | to_entries | .[] | select(.value[] | contains(\"$SHOW_ALLOC\")).key][] as \$i | keys[\$i]] | unique | sort[]" <<<"$SERVICE_CATALOG")"
      mapfile -t SERVICES_ALLOC_TAGGED < <(echo -n "$SERVICES")
      if [ "${#SERVICES_ALLOC_TAGGED[@]}" == "0" ]; then
        echo "  Consul service marker tag NOMAD_ALLOC_ID not found"
      else
        echo
      fi
      for SERVICE in $SERVICES; do
        CACHED="false"
        for INDEX in "${!CACHE_SERVICE[@]}"; do
          if [ "${CACHE_SERVICE[$INDEX]:-}" == "$SERVICE" ]; then
            SERVICE_JSON="${CACHE_SERVICE_JSON[$INDEX]}"
            CACHED="true"
            break
          fi
        done
        if [ "$CACHED" == "false" ]; then
          CACHE_SERVICE+=("$SERVICE")
          SERVICE_JSON="$(curl -s -H "X-Consul-Token: $CONSUL_HTTP_TOKEN" "$CONSUL_HTTP_ADDR/v1/catalog/service/$SERVICE")"
          CACHE_SERVICE_JSON+=("$SERVICE_JSON")
        fi
        SERVICE_SPEC="$(jq -r "map(select(.ServiceTags[] == \"$SHOW_ALLOC\"))[] | .ServiceName + \" (\" + .Address + \":\" + (.ServicePort | tostring) + \")\"" <<<"$SERVICE_JSON")"
        echo "  Alloc Associated Service: $SERVICE_SPEC"
        TAGS="$(jq -r "map(select(.ServiceTags[] == \"$SHOW_ALLOC\"))[] | .ServiceTags | del(.[] | select(. == \"$SHOW_ALLOC\")) | unique | sort[]" <<<"$SERVICE_JSON")"
        for TAG in $TAGS; do
          echo "    $SERVICE:$TAG"
        done
        echo
      done
      echo
    done
  fi
  choose \
    "allocation" \
    "nostrip" \
    "for SHOW_ALLOC in ${SHOW_ALLOCS[*]}; do echo \$SHOW_ALLOC; done"
  ALLOC="$CHOICE"
fi
[ -z "${VERBOSE:-}" ] || echo "ALLOC is: $ALLOC"

if [ "${PROMPT:-}" == "true" ] && [ -z "${CMD:-}" ]; then
  echo -n "Type in the CMD to exec (enter for default: /bin/bash): "
  read -r CMD
  if [ -z "${CMD:-}" ]; then
    CMD="/bin/bash"
  fi
elif [ -z "${CMD:-}" ]; then
  CMD="/bin/bash"
fi
[ -z "${VERBOSE:-}" ] || echo "CMD is: $CMD"

[ -z "${VERBOSE:-}" ] || {
  echo "EXEC CMD is:"
  echo "nomad exec -namespace \"$NOMAD_NAMESPACE\" -task \"$TASK\" \"$ALLOC\" \"$CMD\""
}
nomad exec -namespace "$NOMAD_NAMESPACE" -task "$TASK" "$ALLOC" "$CMD"

exit 0
