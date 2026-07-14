#!/bin/sh
set -eu

: "${SIGNOZ_API_KEY:?SIGNOZ_API_KEY must be set}"
: "${SIGNOZ_ENDPOINT:=http://127.0.0.1:8080}"
SIGNOZ_ENDPOINT="${SIGNOZ_ENDPOINT%/}"

dashboard=/dashboards/equipment-reservation-observability.json
name=equipment-reservation-observability

if [ ! -r "$dashboard" ]; then
  echo "Dashboard definition is not readable: $dashboard" >&2
  exit 1
fi

# The SigNoz API returns the list as one JSON line. The definition's internal
# name is fixed, so this extracts the ID of its existing entry for an idempotent
# PUT. The only writable state is the dashboard record inside SigNoz.
existing="$(curl --fail-with-body --silent --show-error \
  -H "SigNoz-Api-Key: $SIGNOZ_API_KEY" \
  "$SIGNOZ_ENDPOINT/api/v2/dashboards?limit=200")"
dashboard_id="$(
  printf '%s' "$existing" |
    sed -n 's/.*"id":"\([^"]*\)".*"name":"'"$name"'".*/\1/p'
)"

if [ -n "$dashboard_id" ]; then
  method=PUT
  url="$SIGNOZ_ENDPOINT/api/v2/dashboards/$dashboard_id"
  echo "Updating SigNoz dashboard $name ($dashboard_id)"
else
  method=POST
  url="$SIGNOZ_ENDPOINT/api/v2/dashboards"
  echo "Creating SigNoz dashboard $name"
fi

response=/tmp/dashboard-sync-response.json
curl --fail-with-body --silent --show-error \
  -X "$method" \
  -H "SigNoz-Api-Key: $SIGNOZ_API_KEY" \
  -H 'Content-Type: application/json' \
  --data-binary "@$dashboard" \
  -o "$response" \
  "$url"

result_id="$(sed -n 's/.*"data":{"id":"\([^"]*\)".*/\1/p' "$response")"
if [ -z "$result_id" ]; then
  echo "SigNoz returned no dashboard id" >&2
  cat "$response" >&2
  exit 1
fi

echo "Dashboard synced: $SIGNOZ_ENDPOINT/dashboard/$result_id"
