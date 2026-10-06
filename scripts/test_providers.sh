#!/usr/bin/env bash
set -euo pipefail

REPORT_FILE="docs/PROVIDER_STATUS.md"
DATE=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
TMP_FILE="t_probe.txt"

echo "probe-test-$(date +%s)" > "$TMP_FILE"

cat <<EOF > "$REPORT_FILE"
# Provider Status Report

Automated health check conducted on: **${DATE}**

| Provider | Status | HTTP | Sample URL / Response | Download | Notes |
|---|---|---|---|---|---|
EOF

test_provider() {
  local name="$1"
  local enabled="$2"
  local cmd="$3"
  local json_field="$4"
  local notes="$5"

  echo "Testing ${name}..."
  local body=""
  local http_code=""
  local url=""
  local dl_status="N/A"
  local status="FAIL"

  local response_file
  response_file=$(mktemp)
  
  if http_code=$(eval "${cmd} -w '%{http_code}' -o '${response_file}' --max-time 15" 2>/dev/null); then
    body=$(head -c 200 "${response_file}" | tr -d '\r\n')
  else
    http_code="ERR"
    body="Connection failed / timed out"
  fi

  if [[ "${http_code}" =~ ^2 ]]; then
    if [[ -n "${json_field}" ]]; then
      if command -v jq >/dev/null 2>&1; then
        url=$(jq -r "${json_field} // empty" "${response_file}" 2>/dev/null || true)
      fi
    else
      if [[ "${body}" =~ ^https?:// ]]; then
        url="${body}"
      fi
    fi

    if [[ -n "${url}" && "${url}" =~ ^https?:// ]]; then
      if curl -s -f -I -L --max-time 10 "${url}" >/dev/null 2>&1; then
        dl_status="PASS"
        status="PASS"
      else
        dl_status="FAIL"
        status="PARTIAL"
      fi
    elif [[ "${name}" == "zerox" && "${body}" =~ "uploads disabled" ]]; then
      status="DISABLED"
      url="${body}"
    elif [[ -n "${body}" ]]; then
      url="${body}"
    fi
  else
    if [[ -z "${body}" ]]; then
      body="HTTP ${http_code}"
    fi
    url="${body}"
  fi

  rm -f "${response_file}"

  # Sanitize pipe chars for markdown table
  local clean_url
  clean_url=$(echo "${url}" | sed 's/|/\\|/g' | cut -c 1-120)
  local clean_notes
  clean_notes=$(echo "${notes}" | sed 's/|/\\|/g')

  echo "| \`${name}\` | **${status}** | ${http_code} | \`${clean_url}\` | ${dl_status} | ${clean_notes} |" >> "$REPORT_FILE"
}

# 1. catbox
test_provider "catbox" "Yes" \
  "curl -s -S -F reqtype=fileupload -F fileToUpload=@${TMP_FILE} https://catbox.moe/user/api.php" \
  "" \
  "Default host. ~200 MB max."

# 2. litterbox
test_provider "litterbox" "Yes" \
  "curl -s -S -F reqtype=fileupload -F time=1h -F fileToUpload=@${TMP_FILE} https://litterbox.catbox.moe/resources/internals/api.php" \
  "" \
  "Temporary files (1h, 12h, 24h, 72h). 1 GB max."

# 3. tempsh
test_provider "tempsh" "Yes" \
  "curl -s -S -F file=@${TMP_FILE} https://temp.sh/upload" \
  "" \
  "3 days retention, 4 GB limit."

# 4. gofile
test_provider "gofile" "Yes" \
  "curl -s -S -F file=@${TMP_FILE} https://upload.gofile.io/uploadfile" \
  ".data.downloadPage" \
  "Unlimited size, saved guestToken reused."

# 5. tmpfiles
test_provider "tmpfiles" "Yes" \
  "curl -s -S -F file=@${TMP_FILE};filename=probe_upload.txt https://tmpfiles.org/api/v1/upload" \
  ".data.url" \
  "100 MB max, ~1 hour retention. URL rewritten to /dl/."

# 6. zerox (0x0.st)
test_provider "zerox" "No" \
  "curl -s -S -A 'pushy/0.1' -F file=@${TMP_FILE} https://0x0.st" \
  "" \
  "Off by default. Upstream disabled uploads."

# 7. fileio
test_provider "fileio" "No" \
  "curl -s -S -F file=@${TMP_FILE} https://file.io" \
  ".link" \
  "Off by default. Anonymous uploads broken; requires API key."

# 8. transfersh
test_provider "transfersh" "No" \
  "curl -s -S --connect-timeout 5 --upload-file ${TMP_FILE} https://transfer.sh/t_probe.txt" \
  "" \
  "Off by default. Public host unreliable; requires custom base_url."

rm -f "$TMP_FILE"

cat <<EOF >> "$REPORT_FILE"

---

*This report is automatically updated by the [Provider Health Check workflow](.github/workflows/provider-health.yml).*
EOF

echo "Report generated at ${REPORT_FILE}"
